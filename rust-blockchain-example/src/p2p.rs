use super::{App, Block};
use libp2p::{
    floodsub::{Floodsub, FloodsubEvent, Topic},
    identity,
    mdns::{Mdns, MdnsEvent},
    swarm::{NetworkBehaviourEventProcess, Swarm},
    NetworkBehaviour, PeerId,
};
use log::{error, info};
use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use tokio::sync::mpsc;
use sha2::{Sha256, Digest};

// This was in the utils/crypto, but I couldn't get the import to resolve
extern crate ed25519_dalek;
type Sha256Hash = [u8; 32];

// Need lazy initialization to get around the fact that Rust won't let us initialize
// data in global variables with complex types. 
pub static KEYS: Lazy<identity::Keypair> = Lazy::new(identity::Keypair::generate_ed25519);
// Note: he peer ID is a cryptographic multihash of the node's public key
// If you don't tie the keys and peer ID together, the FloodSub protocol will 
// reject messages from peers. 
// https://docs.libp2p.io/concepts/peer-id/
pub static PEER_ID: Lazy<PeerId> = Lazy::new(|| PeerId::from(KEYS.public()));
// Helpful to separate out "topics" (channels) by different pieces of the protocol
pub static CHAIN_TOPIC: Lazy<Topic> = Lazy::new(|| Topic::new("chains"));
pub static BLOCK_TOPIC: Lazy<Topic> = Lazy::new(|| Topic::new("blocks"));
pub static VOTE_TOPIC: Lazy<Topic> = Lazy::new(|| Topic::new("votes"));

// An example of a struct that we can send (or receive) over the network
#[derive(Debug, Serialize, Deserialize)]
pub struct ChainResponse {
    pub blocks: Vec<Block>,
    pub receiver: String,
}

// Similar -- this was designed (in the example)
// for requesting a chain from a specific peer, identified by ID.
#[derive(Debug, Serialize, Deserialize)]
pub struct LocalChainRequest {
    pub from_peer_id: String,
}

// Internal events (things that aren't triggered by receiving something from the network)
pub enum EventType {
    LocalChainResponse(ChainResponse),
    Input(String),
    Init,
}


// --- ARDEN'S MESSAGE STRUCTURES ---
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Message {
    pub header: MessageHeader,
    pub payload: MessagePayload,
    // Need something easily serializable/deserializable
    // Signature -> [u8, 64] -> Vec (and back)
    pub signatures: Vec<Vec<u8>>, 
    pub kind: MessageKind,
}


#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MessageHeader {
    pub destination: String,
}

#[derive(Debug, PartialEq, Serialize, Deserialize, Clone)]
pub struct MessagePayload {
    pub parent_hash: Sha256Hash,
    pub epoch: u32,
    pub payload_string: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum MessageKind {
    Vote,
    Propose,
}

// --- END ARDEN'S MESSAGE STRUCTURES ---

// Core of the distributed behavior
#[derive(NetworkBehaviour)]
pub struct AppBehaviour {
    // Flooding protocol -- will trigger events (see below) 
    // when messages are received. Will also give us "channels"
    // to publish data to peers.
    pub floodsub: Floodsub,
    // A way of discovering peers that are running our protocol. 
    pub mdns: Mdns,
    // Do *not* derive network behavior trait for these -- 
    // just want them as members accessible via impl of this struct. 
    #[behaviour(ignore)]
    pub response_sender: mpsc::UnboundedSender<ChainResponse>,
    #[behaviour(ignore)]
    pub init_sender: mpsc::UnboundedSender<bool>,
    // This is where program should be implement
    #[behaviour(ignore)]
    pub app: App,
}

impl AppBehaviour {
    // Can define an initialization function if we want. 
    // (Can also just declare the struct in `main`, but having a `new` 
    // method is cleaner imo.)
    pub async fn new(
        app: App,
        response_sender: mpsc::UnboundedSender<ChainResponse>,
        init_sender: mpsc::UnboundedSender<bool>,
    ) -> Self {
        let mut behaviour = Self {
            app,
            floodsub: Floodsub::new(*PEER_ID),
            mdns: Mdns::new(Default::default())
                .await
                .expect("can create mdns"),
            response_sender,
            init_sender,
        };
        behaviour.floodsub.subscribe(CHAIN_TOPIC.clone());
        behaviour.floodsub.subscribe(BLOCK_TOPIC.clone());
        behaviour.floodsub.subscribe(VOTE_TOPIC.clone());
        behaviour
    }
}

// Incoming event handler. 
// Triggered when a "FloodsubEvent" happens -- i.e., when a message
// is received on a channel our floodsub instance is subscribed to 
impl NetworkBehaviourEventProcess<FloodsubEvent> for AppBehaviour {
    fn inject_event(&mut self, event: FloodsubEvent) {
        if let FloodsubEvent::Message(msg) = event {
            // We can then match on different types of messages
            if let Ok(resp) = serde_json::from_slice::<ChainResponse>(&msg.data) {
                if resp.receiver == PEER_ID.to_string() {
                    info!("Response from {}:", msg.source);
                    resp.blocks.iter().for_each(|r| info!("{:?}", r));
                    // ...and call into our local application logic
                    self.app.blocks = self.app.choose_chain(self.app.blocks.clone(), resp.blocks);
                }
            } else if let Ok(resp) = serde_json::from_slice::<LocalChainRequest>(&msg.data) {
                info!("sending local chain to {}", msg.source.to_string());
                let peer_id = resp.from_peer_id;
                if PEER_ID.to_string() == peer_id {
                    // ...or directly send data to a different async task
                    // (Sending data through this channel triggers an event defined in `main`.)
                    if let Err(e) = self.response_sender.send(ChainResponse {
                        blocks: self.app.blocks.clone(),
                        receiver: msg.source.to_string(),
                    }) {
                        error!("error sending response via channel, {}", e);
                    }
                }
            } else if let Ok(block) = serde_json::from_slice::<Block>(&msg.data) {
                info!("received new block from {}", msg.source.to_string());
                self.app.try_add_block(block);
            } else if let Ok(resp) = serde_json::from_slice::<Message>(&msg.data) {
                info!("received message from {}", msg.source.to_string());
                info!("{}", resp.payload.payload_string);
            }
        }
    }
}

// MDNS (peer discovery) protocol
// This is pretty standard -- essentially the same in all examples that use it.
impl NetworkBehaviourEventProcess<MdnsEvent> for AppBehaviour {
    fn inject_event(&mut self, event: MdnsEvent) {
        match event {
            MdnsEvent::Discovered(discovered_list) => {
                for (peer, _addr) in discovered_list {
                    self.floodsub.add_node_to_partial_view(peer);
                }
            }
            MdnsEvent::Expired(expired_list) => {
                for (peer, _addr) in expired_list {
                    if !self.mdns.has_node(&peer) {
                        self.floodsub.remove_node_from_partial_view(&peer);
                    }
                }
            }
        }
    }
}

// Helpers for MDNS
pub fn get_list_peers(swarm: &Swarm<AppBehaviour>) -> Vec<String> {
    info!("Discovered Peers:");
    let nodes = swarm.behaviour().mdns.discovered_nodes();
    let mut unique_peers = HashSet::new();
    for peer in nodes {
        unique_peers.insert(peer);
    }
    unique_peers.iter().map(|p| p.to_string()).collect()
}

pub fn handle_print_peers(swarm: &Swarm<AppBehaviour>) {
    let peers = get_list_peers(swarm);
    peers.iter().for_each(|p| info!("{}", p));
}

pub fn handle_print_chain(swarm: &Swarm<AppBehaviour>) {
    info!("Local Blockchain:");
    let pretty_json =
        serde_json::to_string_pretty(&swarm.behaviour().app.blocks).expect("can jsonify blocks");
    info!("{}", pretty_json);
}

// Helper for block creation (user command)
pub fn handle_create_block(cmd: &str, swarm: &mut Swarm<AppBehaviour>) {
    if let Some(data) = cmd.strip_prefix("create b") {
        // Note: we can get our 
        let behaviour = swarm.behaviour_mut();

        // Note: incorporate legit hashes here

        let block = Block::new(data.to_string());
        let json = serde_json::to_string(&block).expect("can jsonify request");
        behaviour.app.blocks.push(block);
        info!("broadcasting new block");
        behaviour
            .floodsub
            .publish(BLOCK_TOPIC.clone(), json.as_bytes());
    }
}

pub fn handle_send_msg(cmd: &str, swarm: &mut Swarm<AppBehaviour>) {
    if let Some(data) = cmd.strip_prefix("send m") {
        let behaviour = swarm.behaviour_mut();

        let mut hasher = Sha256::new();
        hasher.update(data);
        let result = hasher.finalize();
        let bytes: Sha256Hash = result.as_slice().try_into().expect("slice with incorrect length");

        // Create a message
        let payload = MessagePayload {
            parent_hash: bytes,
            epoch: 0,
            payload_string: String::from(data),
        };
        let header = MessageHeader {
            destination: String::from("test destination"),
        };
        let kind = MessageKind::Vote;
        let message = Message {
            header: header.clone(),
            payload: payload.clone(),
            kind: kind,
            signatures: Vec::new(),
        };

        let json = serde_json::to_string(&message).expect("can jsonify request");
        
        info!("broadcasting message");
        behaviour
            .floodsub
            .publish(VOTE_TOPIC.clone(), json.as_bytes());
    }

}
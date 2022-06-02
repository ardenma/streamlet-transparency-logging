use ed25519_dalek::PublicKey;
use log::info;
use rand::Rng;
use serde::{Deserialize, Serialize};
use std::time::SystemTime;
use std::{collections::HashMap};

use super::NetworkStack;
use crate::messages::{Message, MessageKind, MessagePayload};

#[derive(Debug)]
pub struct Peers {
    pub node_name: String,
    pub node_id: u32,
    pub public_key: PublicKey,
    pub peer_list: HashMap<String, PublicKey>,
    num_expected: usize,
    // Solely for demoability - this is for labeling of a peer as having been guilty of certain compromised behavior
    pub compromise_type: CompromiseType,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct PeerAdvertisement {
    pub public_key: PublicKey,
    pub node_id: u32,
    pub node_name: String,
    pub timestamp: SystemTime,
    end_init: bool,
    known_peers: Vec<String>,
}

pub enum InitStatus {
    InProgress,
    Done,
    DoneStartTimer,
}

#[derive(Debug)]
pub enum CompromiseType {
    BadBlocks,
    NoPropose,
    MultiVote,
    NoVote,
    BadPublish,
    TargettedMessages,
    NonLeaderPropose,
    NoCompromise,
}
impl CompromiseType {
    pub fn is_non_leader_propose(&self) -> bool {
        match *self {
            CompromiseType::NonLeaderPropose => true,
            _ => false,
        }
    }
    pub fn is_bad_blocks(&self) -> bool {
        match *self {
            CompromiseType::BadBlocks => true,
            _ => false,
        }
    }
    pub fn is_no_propose(&self) -> bool {
        match *self {
            CompromiseType::NoPropose => true,
            _ => false,
        }
    }
    pub fn is_multi_vote(&self) -> bool {
        match *self {
            CompromiseType::MultiVote => true,
            _ => false,
        }
    }
    pub fn is_no_vote(&self) -> bool {
        match *self {
            CompromiseType::NoVote => true,
            _ => false,
        }
    }
}

impl Peers {
    /* Initializer:
        @param my_name: identifying "name" of this node
            (if empty string, name will be generated from random 32-bit number)
        @param public_key: public key belonging to the owning StreamletInstance
    */
    pub fn new(mut my_name: String, public_key: PublicKey, num_peers: usize) -> Self {
        if my_name == "" {
            let rand: u32 = rand::thread_rng().gen();
            my_name = format!("{}", rand).to_string();
        }
        info!("Initializing peer with name {}; expecting {} peers", my_name, num_peers);
        Self {
            node_name: my_name,
            node_id: 0,
            public_key: public_key,
            peer_list: HashMap::new(),
            compromise_type: CompromiseType::NoCompromise,
            num_expected: num_peers,
        }
    }

    /* Set the peer id with the result of the peer init process.
    @param new_node_id: node id chosen based off of peer init process */
    pub fn set_node_id(&mut self, new_node_id: u32) {
        self.node_id = new_node_id;
    }

    /* Should be triggered whenever a PeerAdvertisement is received from the network.
    Inserts the peer into the hashmap if not already present.
    Protocol will accept first public key received for a peer.
    Closes the initialization channel if all peers have been received.
    @param ad: PeerAdvertisement received from the network
    @param net_stack: network stack containing an initialization channel to send on. */
    pub fn recv_advertisement(
        &mut self,
        ad: &PeerAdvertisement,
        net_stack: &mut NetworkStack,
    ) -> InitStatus {
        if ad.end_init && self.is_done() {
            self.end_init(net_stack);
            return InitStatus::Done;
        }
        if self.is_done() || self.peer_list.contains_key(&ad.node_name) {
            return InitStatus::Done;
        }
        info!("{} adding peer: {}", self.node_name, ad.node_name);
        self.peer_list.insert(ad.node_name.clone(), ad.public_key);

        if !ad.known_peers.contains(&self.node_name) {
            self.advertise_self(net_stack);
        }

        if self.is_done() && net_stack.init_channel_open() {
            info!(
                "{} is done with peer discovery protocol; discovered {} peer(s)",
                self.node_name,
                self.peer_list.len()
            );
            return InitStatus::DoneStartTimer;
        }

        return InitStatus::InProgress;
    }

    /* If all expected advertisements have been received. */
    pub fn is_done(&self) -> bool {
        self.peer_list.len() >= self.num_expected
    }

    pub fn send_end_init(&mut self, net_stack: &mut NetworkStack) {
        let my_ad = PeerAdvertisement {
            end_init: true,
            node_name: String::new(),
            node_id: self.node_id,
            timestamp: SystemTime::now(),
            public_key: self.public_key,
            known_peers: Vec::new(),
        };
        let message = Message::new(
            MessagePayload::PeerAdvertisement(my_ad),
            MessageKind::PeerInit,
            self.node_id,
            self.node_name.clone(),
        );

        net_stack.send_init_channel(message.serialize());
        self.end_init(net_stack);
    }

    /*  Close the initialization channel.
    Should not need to be called externally -- will be closed after the "last"
    advertisement is received. */
    fn end_init(&mut self, net_stack: &mut NetworkStack) {
        if !net_stack.init_channel_open() {
            return;
        }
        info!("ending init protocol");
        info!("Final state: {:?}", self.peer_list);
        net_stack.close_init_channel();
    }

    /* Label this node as compromised (so that other peers can view it as compromised) */
    fn set_compromise(&mut self, compromise_type: CompromiseType) {
        self.compromise_type = compromise_type
    }

    /* May be useful, e.g., if you restart with one additional peer. */
    #[allow(dead_code)]
    pub fn num_peers_expected(&self) -> usize {
        self.num_expected
    }

    /* May be useful, e.g., if we discover a peer is malicious. */
    #[allow(dead_code)]
    pub fn permanently_delete_peer(&mut self, name: String) {
        // If value was in the map, expected count should go down
        if let Some(_) = self.peer_list.remove(&name) {
            self.num_expected -= 1;
        }
    }

    pub fn advertise_self(&mut self, net_stack: &mut NetworkStack) {
        let my_ad = PeerAdvertisement {
            end_init: false,
            node_name: self.node_name.clone(),
            node_id: self.node_id,
            timestamp: SystemTime::now(),
            public_key: self.public_key,
            known_peers: Vec::from_iter(self.peer_list.keys().cloned()),
        };

        let message = Message::new(
            MessagePayload::PeerAdvertisement(my_ad),
            MessageKind::PeerInit,
            self.node_id,
            self.node_name.clone(),
        );

        net_stack.send_init_channel(message.serialize());
    }
}

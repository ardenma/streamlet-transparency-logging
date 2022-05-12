use libp2p::{
    floodsub::{Floodsub, FloodsubEvent, Topic},
    identity,
    noise,
    futures::StreamExt,
    mdns::{Mdns, MdnsEvent},
    swarm::{NetworkBehaviourEventProcess, Swarm, SwarmBuilder},
    tcp::TokioTcpConfig,
    Transport,
    core::upgrade,
    mplex,
    NetworkBehaviour, 
    PeerId,
};
// use log::{error, info};
use tokio::sync::mpsc;
use log::error;
// use log::info;

pub struct NetworkStack {
    // Access to network functionality
    swarm: Swarm<AppBehaviour>,
    // For broadcasting messages
    topic: Topic,
    // Note: could save peer id, but not needed?
}

#[derive(NetworkBehaviour)]
struct AppBehaviour {
    // Flooding protocol -- will trigger events (see below) 
    // when messages are received. Will also give us "channels"
    // to publish data to peers.
    floodsub: Floodsub,
    // A way of discovering peers that are running our protocol. 
    mdns: Mdns,

    // How to send arbitrary network events to the application (core logic)
    #[behaviour(ignore)]
    app_sender: mpsc::UnboundedSender<Vec<u8>>,
}

impl NetworkBehaviourEventProcess<FloodsubEvent> for AppBehaviour {
    fn inject_event(&mut self, event: FloodsubEvent) {
        if let FloodsubEvent::Message(msg) = event { 
            // Forward raw bytes.
            let res =        self.app_sender.send(msg.data);      
            if let Err(e) =  res {
                error!("Error communicating with main application {}", e);
            }
            // Only other information available to us = peer ID of source
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

impl NetworkStack {

    pub async fn new(topic_name: &str, app_sender: mpsc::UnboundedSender<Vec<u8>>) ->Self {
        
        // Metadata
        let keys = identity::Keypair::generate_ed25519();
        let peer_id = PeerId::from(keys.public());
        let topic = Topic::new(topic_name);
        let auth_keys = noise::Keypair::<noise::X25519Spec>::new()
            .into_authentic(&keys)
            .expect("Can't create auth keys for p2p channel");

        // Initialize the network and transport
        let mut behaviour = AppBehaviour { 
            floodsub: Floodsub::new(peer_id),
            mdns: Mdns::new(Default::default()).await.expect("Can't set up peer discovery protocol"),
            app_sender: app_sender,
        };

        behaviour.floodsub.subscribe(topic.clone());        
        
        let transp = TokioTcpConfig::new()  
            .upgrade(upgrade::Version::V1)
            .authenticate(noise::NoiseConfig::xx(auth_keys).into_authenticated())
            .multiplex(mplex::MplexConfig::new())
            .boxed(); // Put it on the heap

        let mut swarm = SwarmBuilder::new(transp, behaviour, peer_id)
            .executor(Box::new(|fut| {
                tokio::spawn(fut);
            }))
            .build();

        swarm.listen_on("/ip4/0.0.0.0/tcp/0".parse().expect("Can't get a local socket"))
            .expect("Can't start swarm!");
        
        Self{ 
            swarm: swarm,
            topic: topic, 
        }
    }

    pub fn broadcast_message(&mut self, message: Vec<u8>) {
        self
            .swarm
            .behaviour_mut()
            .floodsub
            .publish(self.topic.clone(), message);
    }

    // Polling happens via stream
    pub async fn clear_unhandled_event(&mut self) {
        // Returns future that resolves when next item in stream returns
        // (Won't resolve to `none` if stream is empty)
        self.swarm.select_next_some().await;
    }

}



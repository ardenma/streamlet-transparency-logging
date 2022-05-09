use libp2p::{
    floodsub::{self, Floodsub, FloodsubEvent},
    futures::StreamExt,
    identity,
    noise,
    tcp::TokioTcpConfig,
    core::upgrade,
    mplex,
    mdns::{Mdns, MdnsEvent},
    swarm::{NetworkBehaviourEventProcess, Swarm, SwarmBuilder, SwarmEvent},
    NetworkBehaviour,
    PeerId,
    Transport,
};

use std::collections::HashSet;
use std::error::Error;

use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct TestMessage {
    pub data: String,
    pub receiver: String,
}

#[derive(NetworkBehaviour)]
#[behaviour(event_process = true)]
pub struct TestBehaviour {
    pub floodsub: Floodsub,
    pub mdns: Mdns,
}

impl NetworkBehaviourEventProcess<FloodsubEvent> for TestBehaviour {
    fn inject_event(&mut self, event: FloodsubEvent) {
        match event {
            FloodsubEvent::Message(msg) => {
                if let Ok(msg_text) = serde_json::from_slice::<TestMessage>(&msg.data) {
                    println!("Received message from {:?}: {:?}", msg.source, msg_text);
                }
             }
            _ => (),
        }
    }
}

impl NetworkBehaviourEventProcess<MdnsEvent> for TestBehaviour {
    fn inject_event(&mut self, event: MdnsEvent) {
        match event {
            MdnsEvent::Discovered(discovered_list) => {
                for (peer, _) in discovered_list {
                    self.floodsub.add_node_to_partial_view(peer);
                }
            }
            MdnsEvent::Expired(expired_list) => {
                for (peer, _) in expired_list {
                    if !self.mdns.has_node(&peer) {
                        self.floodsub.remove_node_from_partial_view(&peer);
                    }
                }
            }
        }
    }
}

// https://github.com/zupzup/rust-blockchain-example/blob/main/src/p2p.rs
pub fn get_peers(swarm: &Swarm<TestBehaviour>) -> Vec<String> {
    let nodes = swarm.behaviour().mdns.discovered_nodes();
    let mut unique_peers = HashSet::new(); 
    for peer in nodes {
        unique_peers.insert(peer);
    }
    unique_peers.iter().map(|p| p.to_string()).collect()
}

pub async fn network_main(channel_topic: String, name: String) -> Result<(), Box<dyn Error>> {
    // Set up local identity state
    let id_keys = identity::Keypair::generate_ed25519();
    let peer_id = PeerId::from(id_keys.public());
    println!("Local peer ID: {:?}", peer_id);

    // Create keypair for transport layer encryption 
    let noise_keys = noise::Keypair::<noise::X25519Spec>::new()
        .into_authentic(&id_keys)
        .expect("Initiating noise DH keypair failed");

    let transport = TokioTcpConfig::new()
        .nodelay(true)
        .upgrade(upgrade::Version::V1)
        .authenticate(noise::NoiseConfig::xx(noise_keys).into_authenticated())
        .multiplex(mplex::MplexConfig::new())
        .boxed();

    let floodsub_topic = floodsub::Topic::new(channel_topic);

    let mut swarm = {
        let mdns = Mdns::new(Default::default()).await?;
        let mut behaviour = TestBehaviour {
            floodsub: Floodsub::new(peer_id.clone()),
            mdns,
        };
        behaviour.floodsub.subscribe(floodsub_topic.clone());

        // Return
        SwarmBuilder::new(transport, behaviour, peer_id)
            .executor( Box::new( |fut| { tokio::spawn(fut); } ) )
            .build()
    };

    swarm.listen_on("/ip4/0.0.0.0/tcp/0".parse()?)?;


    loop {
        tokio::select! {
            event = swarm.select_next_some() => {
                if let SwarmEvent::NewListenAddr { address, .. } = event {
                    println!("{} is Listening on {:?}", name, address);

                    let peers = get_peers(&swarm);
                    if !peers.is_empty() {
                        println!("sending data");
                        let req = TestMessage {
                            data: format!("Test message from {}", name).to_string(),
                            receiver: peers.iter().last().expect("no peers").to_string(),
                        };
                        let json = serde_json::to_string(&req).expect("can't jsonify req");
                        swarm.behaviour_mut().floodsub.publish(floodsub_topic.clone(), json.as_bytes());
                    }
                } 
                else {
                    println!("Unhandled Swarm event: {:?}", event);
                }
            }
        }
    }

}


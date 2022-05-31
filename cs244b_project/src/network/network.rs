use libp2p::{
    core::{muxing, transport, upgrade},
    futures::StreamExt,
    gossipsub,
    gossipsub::{
        GossipsubEvent, GossipsubMessage, IdentTopic as Topic, MessageAuthenticity, MessageId,
        ValidationMode,
    },
    identity,
    mdns::{Mdns, MdnsEvent},
    mplex, noise,
    swarm::{NetworkBehaviourEventProcess, Swarm, SwarmBuilder},
    tcp::TokioTcpConfig,
    NetworkBehaviour, PeerId, Transport,
};
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
// use log::{error, info};
use log::error;
use log::info;
use std::time::Duration;
use tokio::sync::mpsc;

// Set this to be the max. amount of time we're likely to be running one instance. 
// Generally, due to a (likely) bug in libp2p's mDNS implementation, we can't 
// consistently and accurately maintain connections to peers unless we set the 
// timeout to be very large. 
static IDLE_MINS : u64 = 20;

pub struct NetworkStack {
    // Access to network functionality
    swarm: Swarm<AppBehaviour>,
    // For broadcasting messages
    topic: Topic,
    // Optional: if you want to have a time-bound,
    // initialization period; a topic to subscribe to &
    // unsubscribe from.
    init_topic: Topic,
    init_open: bool,
    // Note: could save peer id, but not needed?
}

#[derive(NetworkBehaviour)]
struct AppBehaviour {
    // Flooding protocol -- will trigger events (see below)
    // when messages are received. Will also give us "channels"
    // to publish data to peers.
    gossipsub: gossipsub::Gossipsub,
    // A way of discovering peers that are running our protocol.
    mdns: Mdns,

    // How to send arbitrary network events to the application (core logic)
    #[behaviour(ignore)]
    app_sender: mpsc::UnboundedSender<Vec<u8>>,
}

impl NetworkBehaviourEventProcess<GossipsubEvent> for AppBehaviour {
    fn inject_event(&mut self, event: GossipsubEvent) {
        if let GossipsubEvent::Message {
            message,
            propagation_source: _,
            message_id: _,
        } = event
        {
            let res = self.app_sender.send(message.data);
            info!("Network stack forwarded message to app");
            if let Err(e) = res {
                error!("Error communicating with main application {}", e);
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
                    self.gossipsub.add_explicit_peer(&peer);
                }
            }
            MdnsEvent::Expired(_expired_list) => {}
        }
    }
}

impl NetworkStack {
    pub async fn new(topic_name: &str, app_sender: mpsc::UnboundedSender<Vec<u8>>) -> Self {
        // Key and identification
        let keys = identity::Keypair::generate_ed25519();
        let peer_id = PeerId::from(keys.public());
        println!("Local peer id: {:?}", peer_id);
        // Topic to listen on
        let topic = Topic::new(topic_name);

        let transport = NetworkStack::create_transport(&keys).await;
        let gossipsub = NetworkStack::init_gossipsub(&topic, &keys);
        let mdns = Mdns::new(Default::default())
            .await
            .expect("Can't set up peer discovery protocol");

        // **** create the swarm ****
        let behaviour = AppBehaviour {
            gossipsub: gossipsub,
            mdns: mdns,
            app_sender: app_sender,
        };
        let mut swarm = SwarmBuilder::new(transport, behaviour, peer_id)
            .executor(Box::new(|fut| {
                tokio::spawn(fut);
            }))
            .build();

        swarm
            .listen_on("/ip4/0.0.0.0/tcp/0".parse().unwrap())
            .expect("Can't set up local socket.");

        let init_topic = Topic::new("init");

        Self {
            swarm: swarm,
            topic: topic,
            init_topic: init_topic,
            init_open: false,
        }
    }

    pub fn broadcast_message(&mut self, message: Vec<u8>) {
        let res = self
            .swarm
            .behaviour_mut()
            .gossipsub
            .publish(self.topic.clone(), message);
        if let Err(e) = res {
            panic!("Failed to send message over GossipSub protocol: {:?}", e);
        }
    }

    // Polling happens via stream
    pub async fn clear_unhandled_event(&mut self) {
        self.swarm.select_next_some().await;
    }

    pub fn add_topic(&mut self, topic: &str) {
        self.swarm
            .behaviour_mut()
            .gossipsub
            .subscribe(&Topic::new(topic))
            .expect("Can't open new topic channel");
    }

    pub fn broadcast_to_topic(&mut self, topic: &str, message: Vec<u8>) {
        let res = self
            .swarm
            .behaviour_mut()
            .gossipsub
            .publish(Topic::new(topic), message);

        if let Err(e) = res {
            panic!("Failed to broadcast to topic {} with error {:?}.", topic, e);
        }
    }

    // Methods for handling an optional "init" channel.
    // Optional, additional channel that can be opened and closed
    // Can be used for, e.g., a short-term bootstrapping period.
    pub fn open_init_channel(&mut self) {
        self.swarm
            .behaviour_mut()
            .gossipsub
            .subscribe(&self.init_topic)
            .expect("Can't open initialization channel");
        self.init_open = true;
    }

    pub fn close_init_channel(&mut self) {
        if !self.init_open {
            return;
        }
        self.swarm
            .behaviour_mut()
            .gossipsub
            .unsubscribe(&self.init_topic)
            .expect("Can't close initialization channel");
        self.init_open = false;
    }

    pub fn init_channel_open(&self) -> bool {
        self.init_open
    }

    pub fn send_init_channel(&mut self, message: Vec<u8>) {
        if !self.init_open {
            return;
        }
        let res = self
            .swarm
            .behaviour_mut()
            .gossipsub
            .publish(self.init_topic.clone(), message);

        if let Err(_e) = res {
            info!("Not enough peers to initialize yet.");
        }
    }

    // ---- HELPERS FOR SETUP ----

    async fn create_transport(
        keys: &identity::Keypair,
    ) -> transport::Boxed<(PeerId, muxing::StreamMuxerBox)> {
        // Needed for configuring encryption on the transport layer
        let auth_keys = noise::Keypair::<noise::X25519Spec>::new()
            .into_authentic(&keys)
            .expect("Can't create auth keys for p2p channel");

        // Create encrypted transport layer
        let transport = TokioTcpConfig::new()
            .nodelay(true)
            .upgrade(upgrade::Version::V1)
            .authenticate(noise::NoiseConfig::xx(auth_keys).into_authenticated())
            .multiplex(mplex::MplexConfig::new())
            .boxed();

        transport
    }

    fn init_gossipsub(topic: &Topic, keys: &identity::Keypair) -> gossipsub::Gossipsub {

        // Set up the gossipsub configuration
        let gossipsub_config = gossipsub::GossipsubConfigBuilder::default()
            .heartbeat_interval(Duration::from_secs(10))
            .validation_mode(ValidationMode::Strict)
            .idle_timeout(Duration::from_secs(60 * IDLE_MINS))
            .build()
            .expect("Can't set up GossipSub configuration");

        let mut gossipsub: gossipsub::Gossipsub =
            gossipsub::Gossipsub::new(MessageAuthenticity::Signed(keys.clone()), gossipsub_config)
                .expect("Can't set up Gossipsub protocol");

        // Set up the gossipsub configuration
        gossipsub
            .subscribe(&topic)
            .expect("Can't subscribe to topic!");

        gossipsub
    }
}

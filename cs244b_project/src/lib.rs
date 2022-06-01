mod app;
mod blockchain;
mod messages;
mod network;
mod utils;

use itertools::Itertools;
use rand::Rng;
use tokio::io::{AsyncWriteExt, AsyncReadExt};
use std::collections::{hash_map::DefaultHasher, HashMap, VecDeque};
use std::hash::Hasher;
use tokio::sync::{Mutex};
use std::sync::Arc;
use bincode::{deserialize, serialize};

use log::{debug, info, warn};
use std::time::Duration;
use tokio::{
    io::{stdin, AsyncBufReadExt, BufReader},
    select, spawn,
    sync::{mpsc, watch},
    time::sleep,
};
use std::net::SocketAddr;
use tokio::net::TcpListener;

pub use app::app_interface::*;
pub use blockchain::{Block, BlockchainManager, Chain, LocalChain, SignedBlock};
pub use messages::{Message, MessageKind, MessagePayload};
pub use network::peer_init;
pub use network::peer_init::PeerAdvertisement;
pub use network::NetworkStack;
pub use utils::crypto::*;

pub struct StreamletInstance {
    pub id: u32,
    pub name: String,
    expected_peer_count: usize,
    blockchain_manager: BlockchainManager,
    pending_transactions: VecDeque<Vec<u8>>,
    keypair: Keypair,
    public_keys: HashMap<String, PublicKey>,
    sorted_peer_names: Vec<String>,
}

enum EventType {
    UserInput(String),
    NetworkInput(Vec<u8>),
    EpochStart,
    TCPRequestBlock,
    TCPRequestChain,
}

const EPOCH_LENGTH_S: u64 = 5;
const EPOCH_DELAY_MS: u64 = 100;

// ==========================
// === Core Streamlet API ===
// ==========================

impl StreamletInstance {
    const STREAMLET_TOPIC: &'static str = "streamlet";

    /* Initializer:
    @param my_name: identifying "name" of this node
    @param expected_peer_count: expected number of StreamletInstances running */
    pub fn new(name: String, expected_peer_count: usize) -> Self {
        // Setup public/private key pair and id
        let mut csprng = OsRng {};
        let keypair: Keypair = Keypair::generate(&mut csprng);
        let pk: PublicKey = keypair.public.clone();
        // let id = rand::thread_rng().gen();  // TODO: set this up in peer init? currently chance for collision

        // Build the streamlet instance
        Self {
            id: 0,
            expected_peer_count: expected_peer_count,
            name: name.clone(),
            blockchain_manager: BlockchainManager::new(),
            pending_transactions: VecDeque::new(),
            keypair: keypair,
            public_keys: HashMap::from([(name.clone(), pk)]),
            sorted_peer_names: Vec::new(),
        }
    }

    /* Main straemlet event loop.
    1. Intializes networking stack + input channels (e.g. stdin)
    2. Performs peer discovery
    3. Runs the main event loop */
    pub async fn run(&mut self) {

        // Share the epoch data here
        let current_epoch_handle = Arc::new(Mutex::new(0));
        let voted_this_epoch_handle = Arc::new(Mutex::new(false));
        // Initialize
        // (1) message queue for the network to send us data
        // (2) message queue for us to receive data from the network
        let (net_sender, mut receiver) = mpsc::unbounded_channel();

        // Initialize the network stack
        let mut net_stack =
            network::NetworkStack::new(StreamletInstance::STREAMLET_TOPIC, net_sender).await;

        // Set up stdin
        let mut stdin = BufReader::new(stdin()).lines();
        
        // Set up TCP for processing application requests
        let addr = "127.0.0.1:0".parse::<SocketAddr>().expect("Couldn't get socket addr");
        let listener = TcpListener::bind(&addr).await.expect("Couldn't create TCP listener");
        let local_addr = listener.local_addr().expect("Couldn't get local addr");
        info!("Listening for inbound TCP connection at {}", local_addr);

        // Accept create channels for streamlet instance to send / receive messages from the TCP thread
        let (tcp_connect_trigger, mut tcp_connect_recv) = watch::channel("tcp_conenct");
        let (tcp_data_sender, mut tcp_data_receiver) = mpsc::unbounded_channel();

        // Spawn thread to listen for TCP requests
        tokio::spawn(async move {
            run_tcp_server(listener, tcp_data_receiver, tcp_connect_trigger).await;
        });

        // Set up what we need to initialize the peer discovery protocol
        let mut peers = peer_init::Peers::new(self.name.clone(), self.keypair.public, self.expected_peer_count);
        net_stack.open_init_channel();

        // Setup epoch timer channel
        let (timer_trigger, mut timer_recv) = watch::channel("timer_init");
        let (epoch_trigger, mut epoch_recv) = watch::channel("epoch_trigger");

        let mut current_epoch_handle_timer = current_epoch_handle.clone();
        // Epoch timer thread
        let voted_this_epoch_handle_copy = voted_this_epoch_handle.clone();
        tokio::spawn(async move {
            // Wait until signaled that peer discovery is done
            let _ = timer_recv.changed().await.is_ok();

            // Epoch timer loop
            loop {
                sleep(Duration::from_secs(EPOCH_LENGTH_S)).await;
                let mut current_epoch = current_epoch_handle_timer.lock().await;
                *current_epoch = *current_epoch + 1;
                drop(current_epoch);
                let mut voted_this_epoch = voted_this_epoch_handle_copy.lock().await;
                *voted_this_epoch = false;
                drop(voted_this_epoch);
                epoch_trigger.send("tick!").expect("Timer reciever closed?");
            }
        });

        let app_interface = AppInterface::new(&mut net_stack);

        // Main event loop!
        loop {
            let evt = {
                select! {
                    // User input
                    line = stdin.next_line() => {
                        let line_data = line.expect("Can't get line").expect("Can't read from stdin");
                        Some(EventType::UserInput(line_data))
                    },

                    // When the network receives *any* message, it forwards the data to us thru this channel
                    network_response = receiver.recv() => {
                        Some(EventType::NetworkInput(network_response.expect("Response doesn't exist.")))
                    },

                    // One way to model the timer tick
                    _tick = epoch_recv.changed() => {
                        Some(EventType::EpochStart)
                    }

                    // Needs to be polled in order to make progress.
                    _ = net_stack.clear_unhandled_event() => {
                        None
                    },
                    
                    // One way to model getting a TCP request
                    _ = tcp_connect_recv.changed() => {
                        let request_type = *tcp_connect_recv.borrow();
                        debug!("From TCP thread recieved request type {}", request_type);
                        match request_type {
                            "chain" => { Some(EventType::TCPRequestChain) }
                            "block" => { Some(EventType::TCPRequestBlock) }
                            _ => { None }
                        }
                    }

                }
            };

            if let Some(event) = evt {
                match event {
                    EventType::UserInput(line) => {
                        if line.starts_with("init") {
                            peers.advertise_self(&mut net_stack);
                        }
                        if line.starts_with("end discovery") || line.starts_with("e d") {
                            peers.send_end_init(&mut net_stack);
                        } else if line.starts_with("notarized chains") || line.starts_with("nc") {
                            self.blockchain_manager.print_notarized_chains();
                        } else if line.starts_with("finalized chain") || line.starts_with("fc") {
                            self.blockchain_manager.print_finalized_chains();
                        }
                    }
                    EventType::TCPRequestChain => {
                        let finalized_chain = self.blockchain_manager.export_local_chain();
                        debug!("Sending chain {} to TCP thread", finalized_chain);
                        tcp_data_sender.send(serialize(&finalized_chain).expect("Failed to serialize chain")).expect("Failed to send chain...");
                    }
                    EventType::TCPRequestBlock => {
                        let (latest_finalized_block, signatures) = self.get_latest_finalized_block();
                        let signed_block = SignedBlock {
                            block: latest_finalized_block,
                            signatures: signatures,
                        };
                        debug!("Sending block {:?} to TCP thread", signed_block);
                        tcp_data_sender.send(serialize(&signed_block).expect("Failed to serialize block")).expect("Failed to send block..");
                    }
                    EventType::EpochStart => {

                        let current_epoch_ref = current_epoch_handle.lock().await;
                        let epoch = *current_epoch_ref;
                        drop(current_epoch_ref);

                        let leader = self.get_epoch_leader(epoch);
                        
                        info!("Epoch: {} starting with leader {}...", epoch, leader);

                        // If I am the current leader, propose a block
                        if leader == &self.name {
                            info!("I'm the leader");
                            if let Some(data) = self.pending_transactions.pop_front() {
                                sleep(Duration::from_millis(EPOCH_DELAY_MS)).await;
                                // Cretae message contents
                                let height = u64::try_from(
                                    self.blockchain_manager.longest_notarized_chain_length,
                                )
                                .unwrap()
                                    - 1;
                                let (parent, _) = self.blockchain_manager.head();
                                let parent_hash = parent.hash.clone();
                                let proposed_block = Block::new(
                                    epoch,
                                    parent_hash,
                                    data,
                                    height,
                                    rand::thread_rng().gen(),
                                );

                                // Construct message
                                let mut message = Message::new(
                                    MessagePayload::Block(proposed_block),
                                    MessageKind::Propose,
                                    self.id,
                                    self.name.clone(),
                                );

                                // Sign and send mesasage
                                if self.sign_message(&mut message) {
                                    info!("Epoch: {}, (Propose) SENDING proposal, broadcasting message {}...", epoch, message.nonce);
                                    net_stack.broadcast_message(message.serialize());
                                } else {
                                    debug!("something weird happened...")
                                }
                            }
                        }
                    }
                    EventType::NetworkInput(bytes) => {
                        // Received message
                        let message = Message::deserialize(&bytes);
                        
                        // Lock mutexes short-term. 
                        // Locking for too long causes epoch timers to get out of sync. 
                        let current_epoch_ref = current_epoch_handle.lock().await;
                        let epoch = *current_epoch_ref;
                        drop(current_epoch_ref);
                        let voted_this_epoch_ref = voted_this_epoch_handle.lock().await;
                        let voted_this_epoch = *voted_this_epoch_ref;
                        drop(voted_this_epoch_ref);

                        info!("Epoch: {}, Received {:?} message...", epoch, &message.kind);
                    
                        // Message processing logic
                        match &message.kind {
                            // Data from application
                            MessageKind::AppSend => {
                                match &message.payload {
                                    MessagePayload::AppData(data) => {
                                        self.pending_transactions.push_back(data.clone());
                                    }
                                    _ => {
                                        debug!("Unkown payload for MessageKind::AppSend");
                                    }
                                };
                            },
                            // Fulfill application request for data (ask the app to create a TCP connection for transport)
                            MessageKind::AppBlockRequest => {
                                // Construct message
                                let new_message = Message::new_with_defined_tag(
                                    MessagePayload::SocketAddr(local_addr),
                                    MessageKind::AppBlockResponse,
                                    message.tag,
                                    self.id,
                                    self.name.clone(),
                                );

                                // TOOD: just send to the application instead of bcast?
                                info!("Epoch: {}, responding to AppBlockRequest with {:?}", epoch, &new_message);
                                app_interface.send_to_app(&mut net_stack, new_message.serialize());
                            },
                            // Fulfill application request for chain (ask the app to create a TCP connection for transport)
                            MessageKind::AppChainRequest => {
                                // Construct message
                                let new_message = Message::new_with_defined_tag(
                                    MessagePayload::SocketAddr(local_addr),
                                    MessageKind::AppChainResponse,
                                    message.tag,
                                    self.id,
                                    self.name.clone(),
                                );

                                info!("Epoch: {}, responding to AppChainRequest with {:?}", epoch, &new_message);
                                net_stack.broadcast_to_topic("app", new_message.serialize());
                            },
                            // Message only for application (we just ignore)
                            MessageKind::AppBlockResponse => { /* Do nothing? */ },
                            MessageKind::AppChainResponse => { /* Do nothing? */ },
                            // Peer advertisement logic
                            MessageKind::PeerInit => {
                                if let MessagePayload::PeerAdvertisement(ad) = &message.payload {
                                    self.add_public_key(ad.node_name.clone(), &ad.public_key);
                                    let status = peers.recv_advertisement(&ad, &mut net_stack);

                                    // Initialize vector of peers (for leader election)
                                    self.sorted_peer_names =
                                        self.public_keys.keys().cloned().sorted().collect();
                                    
                                    // Sometimes, "default" keys (empty string) end up in the map, 
                                    // generally because of how it's initialized. Remove these here. 
                                    self.sorted_peer_names.retain(|x| *x != String::new());

                                    // If we complete the peer discovery protocol, start timer
                                    // so that they start at roughly the same time on all nodes...
                                    // TODO do a better job of syncing timers...
                                    match status {
                                        peer_init::InitStatus::DoneStartTimer => {
                                            let _ = timer_trigger.send("start!").is_ok();
                                        }
                                        _ => { /* Do nothing */ }
                                    }
                                } else {
                                    debug!("Unkown payload for MessageKind::PeerInit");
                                }
                            },
                            // Implicit echo logic
                            // TODO: decide whether we think something's legit
                            MessageKind::Vote => {
                                // Currently signing + echoing everything...
                                // Should probably only sign things we already voted on??
                                // Note sure... TODO
                                // Also when do we stop echoing??? TODO
                                // TODO make sure we only add a notarized block once lol
                                if let MessagePayload::Block(block) = &message.payload {
                                    if self.should_vote(&message, voted_this_epoch, epoch, &block, &app_interface) {
                                        // Clone of message that we can modify
                                        let mut new_message = message.clone();
                                        let signed = self.sign_message(&mut new_message);
                                        // Broadcast messages that we haven't signed yet
                                        // Note: this is an inexact, but reasonable, proxy for echoing
                                        if signed {
                                            info!("Epoch {}: VOTED and signed message {}; broadcasting", epoch, message.nonce);
                                            net_stack.broadcast_message(new_message.serialize());
                                            // Once we've voted for a transaction, we should never propose it. 
                                            self.pending_transactions.retain(|x| *x != block.data);
                                        }
                                    }
                                    // If block is notarized and still extends from a longest notarized chain, 
                                    // then add it to the chain. 
                                    if self.is_notarized(&message) {
                                        let index = self.blockchain_manager.index_of_ancestor_chain(block.clone());
                                        if index.is_some() {
                                            info!("Epoch {}: Adding notarized message {} to chain {}", epoch, message.nonce, index.unwrap());
                                            self.blockchain_manager.add_to_chain( 
                                                block.clone(), 
                                                message.clone().get_signatures(), 
                                                index.unwrap()
                                            );
                                        }
                                    }
                                } else {
                                    debug!("Unkown payload for MessageKind::Vote");
                                }
                            },
                            // Follower proposal handling logic
                            MessageKind::Propose => {
                                // If we haven't voted  yet this epoch and
                                // we receive a message from the leader, sign and vote
                                if let MessagePayload::Block(block) = &message.payload {
                                    if self.should_vote(&message, voted_this_epoch, epoch, &block, &app_interface) {
                                        // Clone of message that we can modify
                                        let mut new_message = message.clone();
                                        // Sign and broadcast
                                        info!("Epoch: {}, (Propose) received PROPOSE, signing and broadcasting message {}...",epoch, message.nonce);
                                        new_message.kind = MessageKind::Vote;
                                        self.sign_message(&mut new_message);
                                        net_stack.broadcast_message(new_message.serialize());
                                        // If an epoch has passed since we locked the mutex, then we may miss an epoch of voting.
                                        // This is assumed to be rare, and nodes will recover in the next epoch. 
                                        let mut voted_this_epoch_ref = voted_this_epoch_handle.lock().await;
                                        *voted_this_epoch_ref = true;
                                        drop(voted_this_epoch_ref);

                                        // Add the received (+ signed by us) message to the chain if its notarized
                                        if self.is_notarized(&message) {
                                            info!("Epoch: {}, (Propose) received PROPOSE, message {} is NOTARIZED, adding to chain...",epoch, message.nonce);
                                            self.blockchain_manager.index_of_ancestor_chain(block.clone()).map(|idx| 
                                                self.blockchain_manager
                                                    .add_to_chain(block.clone(), message.clone().get_signatures(), idx)
                                            );
                                        }

                                        self.pending_transactions.retain(|x| *x != block.data);
                                    }
                                } else {
                                    debug!("Unkown payload for MessageKind::Propose");
                                }
                            },
                            _ => {
                                debug!("Unknown message format/kind - ignoring");
                            },
                        };
                    }
                }
            }
        }
    }

    /* Returns a copy of the instance's public key */
    pub fn get_public_key(&self) -> PublicKey {
        return self.keypair.public;
    }

    /* Returns a copy of the most recently finalized block and its signatures */
    pub fn get_latest_finalized_block(&self) -> (Block, Vec<Signature>) {
        let (block, signatures) = self.blockchain_manager.get_latest_finalized_block();
        (block.clone(), signatures.clone())
    }
}

// =========================
// === Streamlet Helpers ===
// =========================

impl StreamletInstance {
    /* Signs an arbitrary slice of bytes
    @param bytes: arbitrary bytes to sign
    Note: should get rid of this? mainly for testing */
    fn sign(&self, bytes: &[u8]) -> Signature {
        return self.keypair.sign(bytes);
    }

    /* Signs a message's payload and adds the signature to the message
    after verifying it has not already signed it (currently inefficient)
    Returns true if we successfully sign, false if it's already been signed
    by us
     @param message: the message instance with a payload to be signed */
    fn sign_message(&self, message: &mut Message) -> bool {
        // Create signature
        let signature: Signature = self.keypair.sign(message.serialize_payload().as_slice());
        // Make sure we haven't signed already
        for s in message.clone().get_signatures() {
            if signature == s { return false; }
        }

        message.sign_message(signature.clone());
        return true;
    }

    /* Verifies a (message, signature) pair against a public key.
    @param message: the message instance with a (signature, payload) pair to be validated
    @param signature: signature of the message to be validated
    @param pk: public key to verify against the signature */
    fn verify_signature(&self, message: &Message, signature: &Signature, pk: &PublicKey) -> bool {
        let result = pk.verify(message.serialize_payload().as_slice(), signature);
        if let Err(error) = result {
            return false;
        } else {
            return true;
        }
    }

    /* Verifies message signatures against all known public keys, returning the number of valid signatures
    @param message: the message instance with signatures to be validated */
    fn verify_message(&self, message: &Message) -> usize {
        let mut num_valid_signatures = 0;
        let signatures = message.clone().get_signatures(); // Check all signatures

        // Check all sigatures on the message (TODO check duplicates)
        for signature in signatures.iter() {
            // Check against all known pk's
            for pk in self.public_keys.values() {
                if self.verify_signature(message, signature, pk) {
                    num_valid_signatures += 1;
                    break;
                }
            }
        }
        debug!(
            "Attempted validation on message {}, found {} valid signatures",
            message.nonce, num_valid_signatures
        );
        return num_valid_signatures;
    }

    /* Determines if the block associated with a message is notarized.
    @param epoch: epoch number */
    pub fn is_notarized(&self, message: &Message) -> bool {
        // Partially synchronous model: 
        // - Finalize after three blocks
        // - >= 2N/3 signatures for notarization 
        // Note: expected peer count = excluding self; add one to get N
        return self.verify_message(message)
            >= (2.0 * (self.expected_peer_count + 1) as f64 / 3.0).ceil() as usize;
    }

    fn should_vote(&mut self, message: &Message, voted_this_epoch: bool, epoch: u64, block: &Block, app_interface: &AppInterface) -> bool {
        return !voted_this_epoch && 
            self.check_from_leader(epoch, &message) &&
            block.epoch == epoch &&
            self.blockchain_manager.index_of_ancestor_chain(block.clone()).is_some() &&
            app_interface.data_is_valid(&message);
    }

    /* Determines if the block associated with a message is notarized.
    @param epoch: epoch number */
    fn check_from_leader(&self, epoch: u64, message: &Message) -> bool {
        // If message is from leader, only their signature should be on it
        let leader = self.get_epoch_leader(epoch);

        // Make sure we have leader's public key
        if !self.public_keys.contains_key(leader) {
            warn!("Missing leader public key...");
            return false;
        }
        let leader_pk = self.public_keys[leader];

        // Check leader's signature
        let signatures = message.clone().get_signatures();
        if signatures.len() == 1 {
            return self.verify_signature(message, &signatures[0], &leader_pk);
        } else {
            return false;
        }
    }

    /* Determines epoch leader using deterministic hash function. */
    fn get_epoch_leader(&self, epoch: u64) -> &String {
        let mut hasher = DefaultHasher::new();
        hasher.write_u64(epoch);
        let result = hasher.finish() as usize;
        let leader_index = result % (self.expected_peer_count + 1); // +1 for self
        return &self.sorted_peer_names[leader_index];
    }
    /* Determines epoch leader using deterministic hash function.
    @param epoch: epoch number
    Note: for testing, should be taken care of in peer discovery. */
    pub fn add_public_key(&mut self, instance_name: String, pk: &PublicKey) {
        self.public_keys.insert(instance_name, pk.clone()); // TODO catch errors with insertion?
    }
}

// ============================
// === Streamlet Unit Tests ===
// ============================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_streamlet_signatures() {
        let streamlet = StreamletInstance::new(String::from("Test"), 1);
        // Testing signatures
        let message: &[u8] = b"This is a test of the tsunami alert system.";
        let signature: Signature = streamlet.sign(message);
        let public_key: PublicKey = streamlet.get_public_key();
        assert!(public_key.verify(message, &signature).is_ok());
    }

    #[test]
    fn test_streamlet_msg_signatures() {
        let mut streamlet1 = StreamletInstance::new(String::from("Test1"), 3);
        let streamlet2 = StreamletInstance::new(String::from("Test2"), 3);
        let streamlet3 = StreamletInstance::new(String::from("Test3"), 3);

        // Create random hash
        let mut hasher = Sha256::new();
        hasher.update(b"hello world");
        let result = hasher.finalize();
        let bytes: Sha256Hash = result
            .as_slice()
            .try_into()
            .expect("slice with incorrect length");

        // Create a test block
        let blk = Block::new(0, bytes, String::from("test").into_bytes(), 0, 0);

        // Create a message
        let mut message = Message::new_with_defined_nonce(
            MessagePayload::Block(blk),
            MessageKind::Vote,
            0,
            0,
            String::from("test"),
        );

        // Signing message
        streamlet1.sign_message(&mut message);
        assert!(message.signature_count() == 1);
        streamlet2.sign_message(&mut message);
        assert!(message.signature_count() == 2);
        streamlet3.sign_message(&mut message);
        assert!(message.signature_count() == 3);

        // Adding public keys to streamlet1
        streamlet1.add_public_key(String::from("test2"), &streamlet2.get_public_key());
        let bad_result = streamlet1.verify_message(&message);
        assert!(bad_result == 2);
        streamlet1.add_public_key(String::from("test3"), &streamlet3.get_public_key());
        assert!(streamlet1.public_keys.len() == 3);

        // Verify message with all signatures
        let good_result = streamlet1.verify_message(&message);
        assert!(good_result == 3);
    }
}

async fn run_tcp_server(listener: TcpListener, 
                mut tcp_data_receiver: mpsc::UnboundedReceiver<Vec<u8>>, 
                tcp_connect_trigger: tokio::sync::watch::Sender<&str>) 
{
    loop {
        let (mut stream, _) = listener.accept().await.expect("Failed to accept TCP connection");

        // Determine Request Type
        let mut msg_bytes = Vec::new();
        stream.read_to_end(&mut msg_bytes).await.expect("Did not recieive data");
        let msg = String::from_utf8(msg_bytes.clone()).expect("Unable to decode msg bytes");
        
        // Ask streamlet for data
        match msg.as_str() {
            "chain" => {
                debug!("(TCP Thread) asking streamlet for chain");
                tcp_connect_trigger.send("chain").expect("TCP connect trigger closed?");
            }
            "block" => { 
                debug!("(TCP Thread) asking streamlet for block");
                tcp_connect_trigger.send("block").expect("TCP connect trigger closed?"); 
            }
            _ => { 
                info!("Unknown TCP request type"); 
            }
        }
        let data: Vec<u8> = tcp_data_receiver.recv().await.expect("Expected to receive data from streamlet...");
        
        // Send through TCP stream
        stream.write_all(&data).await.expect("Failed writing data to TCP stream...");
        stream.flush().await.expect("Failed to flush stream...");
        stream.shutdown().await.expect("Failed to close stream...");
    }
}

// ***** APPLICATION *****
pub async fn run_app() {
    let mut app = app::Application::new();
    app.run().await;
}


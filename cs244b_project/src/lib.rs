mod blockchain;
mod messages;
mod network;
mod utils;

use rand::Rng;
use std::collections::{hash_map::DefaultHasher, HashMap, VecDeque};
use itertools::Itertools;
use std::hash::Hasher;

use std::time::Duration;
use tokio::{
    io::{stdin, AsyncBufReadExt, BufReader},
    select, spawn,
    sync::{mpsc, watch},
    time::sleep,
};
use log::{info, warn, debug};

pub use blockchain::{Block, Chain, LocalChain, BlockchainManager};
pub use messages::{Message, MessageKind, MessagePayload};
pub use network::peer_init;
pub use network::NetworkStack;
pub use utils::crypto::*;

pub struct StreamletInstance {
    pub id: u32,
    pub name: String,
    expected_peer_count: usize,
    current_epoch: u64,
    voted_this_epoch: bool,
    blockchain_manager: BlockchainManager,
    pending_transactions: VecDeque<String>,
    keypair: Keypair,
    public_keys: HashMap<String, PublicKey>,
    sorted_peer_names: Vec<String>,
}
enum EventType {
    UserInput(String),
    NetworkInput(Vec<u8>),
    DoInit,
    EpochStart,
}

const EPOCH_LENGTH_S: u64 = 1;

// ==========================
// === Core Streamlet API ===
// ==========================

impl StreamletInstance {
    /* Initializer:
        @param my_name: identifying "name" of this node
        @param expected_peer_count: expected number of StreamletInstances running */
    pub fn new(name: String, expected_peer_count: usize, ) -> Self {
        // Setup public/private key pair and id
        let mut csprng = OsRng {};
        let keypair: Keypair = Keypair::generate(&mut csprng);
        let pk: PublicKey = keypair.public.clone();
        // let id = rand::thread_rng().gen();  // TODO: set this up in peer init? currently chance for collision

        // Build the streamlet instance
        Self {
            id: 0,
            expected_peer_count: expected_peer_count,
            current_epoch: 0,
            voted_this_epoch: false,
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
        // Initialize
        // (1) message queue for the network to send us data
        // (2) message queue for us to receive data from the network
        let (net_sender, mut receiver) = mpsc::unbounded_channel();

        // Initialize the network stack
        let mut net_stack = network::NetworkStack::new("test_messages", net_sender).await;

        // Set up stdin
        let mut stdin = BufReader::new(stdin()).lines();

        // Set up what we need to initialize the peer discovery protocol
        let mut peers = peer_init::Peers::new(self.name.clone(),  self.keypair.public);
        let (trigger_init, mut recv_init) = mpsc::channel(1);
        let mut needs_init = true;
        spawn(async move {
            // trigger after MDNS has had a chance to do its thing
            sleep(Duration::from_secs(1)).await;
            trigger_init.send(0).await.expect("can't send init event");
        });

        // Setup epoch timer channel
        let (timer_trigger, mut timer_recv) = watch::channel("timer_init");
        let (epoch_trigger, mut epoch_recv) = watch::channel("epoch_trigger");

        tokio::spawn(async move {
            // Wait until signaled that peer discovery is done
            let _ = timer_recv.changed().await.is_ok();

            // Epoch timer loop
            loop {
                sleep(Duration::from_secs(EPOCH_LENGTH_S)).await;
                epoch_trigger.send("tick!").expect("Timer reciever closed?");
            };
        });

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

                    // One way to model the initialization event
                    _init = recv_init.recv() => {
                        recv_init.close();
                        if needs_init {
                            needs_init = false;
                            Some(EventType::DoInit)
                        } else {
                            None
                        }
                    },

                    // One way to model the timer tick
                    _tick = epoch_recv.changed() => {
                        Some(EventType::EpochStart)
                    }

                    // Needs to be polled in order to make progress.
                    _ = net_stack.clear_unhandled_event() => {
                        None
                    },

                }
            };

            if let Some(event) = evt {
                match event {
                    EventType::UserInput(line) => {
                        if line.starts_with("end discovery") || line.starts_with("e d") {
                            peers.send_end_init(&mut net_stack);
                        } else {
                            info!("User input... adding '{}' to pending transactions", line);
                            self.pending_transactions.push_back(line);
                        }
                    }
                    EventType::EpochStart => {
                        self.voted_this_epoch = false;
                        let leader = self.get_epoch_leader();
                        // debug!("Epoch: {} starting with leader {}...", self.current_epoch, leader);
                 
                        // If I am the current leader, propose a block
                        if leader == &self.name {
                            if let Some(data) = self.pending_transactions.pop_front() {
                                // Send message
                                let height = u64::try_from(self.blockchain_manager.longest_notarized_chain_length).unwrap() - 1;
                                let parent_hash = self.blockchain_manager.head().hash.clone();
                                let proposed_block = Block::new(
                                    self.current_epoch, 
                                    parent_hash,
                                    data,
                                    height, 
                                    rand::thread_rng().gen());
                                
                                // Construct message
                                let mut message = Message {
                                    payload: MessagePayload::Block(proposed_block),
                                    kind: MessageKind::Propose,
                                    nonce: rand::thread_rng().gen(),
                                    signatures: Some(Vec::new()),
                                    sender_id: self.id,
                                    sender_name: self.name.clone(),
                                };

                                // Sign and send mesasage
                                if self.sign_message(&mut message) {
                                    info!("Epoch: {}, (Propose) SENDING proposal, broadcasting message {}...", self.current_epoch, message.nonce);
                                    net_stack.broadcast_message(message.serialize());
                                } else {
                                    panic!("something weird happened...")
                                }
                            }
                        }

                        self.current_epoch += 1;
                    }
                    EventType::NetworkInput(bytes) => {
                        // Received message
                        let message = Message::deserialize(&bytes);

                        // Clone of message that we can modify
                        let mut new_message = message.clone();

                        // Message processing logic
                        match &message.payload {
                            // Peer advertisement logic
                            MessagePayload::PeerAdvertisement(ad) => {
                                self.add_public_key(ad.node_name.clone(), &ad.public_key);
                                let status = peers.recv_advertisement(&ad, &mut net_stack);
                                
                                // Initialize vector of peers (for leader election)
                                self.sorted_peer_names = self.public_keys.keys().cloned().sorted().collect();

                                // If we complete the peer discovery protocol, start timer
                                // so that they start at roughly the same time on all nodes...
                                // TODO do a better job of syncing timers...
                                match status {
                                    peer_init::InitStatus::DoneStartTimer => {
                                        let _ = timer_trigger.send("start!").is_ok();
                                    }
                                    _ => {/* Do nothing */}
                                }
                            }

                            // Main Streamlet message logic
                            MessagePayload::Block(block) => {
                                match &message.kind {
                                    MessageKind::Vote => {
                                        // Currently signing + echoing everything...
                                        // Should probably only sign things we already voted on??
                                        // Note sure... TODO
                                        // Also when do we stop echoing??? TODO

                                        // Only broadcast if we haven't seen it already?
                                        if self.sign_message(&mut new_message) {
                                            info!("Epoch: {}, (Vote) received VOTE, signing and broadcasting message {}...", self.current_epoch, new_message.nonce);
                                            net_stack.broadcast_message(new_message.serialize());
                                        }

                                        // Add the received (+ signed by us) message to the chain if its notarized
                                        if self.is_notarized(&new_message) {
                                            info!("Epoch: {}, (Vote) received VOTE, message {} is NOTARIZED, adding to chain...", self.current_epoch, message.nonce);
                                            self.blockchain_manager.add_to_chain(block.clone());
                                        }
                                    },
                                    MessageKind::Propose => {
                                        // If we haven't voted  yet this epoch and
                                        // we receive a message from the leader, sign and vote

                                        if !self.voted_this_epoch && self.check_from_leader(&message) {
                                            info!("Epoch: {}, (Propose) received PROPOSE, signing and broadcasting message {}...",self.current_epoch, message.nonce);
                                            self.sign_message(&mut new_message);
                                            net_stack.broadcast_message(new_message.serialize());
                                            self.voted_this_epoch = true;
                                            
                                            // Add the received (+ signed by us) message to the chain if its notarized
                                            if self.is_notarized(&new_message) {
                                                info!("Epoch: {}, (Propose) received PROPOSE, message {} is NOTARIZED, adding to chain...", self.current_epoch, message.nonce);
                                                self.blockchain_manager.add_to_chain(block.clone());
                                            }

                                        }


                                    },
                                    _ => { 
                                        debug!("Unknown message format - ignoring");
                                    }
                                }
                            }
                            _ => {}
                        };

                    }
                    EventType::DoInit => {
                        peers.start_init(&mut net_stack, self.expected_peer_count);
                    }
                }
            }
        }
    }

    pub fn get_public_key(&self) -> PublicKey {
        return self.keypair.public;
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
        match &mut message.signatures {
            Some(_) => {
                // Create signature
                let signature: Signature =
                    self.keypair.sign(message.serialize_payload().as_slice());
                let signatures: &mut Vec<Signature> = message.signatures.as_mut().unwrap();

                // Make sure we haven't signed already
                for s in signatures.iter() {
                    if signature == *s {
                        return false;
                    }
                }

                signatures.push(signature);
                return true;
            }
            None => panic!("Tried to add signature to message without signature vector!"),
        }
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
        let signatures = message.signatures.as_ref().unwrap(); // Check all signatures
        
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
        debug!("Attempted validation on message {}, found {} valid signatures", message.nonce, num_valid_signatures);
        return num_valid_signatures;
    }

    /* Determines if the block associated with a message is notarized.
        @param epoch: epoch number */
    pub fn is_notarized(&self, message: &Message) -> bool {
        return self.verify_message(message) >= ((self.expected_peer_count + 1) as f64 / 2.0).ceil() as usize;
    }

    /* Determines if the block associated with a message is notarized.
        @param epoch: epoch number */
    fn check_from_leader(&self, message: &Message) -> bool {
        // If message is from leader, only their signature should be on it
        let leader = self.get_epoch_leader();

        // Make sure we have leader's public key
        if !self.public_keys.contains_key(leader) {
            warn!("Missing leader public key...");
            return false;
        } 
        let leader_pk = self.public_keys[leader];

        // Check leader's signature
        match &message.signatures {
            Some(signatures) => {
                if signatures.len() == 1 {
                    return self.verify_signature(message, &signatures[0], &leader_pk);
                } else {
                    return false;
                }
            },
            None => {
                return false;
            }
        }

    }    

    /* Determines epoch leader using deterministic hash function. */
    fn get_epoch_leader(&self) -> &String {
        let mut hasher = DefaultHasher::new();
        hasher.write_u64(self.current_epoch);
        let result = hasher.finish() as usize;
        let leader_index = result % (self.expected_peer_count + 1);  // +1 for self
        return &self.sorted_peer_names[leader_index];
    }
    /* Determines epoch leader using deterministic hash function.
        @param epoch: epoch number
        Note: for testing, should be taken care of in peer discovery. */
    pub fn add_public_key(&mut self, instance_name: String, pk: &PublicKey) {
        self.public_keys.insert(instance_name, pk.clone());  // TODO catch errors with insertion?
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
        let blk = Block::new(0, bytes, String::from("test"), 0, 0);

        // Create a message
        let mut message = Message {
            payload: MessagePayload::Block(blk),
            kind: MessageKind::Vote,
            nonce: 0,
            sender_id: 0,
            sender_name: String::from("test"),
            signatures: Some(Vec::new()),
        };

        // Signing message
        streamlet1.sign_message(&mut message);
        assert!(message.signatures.as_ref().unwrap().len() == 1);
        streamlet2.sign_message(&mut message);
        assert!(message.signatures.as_ref().unwrap().len() == 2);
        streamlet3.sign_message(&mut message);
        assert!(message.signatures.as_ref().unwrap().len() == 3);

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

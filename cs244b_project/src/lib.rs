mod network;
mod blockchain;
mod messages;
mod utils;

use std::collections::hash_map::DefaultHasher;
use std::hash::{Hasher};

use tokio::{
    io::{stdin, AsyncBufReadExt, BufReader},
    sync::mpsc,
    select,
};
use rand::Rng;

pub use utils::crypto::*;
pub use network::NetworkStack;
pub use blockchain::Block;
pub use messages::{Message, MessagePayload, MessageKind};

pub struct StreamletInstance {
    pub id: u32,
    num_instances: u32,
    keypair: Keypair,
    public_keys: Vec<PublicKey>,
}
enum EventType {
    UserInput(String),
    NetworkInput(Vec<u8>),
}

// ==========================
// === Core Streamlet API ===
// ==========================

impl StreamletInstance {

    // Creates a new Streamlet Instance
    pub fn new(id: u32) -> Self {
        // Setup public/private key pair
        let mut csprng = OsRng{};
        let keypair: Keypair = Keypair::generate(&mut csprng);
        let pk: PublicKey = keypair.public.clone();

        // Encode genesis block (probably should throw this in the chain instead)
        let mut hasher = Sha256::new();
        hasher.update(b"cs244b rocks!");
        let result = hasher.finalize();
        let bytes: Sha256Hash = result.as_slice().try_into().expect("slice with incorrect length");
        let genesis = Block {
            parent_hash: bytes,
            epoch: 0,
            data: String::from("this is the genesis block."),
        };

        // Build the streamlet instance
        Self {
            id: id,
            keypair: keypair, 
            num_instances: 1,
            public_keys: Vec::from([pk])
        }
    }

    // Main Streamlet event loop
    pub async fn run(&self) {
        // Initialize 
        // (1) message queue for the network to send us data
        // (2) message queue for us to receive data from the network
        let (net_sender, mut receiver) = mpsc::unbounded_channel();
        // Initialize the network stack
        let mut net_stack = network::NetworkStack::new("test_messages", net_sender).await;

        // Set up stdin
        let mut stdin = BufReader::new(stdin()).lines();

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
    
                    // Needs to be polled in order to make progress.
                    _ = net_stack.clear_unhandled_event() => {
                        None
                    },
                    
                }
            };
            if let Some(event) = evt {
                match event{
                    EventType::UserInput(_line) => {
                        println!("User input!");

                        let rand : u32 = rand::thread_rng().gen();
                        let message = Message {
                            payload: MessagePayload::String(_line),
                            kind: MessageKind::Test,
                            nonce: rand,
                            signatures: None,
                        };

                        println!("Sending message {:?}", message);
                        
                        net_stack.broadcast_message(message.serialize());
                    }
                    EventType::NetworkInput(bytes) => {
                        let message = Message::deserialize(&bytes);
                        println!("Received message: {:?}", message);
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
 
    fn sign(&self, bytes: &[u8]) -> Signature {
        return self.keypair.sign(bytes);
    }

    fn sign_message(&self, message: &mut Message) {
        match &message.signatures {
            Some(_) => {
                let signature: Signature = self.keypair.sign(message.serialize_payload().as_slice());
                let signatures: &mut Vec<Signature> = message.signatures.as_mut().unwrap();
                signatures.push(signature)
            },
            None => panic!("Tried to add signature to message without signature vector!"),
        }
    }

    fn verify_signature(&self, message: &Message, signature: &Signature, pk: &PublicKey) -> bool {
        let result = pk.verify(message.serialize_payload().as_slice(), signature);
        if let Err(error) = result {
            return false;
        } else {
            return true;
        }
    }

    // Kind of dumb, should make more efficient
    fn verify_message(&self, message: &Message) -> bool {
        let signatures = message.signatures.as_ref().unwrap();
        // Check all signatures
        for signature in signatures.iter() {
            // Check against all known pk's
            let mut success = false;
            for pk in self.public_keys.iter() {
                if self.verify_signature(message, signature, pk) {
                    success = true;
                    break;
                }
            }
            if !success {
                return false;
            }
        }
        return true;
    }

    fn get_epoch_leader(&self, epoch: u32) -> u32 {
        let mut hasher = DefaultHasher::new();
        hasher.write_u32(epoch);
        let result = hasher.finish() as u32;
        return result % self.num_instances;  // Assumes 0 indexing!
    }

    // For testing
    pub fn add_public_key(&mut self, pk: PublicKey) {
        self.public_keys.push(pk.clone());
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
        let streamlet = StreamletInstance::new(0);
        // Testing signatures
        let message: &[u8] = b"This is a test of the tsunami alert system.";
        let signature: Signature = streamlet.sign(message);
        let public_key: PublicKey = streamlet.get_public_key();
        assert!(public_key.verify(message, &signature).is_ok());
    }

    #[test]
    fn test_streamlet_msg_signatures() {
        let mut streamlet1 = StreamletInstance::new(0);
        let streamlet2 = StreamletInstance::new(1);
        let streamlet3 = StreamletInstance::new(2);

        // Create random hash
        let mut hasher = Sha256::new();
        hasher.update(b"hello world");
        let result = hasher.finalize();
        let bytes: Sha256Hash = result.as_slice().try_into().expect("slice with incorrect length");
        
        // Create a test block
        let blk = Block {
            parent_hash: bytes,
            epoch: 0,
            data: String::from("test"),
        };

        // Create a message
        let mut message = Message {
            payload: MessagePayload::Block(blk),
            kind: MessageKind::Vote,
            nonce: 0,
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
        streamlet1.add_public_key(streamlet2.get_public_key());
        let bad_result = streamlet1.verify_message(&message);
        assert!(bad_result == false);
        streamlet1.add_public_key(streamlet3.get_public_key());
        assert!(streamlet1.public_keys.len() == 3);

        // Verify message with all signatures
        let good_result = streamlet1.verify_message(&message);
        assert!(good_result == true);
    }
}
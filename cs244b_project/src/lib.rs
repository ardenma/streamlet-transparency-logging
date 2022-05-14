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

    fn sign_message(&mut self, message: &mut Message) {
        match &message.signatures {
            Some(_) => {
                let signature: Signature = self.keypair.sign(message.serialize_payload().as_slice());
                let signatures: &mut Vec<Signature> = message.signatures.as_mut().unwrap();
                signatures.push(signature)
            },
            None => panic!("Tried to add signature to message without signature vector!"),
        }
    }
 
    // fn send(&mut self, message: &mut Message, add_signature: bool) {
    //     if add_signature {
    //         match &message.signatures {
    //             Some(_) => {
    //                 let signature: Signature = self.keypair.sign(message.serialize_payload().as_slice());
    //                 let signatures: &mut Vec<Signature> = message.signatures.as_mut().unwrap();
    //                 signatures.push(signature)
    //             },
    //             None => panic!("Tried to add signature to message without signature vector!"),
    //         }   
    //     }
    //     self.outbound_messages.push_back(message.clone());
    // }

    // fn recv(&mut self) -> Option<Message> {
    //     return self.inbound_messages.pop_front();
    // }

    fn get_epoch_leader(&self, epoch: u32) -> u32 {
        let mut hasher = DefaultHasher::new();
        hasher.write_u32(epoch);
        let result = hasher.finish() as u32;
        return result % self.num_instances;  // Assumes 0 indexing!
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
}
/* A simple application providing an API similar to a true network directory
   service, modeled off of Tor.
*/

use bincode::{deserialize, serialize};
use log::{debug, info, error};
use rand::{Rng};
use serde::{Deserialize, Serialize};
// use crate::utils::crypto;
// use crate::messages::*;
use crate::app::app_interface::{APP_NET_TOPIC, APP_SENDER_ID, APP_NAME};
use crate::messages::*;
use crate::network::NetworkStack;
use crate::utils::crypto::*;
use crate::blockchain::{LocalChain, SignedBlock};
use rand::distributions::Alphanumeric;
use std::collections::HashSet;
use std::fmt;
use std::io::{Read, Write};
use tokio::{
    io::{stdin, AsyncBufReadExt, BufReader},
    select,
    sync::mpsc,
};
use std::net::{Shutdown, TcpStream, SocketAddr};

/* The data we append to our internal blockchain includes,
for each router on the network:
- The IP address of the router
- The public key that the router uses to set up connections
- A hash of the full router descriptor.
We generate this data randomly, but we base the structure
on the Tor directory specification:
 https://gitweb.torproject.org/torspec.git/tree/dir-spec.txt */
#[derive(Serialize, Deserialize, Debug, PartialEq)]
struct OnionRouterBasicData {
    ip: String,
    onion_key: String,
    full_hash: String,
}

impl fmt::Display for OnionRouterBasicData {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let s =
            format!(
                "* Router information:\nIP address (raw): {},\nOnion key (bytes): {},\nHash of descriptor: {}\n---\n",
                 self.ip,
                 self.onion_key,
                 self.full_hash
            );

        write!(f, "{}", s)
    }
}

#[derive(Serialize, Deserialize, Debug, PartialEq)]
struct OnionRouterNetDirectory {
    or_list: Vec<OnionRouterBasicData>,
}

impl fmt::Display for OnionRouterNetDirectory {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let mut s = String::new();
        for r in &self.or_list {
            s = format!("{}{}", s, r);
        }
        write!(f, "{}", s)
    }
}

impl OnionRouterBasicData {
    // Onion keys must be 1024 bits
    const KEY_SIZE: usize = 128;
    // Arbitrarily, assume use of SHA512
    const REMAINING_DATA_HASH_SIZE: usize = 64;

    // Populate with random data
    pub fn new() -> Self {
        let mut gen = rand::thread_rng();
        Self {
            ip: std::net::Ipv4Addr::new(
                gen.gen_range(0, 255),
                gen.gen_range(0, 255),
                gen.gen_range(0, 255),
                gen.gen_range(0, 255),
            )
            .to_string(),
            onion_key: gen
                .sample_iter(&Alphanumeric)
                .take(OnionRouterBasicData::KEY_SIZE)
                .map(char::from)
                .collect(),
            full_hash: gen
                .sample_iter(&Alphanumeric)
                .take(OnionRouterBasicData::REMAINING_DATA_HASH_SIZE)
                .map(char::from)
                .collect(),
        }
    }
}

impl OnionRouterNetDirectory {
    const MIN_SIZE: usize = 2;
    const MAX_SIZE: usize = 20;

    pub fn new() -> Self {
        let num_entries = rand::thread_rng().gen_range(
            OnionRouterNetDirectory::MIN_SIZE,
            OnionRouterNetDirectory::MAX_SIZE,
        );
        let mut s = Self {
            or_list: Vec::new(),
        };
        for _ in 0..num_entries {
            s.or_list.push(OnionRouterBasicData::new());
        }
        s
    }
}

/*** APPLICATION LOGIC ***/

enum AppEventType {
    UserInput(String),
    NetworkInput(Vec<u8>),
}

pub struct Application {
    keypair: Keypair,
    curr_nonce: u32,
    outstanding_requests: HashSet<u32>,
}

// State: KeyPair for signatures (eventually)

impl Application {
    pub fn new() -> Self {
        let mut csprng = OsRng {};
        let keypair = Keypair::generate(&mut csprng);
        Self {
            keypair: keypair,
            curr_nonce: 0,
            outstanding_requests: HashSet::new(),
        }
    }

    pub async fn run(&mut self) {

        let (net_sender, mut receiver) = mpsc::unbounded_channel();
        let mut net_stack = NetworkStack::new(APP_NET_TOPIC, net_sender).await;

        // Set up STDIN
        let mut stdin = BufReader::new(stdin()).lines();

        /* Main event loop:
          - User input, other than commands, triggers new blocks
          - Input from network is from the Streamlet implementation
        */
        loop {
            let evt = {
                select! {
                    line = stdin.next_line() => {
                        let line_data = line.expect("Can't get line").expect("Can't read from stdin");
                        Some(AppEventType::UserInput(line_data))
                    },
                    network_response = receiver.recv() => {
                        Some(AppEventType::NetworkInput(network_response.expect("Response doesn't exist.")))
                    },
                    _ = net_stack.clear_unhandled_event() => {
                        None
                    },
                }
            };
            if let Some(event) = evt {
                match event {
                    AppEventType::UserInput(_line) => {
                        // Process commands
                        if _line.starts_with("request block") {
                            // Request finalized block
                            let msg = self.make_latest_block_request();
                            info!("Made block request with tag: {}", msg.tag);

                            // Store message tag (expect the same tag back)
                            self.outstanding_requests.insert(msg.tag);
                            
                            // Send message to streamlet instances
                            net_stack.broadcast_message(
                                serialize(&msg).expect("Can't serialize msg from app!"),
                            );
                        } else if _line.starts_with("request chain") {
                            // Request finalized chain
                            let msg = self.make_latest_chain_request();
                            info!("Made chain request with tag: {}", msg.tag);

                            // Store message tag (expect the same tag back)
                            self.outstanding_requests.insert(msg.tag);
                            
                            // Send message to streamlet instances
                            net_stack.broadcast_message(
                                serialize(&msg).expect("Can't serialize msg from app!"),
                            );
                        } else {
                            // Otherwise: create a new directory
                            let msg = self.make_data();
                            net_stack.broadcast_message(
                                serialize(&msg).expect("Can't serialize msg from app!"),
                            );
                        }

                    }
                    AppEventType::NetworkInput(input) => {
                        // Received message
                        let message = Message::deserialize(&input);
                        info!("Received {:?} message from {}...", &message.kind, &message.sender_name);

                        match &message.kind {
                            MessageKind::AppBlockResponse => {
                                // For proof-of-concept: only process first response to an outstanding request with matching tag.
                                if self.outstanding_requests.contains(&message.tag) {
                                    // Process received block
                                    if let MessagePayload::SocketAddr(addr) = message.payload {
                                        self.request_block(&addr, &message);
                                    } else {
                                        debug!("Unkown payload for MessageKind::AppBlockResponse");
                                    }
                                    // Remove corresponding tag from outstanding requests
                                    self.outstanding_requests.remove(&message.tag);
                                }
                            }
                            MessageKind::AppChainResponse => {
                                // Again, for proof of concept: accept first chain 
                                if self.outstanding_requests.contains(&message.tag) {
                                    if let MessagePayload::SocketAddr(addr) = message.payload {
                                        self.request_chain(&addr, &message);
                                        // Remove corresponding tag from outstanding requests
                                        self.outstanding_requests.remove(&message.tag);
                                    } else {
                                        debug!("Unknown payload for MessageKind::AppChainResponse");
                                    }
                                }
                            }
                            _ => {
                                debug!("Unknown message format/kind - ignoring");
                            }
                        }
                    }
                }
            }
        }
    }

    fn make_data(&mut self) -> Message {
        let dir = OnionRouterNetDirectory::new();
        info!("Sending dir to Streamlet: {}", dir);
        let data =
            serialize(&dir).expect("Can't serialize a directory!");
        let sig = self.keypair.sign(&data);
        self.curr_nonce += 1;

        let mut msg = Message::new_with_defined_nonce(
            MessagePayload::AppData(data),
            MessageKind::AppSend,
            self.curr_nonce,
            APP_SENDER_ID,
            "app".to_string(),
        );
        msg.sign_message(sig);
        return msg;
    }

    /* Requests the lastest finalized block from streamlet */
    fn make_latest_block_request(&mut self) -> Message {
        self.curr_nonce += 1;

        let msg = Message::new_with_defined_nonce(
            MessagePayload::None,
            MessageKind::AppBlockRequest,
            self.curr_nonce,
            APP_SENDER_ID,
            "app".to_string(),
        );
        return msg;
    }

    /* Requests the lastest finalized chain from streamlet */
    fn make_latest_chain_request(&mut self) -> Message {
        self.curr_nonce += 1;

        let msg = Message::new_with_defined_nonce(
            MessagePayload::None,
            MessageKind::AppChainRequest,
            self.curr_nonce,
            APP_SENDER_ID,
            APP_NAME.to_string(),
        );
        return msg;
    }

    /* Request a block, presumed to be either the genesis block or an OnionRouterNetDirectory.
     */
    fn request_block(&mut self, addr: &SocketAddr, message: &Message) {
        let mut stream = TcpStream::connect(addr).expect("Couldn't connect to streamlet instance");

        // Request a block
        if let Err(e) = stream.write(&String::from("block").into_bytes()) {
            error!("Failed to request block: {:?}", e);   
        } else {
            // Close write stream
            stream.shutdown(Shutdown::Write).expect("Failed to close write stream");

            // Read the incoming data
            let mut msg = Vec::new();
            stream.read_to_end(&mut msg).unwrap();
            let SignedBlock { block, signatures} = deserialize(&msg).expect("Failed to deserialize block");

            // Close read stream
            if let Err(_) = stream.shutdown(Shutdown::Read) {
                // Note: gives error on macOS... https://doc.rust-lang.org/std/net/struct.TcpStream.html#method.shutdown
                debug!("Error shutting down TCP stream in application (normal on MacOS)");
            }

            
            if block.epoch == 0 { // Handle case of block being genesis
                info!("Recieved genesis block from {} with tag {}", &message.sender_name, message.tag);
            } else {
                let directory: OnionRouterNetDirectory =
                    deserialize(&block.data[..]).expect("Issues unwrapping directory data...");
                info!("Recieved directory data: {} from {}, with epoch {}, tag: {}, and signatures {:?}", directory, &message.sender_name, block.epoch, message.tag, &signatures);
            }
        }
    }

    fn request_chain(&mut self, addr: &SocketAddr, message: &Message) {
        // Process received block
    
        let mut stream = TcpStream::connect(addr).expect("Couldn't connect to streamlet instance");
        
        // Request a chain
        if let Err(e) = stream.write(&String::from("chain").into_bytes()) {
            error!("Failed to request chain: {:?}", e);   
        } else {
            // Close write stream
            stream.shutdown(Shutdown::Write).expect("Failed to close write stream");

            // Read the incoming data
            let mut msg = Vec::new();
            stream.read_to_end(&mut msg).unwrap();
            let chain: LocalChain = deserialize(&msg).expect("Failed to deserialize blockchain");
            info!("Recieved chain: {} from {} with tag {}", chain, &message.sender_name, message.tag);

            // Close read stream
            if let Err(_e) = stream.shutdown(Shutdown::Read) {
                // Note: gives error on macOS... https://doc.rust-lang.org/std/net/struct.TcpStream.html#method.shutdown
                debug!("Error shutting down TCP stream in application (normal on MacOS)");
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_app_serialization() {
        let dir = OnionRouterNetDirectory::new();
        let bytes = serialize(&dir).expect("Can't serialize directory");
        let dir2: OnionRouterNetDirectory =
            deserialize(&bytes[..]).expect("Can't deserialize directory");
        assert!(dir == dir2);
    }
}

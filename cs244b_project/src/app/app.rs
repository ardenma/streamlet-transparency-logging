/* A simple application providing an API similar to a true network directory
   service, modeled off of Tor.
*/

use bincode::{deserialize, serialize};
use log::{debug, info};
use rand::{AsByteSliceMut, Rng};
use serde::{Deserialize, Serialize};
// use crate::utils::crypto;
// use crate::messages::*;
use crate::app::app_interface::{APP_NET_TOPIC, APP_SENDER_ID};
use crate::messages::*;
use crate::network::NetworkStack;
use crate::utils::crypto::*;
use rand::distributions::Alphanumeric;
use std::collections::HashSet;
use std::fmt;
use tokio::{
    io::{stdin, AsyncBufReadExt, BufReader},
    select,
    sync::mpsc,
};

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
        // Set up network stack
        // BIG TODO: extend the network stack so that it works both
        // for streamlet and for the application.
        // Need an additional channel here.
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

                            // Store message nonce (expect the same nonce back)
                            self.outstanding_requests.insert(msg.nonce);
                            
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
                            info!("Sent directory to streamlet: {:?}", msg);
                        }

                    }
                    AppEventType::NetworkInput(_input) => {
                        // Received message
                        let message = Message::deserialize(&_input);
                        info!("Received {:?} message from {}...", &message.kind, &message.sender_name);

                        match &message.kind {
                            // TODO: currently, since streamlet broadcasts responses,
                            // we get multiple responses, either need to do point to point
                            // communication or can alternatively track which messages
                            // have outstanding responses
                            MessageKind::AppBlockResponse => {
                                // Only process first response to an outstanding request
                                // response is assumed to have the same nonce as the request...
                                // TODO: do something more clever?
                                if self.outstanding_requests.contains(&message.nonce) {
                                    // Process received block
                                    if let MessagePayload::Block(block) = message.payload {
                                        if block.epoch == 0 {
                                            info!("Recieved genesis block from {}", &message.sender_name);
                                        } else {
                                            let directory: OnionRouterNetDirectory =
                                                deserialize(&block.data[..]).expect("Issues unwrapping directory data...");
                                            info!("Recieved directory data: {:?} from {}", directory, &message.sender_name);
                                        }
                                    } else {
                                        debug!("Unkown payload for MessageKind::AppData");
                                    }
                                    
                                    // Remove corresponding nonce from outstanding requests
                                    self.outstanding_requests.remove(&message.nonce);
                                }
                            }
                            _ => {
                                debug!("Unknown message format/kind - ignoring");
                            }
                        }
                        // TODO
                        // - Match on message type
                        // - (If doing so) validate
                        // - Deserialize + print
                        // - Option: store?
                    }
                }
            }
        }
    }

    fn make_data(&mut self) -> Message {
        let data =
            serialize(&OnionRouterNetDirectory::new()).expect("Can't serialize a directory!");
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

    // TODO, potentially optional: two-way validation
    // - App signs new directories when created (function here)
    //   Signs with multiple keys to simulate multiple directories.
    // - App initializes (like peer_init on network stack)
    //   so that everyone has the right keys.
    // - Streamlet validates data received from app using PKs
    // - App validates data received from streamlet using PKs

    // TODO, potentially optional, for demo:
    // - Run a web server in a different thread here
    // - Implement a client that can make HTTP requests to it
    // - Send the directory and the streamlet blockchain
    // - (Like an IRL situation)
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

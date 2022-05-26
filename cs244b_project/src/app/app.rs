/* A simple application providing an API similar to a true network directory 
   service, modeled off of Tor. 
*/

use bincode::{deserialize, serialize};
use serde::{Deserialize, Serialize};
use log::info;
use rand::Rng;
// use crate::utils::crypto;
// use crate::messages::*;
use crate::network::NetworkStack;
use rand::distributions::Alphanumeric;
use tokio::{
    io::{stdin, AsyncBufReadExt, BufReader},
    select, 
    sync::mpsc,
};
use std::fmt;

/* Also used by the streamlet implementation. */
const APP_NET_TOPIC: &'static str = "app";

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
    ip: u32,       
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
            ip: gen.gen(), 
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
    const MIN_SIZE : usize = 2;
    const MAX_SIZE : usize = 20;
    
    pub fn new() -> Self {
        let num_entries = 
            rand::thread_rng()
            .gen_range(
                OnionRouterNetDirectory::MIN_SIZE, 
                OnionRouterNetDirectory::MAX_SIZE
            );
        let mut s = Self { or_list: Vec::new() };
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

pub struct Application;
// State: KeyPair for signatures (eventually)

impl Application { 

    pub fn new() -> Self {
        Self{}
    }

    pub async fn run(&self) {
        // Set up network stack
        // BIG TODO: extend the network stack so that it works both 
        // for streamlet and for the application. 
        // Need an additional channel here. 
        let (net_sender, mut receiver) = mpsc::unbounded_channel();
        let mut net_stack = NetworkStack::new(
            APP_NET_TOPIC,
            net_sender
        ).await;
        
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

                        // Otherwise: create a new directory 
                        let dir = OnionRouterNetDirectory::new();
                        net_stack.broadcast_message(
                            serialize(&dir).expect("Can't serialize net directory")
                        );
                        info!("Sent directory to streamlet: {}", dir);
                    }
                    AppEventType::NetworkInput(_input) => {
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
        let bytes = serialize(&dir)
            .expect("Can't serialize directory");
        let dir2 : OnionRouterNetDirectory = deserialize(&bytes[..])
            .expect("Can't deserialize directory");
        assert!(dir == dir2);
    }
}
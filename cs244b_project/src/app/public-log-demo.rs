/* A simple service meant to model a "contract" with a public log. 

- TODO: For if we want to go this route instead
*/

use crate::utils::crypto::*;
use crate::messages::*; 

pub struct CheckpointData {
    block_hash: Sha256Hash,
    epoch: u64,
}

pub struct PublicLog {
    received_data: Vec<CheckpointData>,
}

enum LogEventType {
    UserInput(String),
    NetworkInput(Vec<u8>),
}

pub const LOG_NET_TOPIC: &str = "public_log";

impl PublicLog {
    pub fn new() -> Self {
        Self{ received_data: Vec::new() }
    }

    pub async fn run() {
        pub async fn run(&mut self) {

            // Set up network stack 
            let (net_sender, mut receiver) = mpsc::unbounded_channel();
            let mut net_stack = NetworkStack::new(LOG_NET_TOPIC, net_sender).await;
    
            // Set up STDIN
            let mut stdin = BufReader::new(stdin()).lines();

            loop {
                let evt = {
                    select! {
                        line = stdin.next_line() => {
                            let line_data = line.expect("Can't get line").expect("Can't read from stdin");
                            Some(LogEventType::UserInput(line_data))
                        },
                        network_response = receiver.recv() => {
                            Some(NetworkInput::NetworkInput(network_response.expect("Response doesn't exist.")))
                        },
                        _ = net_stack.clear_unhandled_event() => {
                            None
                        },
                    }
                };

                if let Some(event) = evt { 
                    match event {
                        LogEventType::UserInput(line) => {
                            // Do nothing
                        }
                        LogEventType::NetworkInput(input) => {
                            let message = Message::deserialize(&input);
                            // Etc. 
                        }
                    }
                }

            }

    }
}
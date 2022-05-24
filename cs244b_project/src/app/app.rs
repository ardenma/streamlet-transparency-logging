
use bincode::{deserialize, serialize};
use serde::{Deserialize, Serialize};
use std::collections::{VecDeque, HashSet};
use std::convert::TryFrom;
use log::info;
use std::fs::File;
use std::fs;
use std::path::Path;
use rand::Rng;
use crate::utils::crypto;
use std::time::Duration;
use crate::network::NetworkStack;
use rand::distributions::Alphanumeric;
use tokio::{
    io::{stdin, AsyncBufReadExt, BufReader},
    select, spawn,
    sync::mpsc,
    time::sleep,
};

const APP_CHANNEL: &str = "app";

enum AppEventType {
    UserInput(String),
    NetworkInput(Vec<u8>),
}

// Based on real Tor onion router self-signed certs: 
// https://gitweb.torproject.org/torspec.git/tree/cert-spec.txt
const CERT_SIZE: usize = 104;
// Onion keys must be 1024 bits
// https://gitweb.torproject.org/torspec.git/tree/dir-spec.txt
const KEY_SIZE: usize = 128;
// Basic 512-bit hash of the router
const REMAINING_DATA_HASH: usize = 64;

// The "most important" information for an Onion Router. 
#[derive(Serialize, Deserialize, Debug)]
struct OnionRouterBasicData {
    address: u32, 
    identity_ed25519: String, 
    onion_key: String,
    full_hash: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct SimpleNetDir {
    or_list: Vec<OnionRouterBasicData>,
}

pub struct SimpleTorDirectory {
    dirs_to_send: VecDeque<SimpleNetDir>,
    #[allow(dead_code)]
    pending_dirs: VecDeque<SimpleNetDir>,
    #[allow(dead_code)]
    finalized_dirs: VecDeque<SimpleNetDir>,
    channel: String,
}

impl OnionRouterBasicData {
    // Populate with random data
    pub fn new() -> Self {
        Self {
            address: rand::thread_rng().gen(), 
            identity_ed25519: rand::thread_rng()
                .sample_iter(&Alphanumeric)
                .take(CERT_SIZE)
                .map(char::from)
                .collect(), 
            onion_key: rand::thread_rng()
                .sample_iter(&Alphanumeric)
                .take(KEY_SIZE)
                .map(char::from)
                .collect(), 
            full_hash: rand::thread_rng()
                .sample_iter(&Alphanumeric)
                .take(REMAINING_DATA_HASH)
                .map(char::from)
                .collect(), 
        }
    }
}

impl SimpleNetDir {
    pub fn new() -> Self {
        let num_entries = rand::thread_rng().gen_range(2,20);
        let mut or_list = Vec::new();
        for _ in 2..num_entries {
            or_list.push(OnionRouterBasicData::new());
        }
        Self {
            or_list: or_list,
        }
    }
}

impl SimpleTorDirectory {
    pub fn new() -> Self {

        Self {
            dirs_to_send: VecDeque::from([SimpleNetDir::new()]),
            pending_dirs: VecDeque::new(),
            finalized_dirs: VecDeque::new(),
            channel: APP_CHANNEL.to_string(),
        }

    }

    pub async fn run(&mut self) {
        // Initialize the network stack
        let (net_sender, mut receiver) = mpsc::unbounded_channel();
        let mut net_stack = NetworkStack::new(vec!(&self.channel), net_sender).await;

        // Set up stdin
        let mut stdin = BufReader::new(stdin()).lines();

        loop {
            let evt = {
                select! {
                    line = stdin.next_line() => {
                        let line_data = line.expect("Can't get line").expect("Can't read from stdin");
                        Some(AppEventType::UserInput(line_data))
                    },
                    // When the network receives *any* message, it forwards the data to us thru this channel
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
                        if self.dirs_to_send.is_empty() {
                            self.dirs_to_send.push_front(SimpleNetDir::new());
                        }
                        let msg = self.dirs_to_send.pop_front().unwrap();
                        net_stack.broadcast_message(&self.channel, serialize(&msg).expect("Can't serialize message."));
                    }
                    AppEventType::NetworkInput(_input) => {

                    }
                }
            }
        }

    }
}


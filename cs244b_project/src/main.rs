/*
    To get INFO macro printouts, RUST_LOG=info cargo run
*/
use tokio::{
    io::{stdin, AsyncBufReadExt, BufReader},
    sync::mpsc,
    select,
    time::sleep,
    spawn,
};
use std::time::Duration;
use serde::{Deserialize, Serialize};

use rand::Rng;

mod network;
mod peer_init;

// Demo of the networking API 

enum EventType {
    UserInput(String),
    NetworkInput(Vec<u8>),
    DoInit,
}

#[derive(Deserialize, Serialize)]
pub struct Msg {
    data: String,
}

const DEFAULT_NUM_HOSTS: usize = 2;

#[tokio::main]
async fn main() {

    pretty_env_logger::init();

    /* Parse optional CL args: <expected peers> <name of this host> */
    let args : Vec<String> = std::env::args().collect();
    let expected_peer_count = {
        if args.len() >= 2 {
            let num_hosts = args[1].clone().parse::<usize>().expect("(Optional) first argument should be host count.");
            // Number of peers = num_hosts - this node 
            num_hosts - 1
        } else {
            DEFAULT_NUM_HOSTS - 1
        }
    };
    let name = { 
        if args.len() >= 3 {
            args[2].clone()
        } else {
            String::new()
        }  
    };

    // Initialize 
    // (1) message queue for the network to send us data
    // (2) message queue for us to receive data from the network
    let (net_sender, mut receiver) = mpsc::unbounded_channel();
    // Initialize the network stack
    let mut net_stack = network::NetworkStack::new("test_messages", net_sender).await;

    // Set up stdin
    let mut stdin = BufReader::new(stdin()).lines();

    // Set up what we need to initialize the peer discovery protocol
    let mut peers = peer_init::Peers::new(name.clone());
    let (trigger_init, mut recv_init) = mpsc::channel(1);
    let mut needs_init = true;
    spawn(async move { // trigger after MDNS has had a chance to do its thing
        sleep(Duration::from_secs(1)).await;
        trigger_init.send(0).await.expect("can't send init event");
    });

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

                // Needs to be polled in order to make progress.
                _ = net_stack.clear_unhandled_event() => {
                    None
                },
                
            }
        };
        if let Some(event) = evt {
            
            match event{
                EventType::UserInput(line) => {
                    if line.starts_with("end discovery") || line.starts_with("e d") {
                        peers.send_end_init(&mut net_stack);
                    } else {
                        println!("User input!");
                        let rand : u32 = rand::thread_rng().gen();
                        println!("Sending message with nonce: {}", rand);
                        let msg = Msg{ data : String::from(format!("Hello from {}", rand)) };
                        net_stack.broadcast_message(serde_json::to_vec(&msg).expect("Can't serialize message!"));
                    }
                }
                EventType::NetworkInput(bytes) => {
                    println!("Message is {} bytes", bytes.len());
                    if let Ok(ad) = serde_json::from_slice::<peer_init::PeerAdvertisement>(&bytes) {
                        peers.recv_advertisement(ad, &mut net_stack);
                    }
                    else if let Ok(msg) = serde_json::from_slice::<Msg>(&bytes) {
                        println!("Received message: {}", msg.data);
                    }
                }
                EventType::DoInit => {
                    peers.start_init(&mut net_stack, expected_peer_count);
                }
            }
        }
    }
}

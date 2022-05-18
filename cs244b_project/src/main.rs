use tokio::{
    io::{stdin, AsyncBufReadExt, BufReader},
    sync::mpsc,
    select,
};

use rand::Rng;

mod network;
mod app;

// Demo of the networking API 

enum EventType {
    UserInput(String),
    NetworkInput(Vec<u8>),
}

#[tokio::main]
async fn main() {

    let args : Vec<String> = std::env::args().collect();
    if args[1] == "setup" {
        app::init(args);
        return;
    }

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
                    println!("Sending message with nonce: {}", rand);
                    net_stack.broadcast_message(String::into_bytes(format!("Hello out there from {}", rand)));
                }
                EventType::NetworkInput(bytes) => {
                    println!("Received message: {}", String::from_utf8(bytes).expect("Can't decode message to string."));
                }
            }
        }
    }

}

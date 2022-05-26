use tokio;

use cs244b_project::StreamletInstance;

const DEFAULT_NUM_HOSTS: usize = 2;

#[tokio::main]
async fn main() {
    pretty_env_logger::init();

    /* Parse optional CL args: */
    let args: Vec<String> = std::env::args().collect();
    
    /* - For application (net directory service): app */
    if args.len() == 2 && args[1].starts_with("app") {
        cs244b_project::run_app().await;
        // Run the app and return. 
        return;
    }

    /* - For streamlet: <expected peers> <name of this host> */
    let expected_peer_count = {
        if args.len() >= 2 {
            let num_hosts = args[1]
                .clone()
                .parse::<usize>()
                .expect("(Optional) first argument should be host count.");
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

    let mut streamlet = StreamletInstance::new(name, expected_peer_count);

    // Probably want to setup the id, num instances, exchange keys, etc.
    streamlet.run().await; // Runs libp2p event loop
}

use tokio;

use cs244b_project::{StreamletInstance};

#[tokio::main]
async fn main() {
    let streamlet = StreamletInstance::new(0);
    // Probably want to setup the id, num instances, exchange keys, etc.
    streamlet.run().await;  // Runs libp2p event loop
}

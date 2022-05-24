use tokio;

mod blockchain;
mod messages;
mod network;
mod utils;
mod app;

#[tokio::main]
async fn main() {
    let mut demo_app = app::SimpleTorDirectory::new();
    demo_app.run().await;
}
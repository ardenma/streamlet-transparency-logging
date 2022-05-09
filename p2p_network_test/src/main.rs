mod network; 

#[tokio::main]
async fn main() {
    let arg = std::env::args().nth(1).expect("Expecting at least one argument.").clone();
    println!("Hello, world from {}", arg);
    network::network_main("test_messages".to_string(), arg).await.unwrap();
}

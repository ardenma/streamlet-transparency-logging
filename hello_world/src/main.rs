use std::env;
use std::{thread, time};

fn main() {
    let args: Vec<String> = env::args().collect();
    println!("Hello, world from {}", args[1]);

    thread::sleep(time::Duration::from_secs(2));

    println!("Hello again from {}", args[1]);
}

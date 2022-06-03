use log::info;
use tokio;
use tokio::io::AsyncWriteExt;
use tokio::process::Command;
use tokio::time::sleep;
use std::time::Duration;
use std::process::exit;


use cs244b_project::{StreamletInstance, OperationMode, BenchmarkDataType};

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
    let epoch_length = {
        if args.len() >= 4 {
            args[3].clone().parse::<u64>().expect("(Optional) third argument should be epoch length in seconds")
        } else {
            10  // 10s default epoch duration
        }
    };
    let data_type = {
        if args.len() >= 5 {
           args[4].clone()
        } else {
            String::from("?")
        }
    };
    let config = {
        if args.len() >= 6 {
            args[5].clone()
        } else {
            String::from("normal")
        }
    };

    match config.as_str() {
        "benchmark" => {
            info!("Starting benchmark master node...");
            std::fs::create_dir_all("./logs");
            let mut children = Vec::new();
            for i in 0..(expected_peer_count+1) {
                let num_hosts = expected_peer_count + 1;
                let mut child = Command::new("./target/debug/cs244b_project").arg(format!("{num_hosts}")).arg(format!("Test{i}")).arg(format!("{epoch_length}")).arg(format!("{data_type}")).arg("benchmark-worker").spawn().expect("failed to spawn");
                children.push(child);
            }
            
            for mut child in children {
                let _ = child.wait().await;
            }
            exit(0);
        },
        "benchmark-worker" => {
            info!("Starting benchmark worker node {name}...");
            let mut streamlet = StreamletInstance::new(name, expected_peer_count, epoch_length, OperationMode::Benchmark);
            let data_type = match args[4].clone().as_str() {
                "small" => BenchmarkDataType::Small,
                "medium" => BenchmarkDataType::Medium,
                "large" => BenchmarkDataType::Large,
                _ => panic!("unrecognized data type")
            };
            streamlet.run(data_type).await; // Runs libp2p event loop
            exit(1);
        }
        _ => {
            info!("Starting normal operation node {name}...");
            let mut streamlet = StreamletInstance::new(name, expected_peer_count, epoch_length, OperationMode::Normal);
            streamlet.run(BenchmarkDataType::Small).await; // Runs libp2p event loop
        }
    }

}

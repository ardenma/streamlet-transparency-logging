use log::info;
use tokio;
use tokio::io::AsyncWriteExt;
use tokio::process::Command;
use tokio::time::sleep;
use std::time::Duration;
use std::process::exit;


use cs244b_project::{StreamletInstance, OperationMode, BenchmarkDataType, CompromiseType};

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
    let compromise_flags = {
        if args.len() >= 6 {
            args[5].clone()
        } else {
            String::from("r")  // no compromise
        }
    };
    let config = {
        if args.len() >= 7 {
            args[6].clone()
        } else {
            String::from("normal")
        }
    };
    let compromised_nodes = {
        if args.len() >= 8 {
            args[7].clone().parse::<usize>().expect("Last argument should be number of nodes to compromise")
        } else {
            0  // 10s default epoch duration
        }
    };

    // Parse flags
    let compromise_types = {
        if args.len() >= 6 {
            let mut tmp = Vec::new();
            for c in compromise_flags.chars() {
                let compromise = match c {
                    'r' => CompromiseType::NoCompromise,
                    'e' => CompromiseType::EarlyEpoch,
                    'l' => CompromiseType::LateEpoch,
                    'p' => CompromiseType::NoPropose,
                    'h' => CompromiseType::WrongParentHash,
                    'v' => CompromiseType::NoVote,
                    'n' => CompromiseType::NonLeaderPropose,
                    _ => CompromiseType::NoCompromise
                };
                tmp.push(compromise);
            }
            tmp
        } else {
            Vec::from([CompromiseType::NoCompromise])
        }
    };

    match config.as_str() {
        "benchmark" => {
            info!("Starting benchmark master node...");
            std::fs::create_dir_all("./logs");
            let mut children = Vec::new();
            let total_nodes = expected_peer_count + 1;
            let honest_nodes = total_nodes - compromised_nodes;

            // Spawn honest nodes
            for i in 0..honest_nodes {
                let mut child = Command::new("./target/debug/cs244b_project").arg(format!("{total_nodes}")).arg(format!("Test{i}")).arg(format!("{epoch_length}")).arg(format!("{data_type}")).arg("r").arg("benchmark-worker").spawn().expect("failed to spawn");
                children.push(child);
            }
            
            // Spawn compromised nodes
            for i in honest_nodes..total_nodes {
                let mut child = Command::new("./target/debug/cs244b_project").arg(format!("{total_nodes}")).arg(format!("Test{i}")).arg(format!("{epoch_length}")).arg(format!("{data_type}")).arg(compromise_flags.as_str()).arg("benchmark-worker").spawn().expect("failed to spawn");
                children.push(child);
            }
            
            for mut child in children {
                let _ = child.wait().await;
            }
            exit(0);
        },
        "benchmark-worker" => {
            info!("Starting benchmark worker node {name}, received compromise flags {compromise_flags}...");
            let mut streamlet = StreamletInstance::new(name, expected_peer_count, epoch_length, OperationMode::Benchmark, compromise_types);
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
            let mut streamlet = StreamletInstance::new(name, expected_peer_count, epoch_length, OperationMode::Normal, compromise_types);
            streamlet.run(BenchmarkDataType::Small).await; // Runs libp2p event loop
        }
    }

}

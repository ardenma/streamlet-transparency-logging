use std::fs::File;
use serde::{Deserialize, Serialize};

static BLOCK_DATA : &str = "network_data/simple_data.json";


#[derive(Serialize, Deserialize, Debug)]
struct RouterData {
    nickname: String, 
    address: String, 
    or_port: String, 
    socks_port: String, 
    dir_port: String, 
}


#[derive(Serialize, Deserialize, Debug)]
struct OrData {
    router: RouterData, 
    identity_ed25519: String, 
    master_key_ed25519: String, 
    bandwidth: String,
    platform: String, 
    published: String,
    onion_key: String, 
    onion_key_crosscert: String, 
    ipv6_policy: String, 
    contact: String,
}


#[derive(Serialize, Deserialize, Debug)]
pub struct NetDir {
    or_list: Vec<OrData>,
} 


#[derive(Serialize, Deserialize, Debug)]
pub struct AllNetDirs {
    dir_list: Vec<NetDir>,
    cur_idx: usize,
}

impl AllNetDirs {
    // Read in block data from file
    // Expect a list of net directories
    
    pub fn new() -> Self {
        let all_data = 
            serde_json::from_reader(
            File::open(BLOCK_DATA)
                .expect("Failed to open block data file"))
            .expect("Failed to parse block data");
        all_data
    }

    
    pub fn pop_next(&mut self) -> String {
        let i = self.cur_idx.clone();
        self.cur_idx += 1;
        let ret = self.dir_list.get(i);
        if let Some(dir) = ret {
            return serde_json::to_string(dir).unwrap()
        } else {
            return "".to_string()
        }
    }

    #[allow(dead_code)]
    pub fn peek_next(&self) -> String {
        let ret = self.dir_list.get(self.cur_idx);
        if let Some(dir) = ret {
            return serde_json::to_string(dir).unwrap()
        } else {
            return "".to_string()
        }
    }

    #[allow(dead_code)]
    pub fn is_more_data(&self) -> bool {
        self.cur_idx < self.dir_list.len()
    }

}


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_network_dir() {
        let mut net_data = AllNetDirs::new();
        println!("Net data: {:?}", net_data);
        println!("Printing data...");
        let mut prev_data = "".to_string();
        while net_data.is_more_data() {
            let peek_data = net_data.peek_next();
            let this_data = net_data.pop_next();
            assert!(peek_data == this_data);
            println!("{:?}", this_data);
            println!("");
            // Check that we're printing new data each time
            assert!(this_data != prev_data);
            prev_data = this_data;
        } 
        // To force stdout:
        // assert!(false);
    }
}
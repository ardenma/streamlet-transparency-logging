/* 
    Meant to provide an API similar to what a directory service 
    using our Streamlet implementation would provide. 

    We assume this works like Tor: 
    - The data we append to our blockchain consists of 
      (1) the hash of a network directory -- e.g., what would be returned to a Tor client;
      (2) signatures on this hash from various directory servers.
    - A block is considered "valid" if it is signed by a majority of directory servers. 

    NOTE: this is a strawman, quick implementation. 
    It seems to work; AND, the data it produces is long (800 bytes for 3 fake "servers"), 
    because signatures are long. 
 */


use serde::{Serialize, Deserialize};
use std::collections::{VecDeque, HashSet};
use std::convert::TryFrom;
use log::info;
use std::fs::File;
use std::fs;
use std::path::Path;

// Would be in the crypto mod
use ed25519_dalek::{Keypair, PublicKey, Signature, Signer, Verifier};
use rand::rngs::{OsRng};
use rand::Rng;
// Would be in the crypto mod
type Sha256Hash = [u8; 32];

#[derive(Serialize, Deserialize, Debug, Clone)]
struct DirectoryEntry {
    hash: Sha256Hash,
    signatures: Vec<Signature>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct DirectoryService {
    public_keys: Vec<PublicKey>,
    entries: VecDeque<DirectoryEntry>,
}

static FILENAME : &str = "directory_data.json";
static DIRS_PER_NODE : usize = 20;
static NUM_DIR_SERVERS: usize = 3;

impl DirectoryService {

    pub fn new(my_id: usize) -> Self {
        // Read in from file FILENAME
        let dirs : Vec<DirectoryService> = 
            serde_json::from_reader(
            File::open(FILENAME)
                .expect("Failed to initialize from file")
            )
            .expect("Failed to parse directory data");
        
        dirs.get(my_id)
            .expect("ID was entered incorrectly!")
            .clone()
    }

    pub fn is_valid(&self, bytes: &Vec<u8>) -> bool {
        let res : Result<DirectoryEntry, serde_json::Error> = serde_json::from_slice(bytes);

        // If it's not in the format of a directoy entry, consider this invalid
        if let Err(_e) = res {
            return false;
        }

        // Otherwise, ensure there are >n/2 valid signatures
        else if let Ok(dir) = res {
            let mut valid_signatures = HashSet::new();
            for sig in &dir.signatures {
                // Valid if it matches with a public key we have stored
                let mut valid_idx = -1;
                for i in 0..(self.public_keys.len()) {
                    if let Ok(()) = self.public_keys[i].verify(&dir.hash, sig) {
                        valid_idx = i32::try_from(i).unwrap();
                        break;
                    }
                }

                // Count valid signatures
                if valid_idx >= 0 {
                    valid_signatures.insert(valid_idx);
                    if valid_signatures.len() > self.public_keys.len() / 2 {
                        // If we hit >n/2 signatures, this will return true. 
                        return true;
                    }
                } 
            }
        } 
        false
    }

    pub fn next(&mut self) -> Option<Vec<u8>> {
        let dir = self.entries.pop_front()?;
        Some(serde_json::to_vec(&dir).expect("Can't serialize directory!"))
    }

}

pub fn init(args: Vec<String>) {
    // Clear state 
    if let Ok(()) = fs::remove_file(FILENAME) {
        println!("Removing old network file...");
    }
    assert!(!Path::new(FILENAME).exists());

    // Figure out number of nodes
    let num_nodes = args
                          .get(2)
                          .expect("For initialization, must specify the number of peers.")
                          .parse::<usize>()
                          .expect("Couldn't parse number of peers.");
    
    
    // Create keypairs 
    let mut all_keys = Vec::<Keypair>::new();
    let mut public_keys = Vec::<PublicKey>::new();
    let mut csprng = OsRng{};
    for _ in 0..NUM_DIR_SERVERS {
        let keypair = Keypair::generate(&mut csprng);
        public_keys.push(keypair.public.clone());
        all_keys.push(keypair);
    } 

    // Create container for directory services
    let mut all_dirs = Vec::<DirectoryService>::new();
    
    info!("Initializing (dummy) directory data for {} nodes...", num_nodes);

    // Create a DirectoryService for each node
    for _i in 0..num_nodes {
        all_dirs.push(init_one_directory_service(&all_keys, &public_keys));
    }
    
    fs::write(FILENAME, 
        serde_json::to_vec(&all_dirs).expect("Can't serialize"))
        .expect("Can't write directory data to file.");

    println!("Saved dummy directory data to {}", FILENAME);
}

fn init_one_directory_service(all_keys: &Vec<Keypair>, 
                                public_keys: &Vec<PublicKey>) -> DirectoryService {
    
    let mut ret = DirectoryService{ 
        public_keys: public_keys.clone(), 
        entries: VecDeque::new() 
    };

    for _j in 0..DIRS_PER_NODE {
        let mut dir = DirectoryEntry{ 
            hash: rand::thread_rng().gen::<Sha256Hash>(), 
            signatures: Vec::new() 
        };

        // Just sign with a majority of keys.
        for x in 0..(all_keys.len() / 2 + 1) {
            dir.signatures.push(all_keys[x].sign(&dir.hash));
        }
        ret.entries.push_back(dir);
    }

    ret
}


#[cfg(test)]
mod tests {

    use super::*;

    fn test_valid(services: &mut Vec<DirectoryService>) {
        // Check validity 
        for i in 0..5 {
            let mut next = services[i].next();
            while next.is_some() {
                let val = next.clone().unwrap();
                for j in 0..5 {
                    assert!(services[j].is_valid(&val));
                }
                let new_next = services[i].next();
                assert!(new_next != next);
                next = new_next;
            }
        } 
    }

    fn test_invalid(services: &mut Vec<DirectoryService>) {
        // Bogus data
        let mut bytes = rand::thread_rng().gen::<[u8; 32]>().to_vec();
        bytes.append(&mut rand::thread_rng().gen::<[u8; 32]>().to_vec());
        assert!(!services[0].is_valid(&bytes));

        // Forge signatures: 
        let mut all_keys = Vec::<Keypair>::new();
        let mut public_keys = Vec::<PublicKey>::new();
        let mut csprng = OsRng{};
        for _ in 0..NUM_DIR_SERVERS {
            let keypair = Keypair::generate(&mut csprng);
            public_keys.push(keypair.public.clone());
            all_keys.push(keypair);
        } 
        let dir = init_one_directory_service(&all_keys, &public_keys);
        let bytes = serde_json::to_vec(&dir).unwrap();
        assert!(!services[0].is_valid(&bytes));

        // Remove signatures to get no majority: 
        let mut dir : DirectoryEntry = 
            serde_json::from_slice(&services[0].next().unwrap())
            .unwrap();
        while dir.signatures.len() > NUM_DIR_SERVERS / 2 {
            dir.signatures.remove(dir.signatures.len() - 1);
        }
        assert!(!services[0].is_valid(&serde_json::to_vec(&dir).unwrap()));

        // Add back in duplicate signatures -- should still be invalid
        while dir.signatures.len() > NUM_DIR_SERVERS / 2 {
            dir.signatures.push(dir.signatures.last().unwrap().clone());
        }
        assert!(!services[0].is_valid(&serde_json::to_vec(&dir).unwrap()));
    }

    #[test]
    fn test_all() {
        // Init with 5 dir servers
        init(vec!["executable".to_string(), "setup".to_string(), "5".to_string()]);
        assert!(Path::new(FILENAME).exists());
        // Read in the data:
        let mut services = Vec::new(); 
        for i in 0..5 {
            services.push(DirectoryService::new(i));
        }
        assert!(services[0].public_keys.len() == NUM_DIR_SERVERS);
        test_invalid(&mut services);
        test_valid(&mut services);
    }
}


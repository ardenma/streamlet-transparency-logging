use sha2::{Sha256, Digest};
use serde::{Serialize, Deserialize};
    
// This was in the utils/crypto, but I couldn't get the import to resolve
type Sha256Hash = [u8; 32];

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Block {
    pub hash: Sha256Hash,
    pub data: String,
    pub prev_hash: String,
}

impl Block {

    pub fn new(data: String) -> Self {
        let mut hasher = Sha256::new();
        hasher.update(&data);
        let result = hasher.finalize();
        let bytes: Sha256Hash = result.as_slice().try_into().expect("slice with incorrect length");
        Self{ hash: bytes, data: data, prev_hash: "".to_string() }
    }
 
    pub fn hash(&self) -> Sha256Hash {
        // create a Sha256 object
        let mut hasher = Sha256::new();

        // add block fields
        hasher.update(self.hash.as_slice()); 
        hasher.update(self.data.as_bytes());

        // read hash digest and consume hasher
        let result = hasher.finalize();
        let bytes = result.as_slice().try_into().expect("slice with incorrect length");
        bytes
    }

    #[allow(dead_code)]
    pub fn validate_block(&self, _prev_block: &Block) -> bool {
        // ToDo: implement
        true
    }
}

pub fn hash(data: &String) -> Sha256Hash {
    let mut hasher = Sha256::new();
    // add block fields
    hasher.update(data.as_bytes()); 
    hasher.finalize().as_slice().try_into().expect("slice with incorrect length")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_block_hash() {
        // Create random hash
        let mut hasher = Sha256::new();
        hasher.update(b"hello world");
        let result = hasher.finalize();
        let bytes: Sha256Hash = result.as_slice().try_into().expect("slice with incorrect length");

        // Create some blocks
        let blk1 = Block {
            hash: bytes,
            data: String::from("foo"),
            prev_hash: "".to_string()
        };
        let blk2 = Block {
            hash: bytes,
            data: String::from("bar"),
            prev_hash: "".to_string()
        };
        let blk3 = Block {
            hash: bytes,
            data: String::from("bar"),
            prev_hash: "".to_string()
        };

        // println!("{:?}", blk1.hash());
        // println!("{:?}", blk2.hash());
        // println!("{:?}", blk1.hash() == blk2.hash());
        // println!("{:?}", blk2.hash() == blk3.hash());
        assert_ne!(blk1.hash(), blk2.hash());
        assert_eq!(blk2.hash(), blk3.hash());
    }
}
use sha2::{Sha256, Digest};
use serde::{Serialize, Deserialize};

use crate::utils::crypto::*;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Block {
    pub parent_hash: Sha256Hash,
    pub epoch: u32,
    pub data: String,
}

impl Block {
    pub fn hash(&self) -> Sha256Hash {
        // create a Sha256 object
        let mut hasher = Sha256::new();

        // add block fields
        hasher.update(self.parent_hash.as_slice());
        hasher.update(self.epoch.to_ne_bytes().as_slice());
        hasher.update(self.data.as_bytes());

        // read hash digest and consume hasher
        let result = hasher.finalize();
        let bytes = result.as_slice().try_into().expect("slice with incorrect length");
        return bytes;
    }
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
            parent_hash: bytes,
            epoch: 0,
            data: String::from("foo"),
        };
        let blk2 = Block {
            parent_hash: bytes,
            epoch: 0,
            data: String::from("bar"),
        };
        let blk3 = Block {
            parent_hash: bytes,
            epoch: 0,
            data: String::from("bar"),
        };

        // println!("{:?}", blk1.hash());
        // println!("{:?}", blk2.hash());
        // println!("{:?}", blk1.hash() == blk2.hash());
        // println!("{:?}", blk2.hash() == blk3.hash());
        assert_ne!(blk1.hash(), blk2.hash());
        assert_eq!(blk2.hash(), blk3.hash());
    }
}
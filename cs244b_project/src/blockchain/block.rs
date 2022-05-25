pub use crate::utils::crypto::*;
use crate::Sha256Hash;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Block {
    pub epoch: i64,
    pub hash: Sha256Hash,
    pub parent_hash: Sha256Hash,
    pub data: String, // some stringified version of a vec<Transaction>
    pub nonce: u64,
}

impl Block {
    pub fn new(epoch: i64, parent_hash: Sha256Hash, payload: String, nonce: u64) -> Self {
        // Create random hash
        let mut hasher = Sha256::new();
        hasher.update(&payload);
        let result = hasher.finalize();
        let bytes: Sha256Hash = result
            .as_slice()
            .try_into()
            .expect("slice with incorrect length");

        Self {
            epoch,
            hash: bytes,
            parent_hash,
            data: payload,
            nonce,
        }
    }

    pub fn hash(&self) -> Sha256Hash {
        // create a Sha256 object
        let mut hasher = Sha256::new();

        // add block fields
        hasher.update(self.parent_hash.as_slice());
        hasher.update(self.epoch.to_ne_bytes().as_slice());
        hasher.update(self.data.as_bytes());
        hasher.update(self.nonce.to_ne_bytes().as_slice());

        // read hash digest and consume hasher
        let result = hasher.finalize();
        let bytes = result
            .as_slice()
            .try_into()
            .expect("slice with incorrect length");
        return bytes;
    }

    pub fn generate_test_block(data: &String) -> Block {
        let mut hasher = Sha256::new();
        hasher.update(b"hello world");
        let result = hasher.finalize();
        let bytes: Sha256Hash = result
            .as_slice()
            .try_into()
            .expect("slice with incorrect length");

        return Block::new(-1, bytes, String::from("test"), 0);
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
        let bytes: Sha256Hash = result
            .as_slice()
            .try_into()
            .expect("slice with incorrect length");

        // Create some blocks
        let blk1 = Block::new(0, bytes, String::from("foo"), 0);
        let blk2 = Block::new(0, bytes, String::from("bar"), 0);
        let blk3 = Block::new(0, bytes, String::from("bar"), 0);

        assert_ne!(blk1.hash(), blk2.hash());
        assert_eq!(blk2.hash(), blk3.hash());
    }
}

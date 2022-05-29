pub use crate::utils::crypto::*;
use crate::Sha256Hash;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SignedBlock {
    pub block: Block,
    pub signatures: Vec<Signature>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Block {
    pub epoch: u64,              // epoch the block was proposed in
    pub hash: Sha256Hash,        // hash of the block
    pub parent_hash: Sha256Hash, // hash of the parent block
    pub data: Vec<u8>,           // some stringified version of a vec<Transaction>
    pub height: u64,             // metadata to make constructing chains easier
    pub nonce: u64,              // not sure what this is for? maybe helpful lol
}

impl Block {
    pub fn new(
        epoch: u64,
        parent_hash: Sha256Hash,
        data: Vec<u8>,
        height: u64,
        nonce: u64,
    ) -> Self {
        // create a Sha256 object
        let mut hasher = Sha256::new();

        // add block fields
        hasher.update(parent_hash.as_slice());
        hasher.update(epoch.to_ne_bytes().as_slice());
        hasher.update(&data);
        hasher.update(nonce.to_ne_bytes().as_slice());

        let result = hasher.finalize();
        let bytes: Sha256Hash = result
            .as_slice()
            .try_into()
            .expect("slice with incorrect length");

        Self {
            epoch,
            hash: bytes,
            parent_hash,
            data,
            height,
            nonce,
        }
    }

    pub fn generate_test_block(data: Vec<u8>) -> Block {
        let mut hasher = Sha256::new();
        hasher.update(b"hello world");
        let result = hasher.finalize();
        let bytes: Sha256Hash = result
            .as_slice()
            .try_into()
            .expect("slice with incorrect length");

        return Block::new(0, bytes, data, 0, 0);
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
        let blk1 = Block::new(0, bytes, String::from("foo").into_bytes(), 0, 0);
        let blk2 = Block::new(0, bytes, String::from("bar").into_bytes(), 0, 0);
        let blk3 = Block::new(0, bytes, String::from("bar").into_bytes(), 0, 0);

        assert_ne!(blk1.hash, blk2.hash);
        assert_eq!(blk2.hash, blk3.hash);
    }
}
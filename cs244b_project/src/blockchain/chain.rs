use crate::blockchain::block::{Block, SignedBlock};
use crate::utils::crypto::*;
use crate::Sha256Hash;
use serde::{Serialize, Deserialize};
use sha2::{Digest, Sha256};
use std::fmt;

// May not end up needing this trait, I did this in case we wanted to separate the type of chains stored locally
// from the type of chain that is ultimately pushed to a public blockchain and any other representations we may want.
pub trait Chain {
    fn new() -> Self;
    fn append_block(&mut self, block: Block, signatures: Vec<Signature>);
    fn fetch_block(&self, id: u64) -> Block;
    fn validate_block(block: &Block, parent_block: &Block) -> bool;
    fn finalize_block();
    fn head(&self) -> (&Block, &Vec<Signature>);
    fn length(&self) -> usize;
    fn copy_up_to_height(&self, height: u64) -> Self;
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LocalChain {
    pub blocks: Vec<SignedBlock>,
}

// Private helper functions
impl LocalChain {
    fn genesis(&mut self) {
        let mut hasher = Sha256::new();
        hasher.update("genesis");
        let result = hasher.finalize();
        let bytes: Sha256Hash = result
            .as_slice()
            .try_into()
            .expect("slice with incorrect length");

        // Create genesis block, and wrapper to store signatures (genesis doesn't need any)
        let genesis_block =
            Block::new(0, bytes, String::from("genesis payload").into_bytes(), 0, 0);
        let genesis_block_wrapper = SignedBlock {
            block: genesis_block,
            signatures: Vec::new(),
        };

        self.blocks.push(genesis_block_wrapper);
    }
}

impl Chain for LocalChain {
    fn new() -> Self {
        let mut chain = Self { blocks: vec![] };
        chain.genesis();
        return chain;
    }
    fn append_block(&mut self, block: Block, signatures: Vec<Signature>) {
        let signed_block = SignedBlock { block, signatures };
        self.blocks.push(signed_block);
    }
    fn fetch_block(&self, id: u64) -> Block {
        todo!()
    }
    fn validate_block(block: &Block, parent_block: &Block) -> bool {
        return if block.parent_hash != parent_block.hash {
            true
        } else {
            false
        };
    }
    fn finalize_block() {}
    fn head(&self) -> (&Block, &Vec<Signature>) {
        let SignedBlock { block, signatures } =
            &self.blocks.last().expect("Blockchain is empty...");
        (block, signatures)
    }
    fn length(&self) -> usize {
        return self.blocks.len();
    }
    fn copy_up_to_height(&self, height: u64) -> LocalChain {
        // +1 because slice end is exclusive
        let copy_idx = usize::try_from(height + 1).expect("could not cast u64 to usize");
        Self {
            blocks: self.blocks[..copy_idx].to_vec(),
        }
    }
}

impl fmt::Display for LocalChain {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let mut s = String::from("");

        for signed_block in self.blocks.iter() {
            s = format!("{}{}->", s, signed_block.block.epoch);
        }

        write!(f, "{}", s)
    }    
}

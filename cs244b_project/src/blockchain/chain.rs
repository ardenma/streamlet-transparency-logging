use crate::blockchain::block::Block;
use crate::Sha256Hash;
use sha2::{Digest, Sha256};

// May not end up needing this trait, I did this in case we wanted to separate the type of chains stored locally
// from the type of chain that is ultimately pushed to a public blockchain and any other representations we may want.
pub trait Chain {
    fn new() -> Self;
    fn genesis(&mut self) -> Block;
    fn append_block(&mut self, block: Block);
    fn fetch_block(&self, id: u64) -> Block;
    fn validate_block(block: &Block, parent_block: &Block) -> bool;
    fn finalize_block();
}
#[derive(Debug, Clone)]
pub struct LocalChain {
    pub blocks: Vec<Block>,
}

impl Chain for LocalChain {
    fn new() -> Self {
        Self { blocks: vec![] }
    }
    fn genesis(&mut self) -> Block {
        let mut hasher = Sha256::new();
        hasher.update("genesis");
        let result = hasher.finalize();
        let bytes: Sha256Hash = result
            .as_slice()
            .try_into()
            .expect("slice with incorrect length");

        let genesis_block = Block {
            epoch: 0,
            hash: bytes,
            parent_hash: bytes,
            data: String::from("genesis payload"),
            votes: vec![],
            nonce: 0,
        };
        self.blocks.push(genesis_block.clone());
        return genesis_block;
    }
    fn append_block(&mut self, block: Block) {
        self.blocks.push(block);
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
}

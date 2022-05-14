use chrono::Utc;
use crate::block::*;

// May not end up needing this trait, I did this in case we wanted to separate the type of chains stored locally
// from the type of chain that is ultimately pushed to a public blockchain and any other representations we may want.
pub trait Chain {
    fn new() -> Self;
    fn genesis(&mut self);
    fn append_block(&mut self, block: Block);
    fn fetch_block(&self, id: u64) -> Block;
    fn validate_block(block: &Block, parent_block: &Block) -> bool;
    fn finalize_block();
}
pub struct LocalChain {
    pub blocks: Vec<Block>,
}

impl Chain for LocalChain {
    fn new() -> Self {
        Self { blocks: vec![] }
    }
    fn genesis(&mut self) {
        let genesis_block = Block {
            timestamp: Utc::now().timestamp(),
            epoch: 0,
            hash: String::from("genesis hash"),
            parent_hash: String::from("n/a"),
            payload: String::from("genesis payload"),
            votes: vec![],
            nonce: 0,
        };
        self.blocks.push(genesis_block);
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
        } else { false }
    }
    fn finalize_block() {}
}
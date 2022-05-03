use block::*;
use tokio::*;
use chrono::*;


trait Chain {
    fn new() -> Self;
    fn genesis(&self);
    fn append(&self, block: Block) -> Self;
    fn fetch_block(&self, id: u64) -> Block;
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
            id: 0,
            timestamp: Utc::now().timestamp(),
            hash: String::from("foo"),
            prev_hash: String::from("nope"),
            payload: String::from("ip addresses: 1,2,3,4,5"),
        };
        self.blocks.push(genesis_block);
    }

    fn append(&mut self, block: Block) {
        todo!()
    }
    fn fetch_block(&self, id: u64) -> Block {
        todo!()
    }
}
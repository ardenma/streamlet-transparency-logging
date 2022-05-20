use crate::block::*;
use crate::chain::*;
use crate::chain;

pub struct Player {
    pub is_leader: bool,
    pub public_key: String,
    private_key: String,
    player_pool_size: i64,
}

impl Player {
    pub fn new(){}
    pub fn is_chain_valid(chain: LocalChain) -> bool {
        // this will be very trivial chain validation check so we don't have to start making this into notarizing/finalizing/etc...
        // not totally sure how involved to get with it at the moment
        for i in 0..chain.blocks.len() {
            if i == 0 {
                // genesis block can be ignored
                continue;
            }
            let first = chain.blocks.get(i - 1).expect("expected parent block");
            let second = chain.blocks.get(i).expect("expected block");
            if !<LocalChain as Chain>::validate_block(second, first) {
                return false;
            }
        }
        true
    }
    pub fn finalize(){}
    pub fn vote(self, mut block: Block) {
        block.votes.push(self.public_key)
    }
}
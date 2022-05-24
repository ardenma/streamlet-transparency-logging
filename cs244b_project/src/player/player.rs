use crate::blockchain::*;
use crate::message::*;
use crate::Sha256Hash;

pub struct Player {
    pub is_leader: bool,
    pub public_key: String,
    private_key: String,
    player_pool_size: i64,
    viewed_chains: Vec<LocalChain>,
    notarized_chains: Vec<LocalChain>, // i *think* we only track a certain number of these... not sure how to determine this exactly
    longest_notarized_chain: LocalChain,
    unconfirmed_pending_transactions: String, // we should formulate some actual Transaction struct but I'm not sure what that looks like
    finalized_log: LocalChain,
}

impl Player {
    // I think this is the sorta thing that might depend on some networking details so I'll defer on implementing this
    // I suggest that the default longest notarized chain just be an empty list value so that it's easy to overwrite
    pub fn new(){ todo!() }
    pub fn observe_chain(&mut self, chain: LocalChain) {
        self.notarized_chains.push(chain)
    }
    pub fn is_chain_valid(chain: LocalChain) -> bool {
        // this will be very trivial chain validation check so we don't have to start making this into notarizing/finalizing/etc...
        // not totally sure how involved to get with it at the moment
        for i in 0..chain.blocks.len() {
            if i == 0 {
                // genesis block can be ignored
                continue;
            }
            let first: &Block = chain.blocks.get(i - 1).expect("expected parent block");
            let second: &Block = chain.blocks.get(i).expect("expected block");
            if !<LocalChain as Chain>::validate_block(second, first) {
                return false;
            }
        }
        true
    }
    pub fn propose(self, epoch: i64, nonce: u64, parent_hash: Sha256Hash) -> Block {
        let mut hasher = Sha256::new();
        // this hashing chunk is used a lot, if this is its final-form we should abstract it out
        hasher.update(self.unconfirmed_pending_transactions.clone());
        let result = hasher.finalize();
        let bytes: Sha256Hash = result.as_slice().try_into().expect("slice with incorrect length");
        Block {
            epoch,
            hash: bytes,
            parent_hash,
            data: self.unconfirmed_pending_transactions.clone(),
            votes: vec![],
            nonce
        }
    }
    pub fn vote(&self, mut block: Block) -> Message {
        // Create a message
        let public_key_copy = self.public_key.clone();
        let message = Message {
            payload: MessagePayload::Block(block),
            kind: MessageKind::Vote,
            nonce: 0,
            signatures: Vec::new(),
        };
        // there needs to be a check to see if this extends a longest notarized chain which is not a hard check but i'm not sure it should go here...
        // it's just a matter of seeing if the parent hash referenced is extended from the last block on any in the notarized list
        block.votes.push(public_key_copy.clone()); // I couldn't tell from the paper if you append your own vote to the block before broadcast
        return message;
    }
    pub fn is_notarized_block(&self, block: &Block) -> bool {
        return block.votes.len().unwrap() >= self.player_pool_size / 2;
    }
    pub fn notarize_chain(&mut self, chain: LocalChain) -> bool {
        for i in 0..chain.blocks.len() {
            if i == 0 {
                // genesis block can be ignored
                continue;
            }
            let block: &Block = chain.blocks.get(i).expect("expected block for notarization check");
            if !self.is_notarized_block(block) { return false; }
        }
        self.notarized_chains.push(chain);
        if chain.blocks.len().unwrap() >= self.longest_notarized_chain.blocks.len() { self.longest_notarized_chain = chain.clone() }
        return true;
    }
    pub fn finalize(&mut self, notarized_chain: LocalChain) {
        // Check if the last 3 consecutive notarized blocks have sequential epochs and if so, commit the first two to finalized log
        // This may need to be generalized to be an overall loop; can change it then if needed just wasn't sure why it would have to be this way
        if notarized_chain.blocks.len().unwrap() < 4 { return; }
        let i = notarized_chain.blocks.len();
        let newest: &Block = chain.blocks.get(i - 1).expect("expected recent block");
        let commit_2: &Block = chain.blocks.get(i - 2).expect("expected latter notarized block");
        let commit_1: &Block = chain.blocks.get(i - 3).expect("expected former notarized block");
        if newest.epoch == commit_2.epoch + 1 && commit_2.epoch == commit_1.epoch + 1 {
            let last_finalized_block =  self.finalized_log.blocks.last().unwrap_or(&Block::test_block(&String::from("finalization test block")));
            if last_finalized_block.epoch < commit_1.epoch {
                self.finalized_log.append_block(*commit_1.clone());
                self.finalized_log.append_block(*commit_2.clone());
            } else if last_finalized_block.epoch < commit_2.epoch {
                self.finalized_log.append_block(*commit_2.clone());
            }
        }

    }
    pub fn export_local_chain(&self) {
        // not totally sure on how to specify the where to export to functionality so for now this just pretty-prints the finalized chain representing the log
        println!("{:?}", self.finalized_log);
    }
}
use crate::blockchain::*;
use log::info;

// Struct for managing multiple notarized chains and a finalied chain.
// Provides the abstraction of a single Chain the user can query/manipulate
// Responsbility for verifying that a block is notarized falls upon code which
// uses this struct
pub struct BlockchainManager {
    pub finalized_chain_length: usize,
    pub finalized_chain: LocalChain,
    pub longest_notarized_chain_length: usize,  // length = max_height + 1
    notarized_chains: Vec<LocalChain>, // i *think* we only track a certain number of these... not sure how to determine this exactly
    // longest_notarized_chain: LocalChain,
    // unconfirmed_pending_transactions: String, // we should formulate some actual Transaction struct but I'm not sure what that looks like
}

impl BlockchainManager {
    // I think this is the sorta thing that might depend on some networking details so I'll defer on implementing this
    // I suggest that the default longest notarized chain just be an empty list value so that it's easy to overwrite
  
    /* Creates a new BlockchainManager instance. */
    pub fn new() -> Self {
        Self { 
            finalized_chain_length: 1, 
            finalized_chain: LocalChain::new(), 
            longest_notarized_chain_length: 1, 
            notarized_chains: Vec::from([LocalChain::new()])
        }            
    }

    /* Adds a chain to vector of notarized chains
        @param chain: notarized chain that was observed */ 
    pub fn observe_chain(&mut self, chain: LocalChain) {
        self.notarized_chains.push(chain)
    }

    /* Validates a chain by checking the hash chain.
        @param chain: chain to be validated */   
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
 
    /* Tries to adds block to a notarized chain and tries to finalize the 
       chain (TODO make more efficient)
        @param notarized_block: notarized_block to add */   
    pub fn add_to_chain(&mut self, notarized_block: Block) -> bool{
        let mut chain_idx = 0;
        let mut success = false;

        // Try to add notarized block to one of the notarized chains
        for chain in self.notarized_chains.iter_mut() {
            if chain.head().hash == notarized_block.parent_hash {
                info!("Added notarized block with epoch: {}, nonce: {}, parent hash: {:?}, hash: {:?}",
                      notarized_block.epoch, notarized_block.nonce, notarized_block.parent_hash, notarized_block.hash);
                chain.append_block(notarized_block);
                // Update longest notarized chain length if needed
                if chain.length() > self.longest_notarized_chain_length {
                    info!("New longest notarized chain length: {}", self.longest_notarized_chain_length);
                    self.longest_notarized_chain_length = chain.length();
                }
                success = true;
                break;
            }
            chain_idx += 1;
        }

        // TODO: make more efficient, e.g. keep a record of last n (3 or 6) blocks
        // and only try to finalize if they all are notaraized on the same chain
        if success {
            self.try_finalize(chain_idx);
            return true;
        }

        return false;
    }

    /* Tries to finalize the notarized chain given by notarized_chain_idx.
       If finalization succeeds, updates the finalized chain (currently by copying
       it, TODO make more efficient)
        @param notarized_chain_idx: index of the notarized chain (in the vec) */ 
    fn try_finalize(&mut self, notarized_chain_idx: usize) {
        // Check if the last 3 consecutive notarized blocks have sequential epochs and if so, commit the first two to finalized log
        // This may need to be generalized to be an overall loop; can change it then if needed just wasn't sure why it would have to be this way
        let notarized_chain = &self.notarized_chains[notarized_chain_idx];
        if notarized_chain.blocks.len() < 4 {
            return;
        }
        let i = notarized_chain.blocks.len();
        let newest: &Block = notarized_chain
            .blocks
            .get(i - 1)
            .expect("expected recent block");
        let commit_2: &Block = notarized_chain
            .blocks
            .get(i - 2)
            .expect("expected latter notarized block");
        let commit_1: &Block = notarized_chain
            .blocks
            .get(i - 3)
            .expect("expected former notarized block");
        if newest.epoch == commit_2.epoch + 1 && commit_2.epoch == commit_1.epoch + 1 {
            // let last_finalized_block = match self.finalized_chain.blocks.last() {
            //     Some(b) => b.clone(),
            //     None => Block::generate_test_block(&String::from("finalization test block")),
            // };

            // if last_finalized_block.epoch < commit_1.epoch {
            //     self.finalized_chain.append_block(commit_1.clone());
            //     self.finalized_chain.append_block(commit_2.clone());
            // } else if last_finalized_block.epoch < commit_2.epoch {
            //     self.finalized_chain.append_block(commit_2.clone());
            // }
            
            // Since chain is implicitly finalized up to commit_2
            // TODO make more efficient, e.g. only add missing blocks
            // TODO fix to check 6 commits
            self.finalized_chain = notarized_chain.copy_up_to_height(commit_2.height);
            self.finalized_chain_length = self.finalized_chain.length();
            info!("Successfully finalized chain up to: {}", self.finalized_chain_length);
        }
    }

    pub fn export_local_chain(&self) {
        // not totally sure on how to specify the where to export to functionality so for now this just pretty-prints the finalized chain representing the log
        println!("{:?}", self.finalized_chain);
        todo!()
    }

    pub fn head(&mut self) -> &Block{
        // Garbage collect
        self.cleanup_notarized_chains();
        return self.notarized_chains[0].head();
    }

    /* Garbage collect all notarized chains which are no longer a "longest notarized chain" */
    fn cleanup_notarized_chains(&mut self) {
        self.notarized_chains.retain(|x| x.length() == self.longest_notarized_chain_length);
    }
}

use crate::blockchain::*;
use log::info;
use std::fs;
use std::fs::OpenOptions;
use std::env;
use std::io::{Read, Write};

// Struct for managing multiple notarized chains and a finalied chain.
// Provides the abstraction of a single Chain the user can query/manipulate
// Responsbility for verifying that a block is notarized falls upon code which
// uses this struct
pub struct BlockchainManager {
    pub finalized_chain_length: usize,
    pub finalized_chain: LocalChain,
    pub longest_notarized_chain_length: usize, // length = max_height + 1
    notarized_chains: Vec<LocalChain>, 
    pub last_logged_epoch: u64,
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
            notarized_chains: Vec::from([LocalChain::new()]),
            last_logged_epoch: 0,
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
        for i in 0..chain.length() {
            if i == 0 {
                // genesis block can be ignored
                continue;
            }
            let SignedBlock { block: first, .. } =
                chain.blocks.get(i - 1).expect("expected parent block");
            let SignedBlock { block: second, .. } = chain.blocks.get(i).expect("expected block");
            if !<LocalChain as Chain>::validate_block(second, first) {
                return false;
            }
        }
        true
    }

    pub fn index_of_ancestor_chain(&mut self, new_block: Block) -> Option<usize> {
        let mut chain_idx = 0;
        for chain in self.notarized_chains.iter_mut() {
            if chain.length() < self.longest_notarized_chain_length { continue; }
            let (block, _) = chain.head();
            if block.hash == new_block.parent_hash { return Some(chain_idx) }
            chain_idx += 1;
        }
        return None;
    }

    /* Tries to adds block to a notarized chain and tries to finalize the chain
     @param notarized_block: notarized_block to add */
    pub fn add_to_chain(&mut self, notarized_block: Block, signatures: Vec<Signature>, chain_index: usize) {
        let notarized_chain_opt = self.notarized_chains.get_mut(chain_index);         
        match notarized_chain_opt {
            Some(notarized_chain) => {
                // fail-fast checks to protect against public function shenanigans
                if !(notarized_chain.head().0.hash == notarized_block.parent_hash) { panic!("Block was supposed to be a descendent of the given chain, but was not.") };
                // if we make stuff more private we should confirm here that the block actually is notarized though this is more of a local issue so 
                // not crazy important

                notarized_chain.append_block(notarized_block.clone(), signatures);
                info!("\n\nAdded notarized block with epoch: {}, \nnonce: {}, \nparent hash: {:?}, \nhash: {:?}\nNew chain: {}\n",
                      notarized_block.epoch, notarized_block.nonce, String::from_utf8_lossy(&notarized_block.parent_hash[..]), String::from_utf8_lossy(&notarized_block.hash[..]), notarized_chain);
                if notarized_chain.length() > self.longest_notarized_chain_length {
                    self.longest_notarized_chain_length = notarized_chain.length();
                    info!(
                        "New longest notarized chain length: {}",
                        self.longest_notarized_chain_length
                    );
                }
                self.notarized_chains.sort_by(|a, b| b.length().cmp(&a.length()));
                self.try_finalize(chain_index);
            }
            None => {}
        }
    }

    /* Tries to finalize the notarized chain given by notarized_chain_idx.
        If finalization succeeds, updates the finalized chain .
     @param notarized_chain_idx: index of the notarized chain (in the vec) */
    fn try_finalize(&mut self, notarized_chain_idx: usize) {
        // Check if the last 3 consecutive notarized blocks have sequential epochs and if so, commit the first two to finalized log
        // This may need to be generalized to be an overall loop; can change it then if needed just wasn't sure why it would have to be this way
        let notarized_chain = &self.notarized_chains[notarized_chain_idx];
        // Require 3 blocks
        if notarized_chain.length() < 3 {
            return;
        }
        let i = notarized_chain.length();
        // Newest block 
        let SignedBlock { block: newest, .. } = notarized_chain
            .blocks
            .get(i - 1)
            .expect("expected recent block");
        // Second-newest block 
        let SignedBlock {
            block: commit_2, ..
        } = notarized_chain
            .blocks
            .get(i - 2)
            .expect("expected latter notarized block");
        // Third-newest block
        let SignedBlock {
            block: commit_1, ..
        } = notarized_chain
            .blocks
            .get(i - 3)
            .expect("expected former notarized block");

        if newest.epoch == commit_2.epoch + 1 
            && commit_2.epoch == commit_1.epoch + 1 {
            self.finalized_chain = notarized_chain.copy_up_to_height(commit_2.height);
            self.finalized_chain_length = self.finalized_chain.length();
            info!(
                "\n\nSuccessfully finalized chain, new finalized chain {}\n",
                self.finalized_chain
            );
        }
    }

    pub fn fetch_chain_after_epoch(&mut self, epoch: u64) -> Vec<SignedBlock> {
        let chain = self.finalized_chain.clone().blocks;
        let chain = chain
            .into_iter()
            .filter(|signed_block| (signed_block.block.epoch > epoch) || (self.last_logged_epoch == 0))
            .collect();
        chain
    }
    pub fn fetch_local_finalized_chain(&self) -> LocalChain { self.finalized_chain.clone() }
    pub fn export_local_finalized_chain_to_file(&mut self, local_file_path: String) {
        let last_epoch = self.last_logged_epoch;
        info!("exporting local finalized chain to: {}", local_file_path);
        let mut file = OpenOptions::new()
            .append(true)
            .create(true)
            .open(local_file_path)
            .unwrap();
        let unlogged_chain = serde_json::to_string_pretty(&self.fetch_chain_after_epoch(last_epoch)).unwrap();
        file.write_all(unlogged_chain.as_bytes()).unwrap();
        self.last_logged_epoch = self.get_latest_finalized_block().0.epoch;
    }
    pub fn publish_last_finalized_block(&self) {
        info!("publishing most recent finalized block to public chain");
        // Hard-coding the public path here but if we want to get fancy with it we could have this be determined by the app or have it be specifiable. Just trying to make checking easy
        let public_path = format!("{}/src/tmp/{}.txt", env::current_dir().expect("invalid current directory").display().to_string(), "pub");
        let mut file = OpenOptions::new()
            .append(true)
            .create(true)
            .open(public_path)
            .unwrap();
        let latest_finalized_block_msg = serde_json::to_string_pretty(&self.get_latest_finalized_block()).unwrap();
        file.write_all(latest_finalized_block_msg.as_bytes()).unwrap();
        
    }

    /* Returns the most recent notarized block on one of the longest notarized
    chains. */
    pub fn head(&mut self) -> (&Block, &Vec<Signature>) {
        // Garbage collect
        self.cleanup_notarized_chains();
        return self.notarized_chains[0].head();
    }
    /* Returns a copy of the most recent finalized block. */
    pub fn get_latest_finalized_block(&self) -> (&Block, &Vec<Signature>) {
        let SignedBlock { block, signatures } = &self
            .finalized_chain
            .blocks
            .last()
            .expect("finalized chain is empty...");
        (block, signatures)
    }

    /* Garbage collect all notarized chains which are no longer a "longest notarized chain" */
    fn cleanup_notarized_chains(&mut self) {
        self.notarized_chains
            .retain(|x| x.length() == self.longest_notarized_chain_length);
    }

    pub fn print_notarized_chains(&self) {
        println!("************************ PRINTING NOTARIZED CHAINS **********************");
        for chain in self.notarized_chains.iter() {
            println!("{}", chain);
        }
        println!("*************************************************************************");
    }

    pub fn print_finalized_chains(&self) {
        println!("************************ PRINTING FINALIZED CHAIN **********************");
        println!("{:?}", self.finalized_chain);
        println!("*************************************************************************");
    }

}

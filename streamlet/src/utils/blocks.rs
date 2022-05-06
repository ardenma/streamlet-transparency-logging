use sha2::{Sha256, Digest};

use crate::utils::Sha256Hash;

#[derive(Debug)]
pub struct Block {
    pub hash: Sha256Hash,
    pub data: String,
}

impl Block {
    pub fn hash(&self) -> Sha256Hash {
        // create a Sha256 object
        let mut hasher = Sha256::new();

        // add block fields
        hasher.update(self.hash.as_slice());
        hasher.update(self.data.as_bytes());

        // read hash digest and consume hasher
        let result = hasher.finalize();
        let bytes = result.as_slice().try_into().expect("slice with incorrect length");
        return bytes;
    }
}



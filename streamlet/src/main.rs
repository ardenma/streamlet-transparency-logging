mod utils;

use utils::{Block, Vote, Sha256Hash};
use sha2::{Sha256, Digest};


fn main() {
    // create a Sha256 object
    let mut hasher = Sha256::new();

    // write input message
    hasher.update(b"hello world");

    // read hash digest and consume hasher
    let result = hasher.finalize();
    let bytes: Sha256Hash = result.as_slice().try_into().expect("slice with incorrect length");

    let blk1 = Block {
        hash: bytes,
        data: String::from("foo"),
    };
    let blk2 = Block {
        hash: bytes,
        data: String::from("bar"),
    };
    let blk3 = Block {
        hash: bytes,
        data: String::from("bar"),
    };


    println!("{:?}", blk1.hash());
    println!("{:?}", blk2.hash());
    println!("{:?}", blk1.hash() == blk2.hash());
    println!("{:?}", blk2.hash() == blk3.hash());
}

mod utils;

use utils::{Block, Vote, Sha256Hash};
use sha2::{Sha256, Digest};
use rand::rngs::OsRng;
use ed25519_dalek::{Keypair, PublicKey, Signature, Signer, Verifier};

struct StreamletInstance {
    keypair: Keypair,
}
impl StreamletInstance {
    pub fn get_public_key(&self) -> PublicKey {
        return self.keypair.public;
    }
    pub fn sign(&self, message: &[u8]) -> Signature {
        return self.keypair.sign(message);
    }
}


fn create_streamlet_instance() -> StreamletInstance {
    // Setup public/private key pair
    let mut csprng = OsRng{};
    let keypair: Keypair = Keypair::generate(&mut csprng);

    return StreamletInstance {keypair: keypair};
}

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

    let streamlet: StreamletInstance = create_streamlet_instance();

    let message: &[u8] = b"This is a test of the tsunami alert system.";
    let signature: Signature = streamlet.sign(message);
    let public_key: PublicKey = streamlet.get_public_key();
    assert!(public_key.verify(message, &signature).is_ok());
}

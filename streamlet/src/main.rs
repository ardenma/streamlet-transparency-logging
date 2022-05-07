mod utils;

use std::collections::VecDeque;

use sha2::{Sha256, Digest};
use rand::rngs::OsRng;
use ed25519_dalek::{Keypair, PublicKey, Signature, Signer, Verifier};

use utils::{Block, Message, MessageHeader, MessagePayload, Sha256Hash};

struct StreamletInstance {
    pub id: u32,
    pub inbound_messages: VecDeque<Message>,
    pub outbound_messages: VecDeque<Message>,
    keypair: Keypair,

}

impl StreamletInstance {
    pub fn get_public_key(&self) -> PublicKey {
        return self.keypair.public;
    }
    pub fn sign(&self, message: &[u8]) -> Signature {
        return self.keypair.sign(message);
    }
    pub fn send(&mut self, message: &mut Message, add_signature: bool) {
        if (add_signature) {
            let signature: Signature = self.keypair.sign(message.serialize_payload().as_slice());
            message.signatures.push(signature)
        }
        self.outbound_messages.push_back(message.clone());
    }
    pub fn recv(&mut self) -> Option<Message> {
        return self.inbound_messages.pop_front();
    }
}

fn create_streamlet_instance(id: u32) -> StreamletInstance {
    // Setup public/private key pair
    let mut csprng = OsRng{};
    let keypair: Keypair = Keypair::generate(&mut csprng);

    // Encode genesis block (probably should throw this in the chain instead)
    let mut hasher = Sha256::new();
    hasher.update(b"cs244b rocks!");
    let result = hasher.finalize();
    let bytes: Sha256Hash = result.as_slice().try_into().expect("slice with incorrect length");
    let genesis = Block {
        hash: bytes,
        data: String::from("this is the genesis block."),
    };

    // Build the streamlet instance
    let streamlet = StreamletInstance {
        id: id,
        keypair: keypair, 
        inbound_messages: VecDeque::new(),
        outbound_messages: VecDeque::new(),
    };

    return streamlet;
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

    let streamlet: StreamletInstance = create_streamlet_instance(0);

    // Testing signatures
    let message: &[u8] = b"This is a test of the tsunami alert system.";
    let signature: Signature = streamlet.sign(message);
    let public_key: PublicKey = streamlet.get_public_key();
    assert!(public_key.verify(message, &signature).is_ok());

    // Testing serdes
    let payload = MessagePayload {
        parent_hash: bytes,
        epoch: 0,
        payload_string: String::from("test"),
    };
    let encoded = payload.serialize();
    let decoded = MessagePayload::deserialize(&encoded);
    assert_eq!(payload, decoded);
}

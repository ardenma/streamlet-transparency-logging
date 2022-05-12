mod blockchain;
mod messages;
mod utils;

use std::collections::VecDeque;
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

pub use utils::crypto::*;
pub use blockchain::Block;
pub use messages::{Message, MessageHeader, MessagePayload};

pub struct StreamletInstance {
    pub id: u32,
    pub inbound_messages: VecDeque<Message>,
    pub outbound_messages: VecDeque<Message>,
    num_instances: u32,
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
        if add_signature {
            let signature: Signature = self.keypair.sign(message.serialize_payload().as_slice());
            message.signatures.push(signature)
        }
        self.outbound_messages.push_back(message.clone());
    }
    pub fn recv(&mut self) -> Option<Message> {
        return self.inbound_messages.pop_front();
    }
    pub fn get_epoch_leader(&self, epoch: u32) -> u32 {
        let mut hasher = DefaultHasher::new();
        hasher.write_u32(epoch);
        let result = hasher.finish() as u32;
        return result % self.num_instances;  // Assumes 0 indexing!
    }
}

pub fn create_streamlet_instance(id: u32) -> StreamletInstance {
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
        num_instances: 1,
        inbound_messages: VecDeque::new(),
        outbound_messages: VecDeque::new(),
    };

    return streamlet;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_streamlet_signatures() {
        let streamlet: StreamletInstance = create_streamlet_instance(0);

        // Testing signatures
        let message: &[u8] = b"This is a test of the tsunami alert system.";
        let signature: Signature = streamlet.sign(message);
        let public_key: PublicKey = streamlet.get_public_key();
        assert!(public_key.verify(message, &signature).is_ok());
    }
}
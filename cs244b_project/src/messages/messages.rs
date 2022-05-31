use bincode::{deserialize, serialize};
use rand::Rng;
use serde::{Deserialize, Serialize};
use std::vec::Vec;

use crate::blockchain::Block;
use crate::network::peer_init::PeerAdvertisement;
use crate::utils::crypto::*;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Message {
    pub payload: MessagePayload,
    pub kind: MessageKind,
    pub nonce: u32,
    pub tag: u32,
    pub sender_id: u32,
    pub sender_name: String,
    signatures: Vec<Signature>,
}

impl Message {
    pub fn new(payload: MessagePayload, kind: MessageKind, sender_id: u32, sender_name: String) -> Message {
        Message { 
            payload, 
            kind, 
            nonce: rand::thread_rng().gen(), 
            tag: rand::thread_rng().gen(),
            sender_id,
            sender_name, 
            signatures: Vec::new() 
        }
    }
    // Grr... no overloading in Rust... eventually `nonce` should be private. Security-wise it doesn't make sense this way
    pub fn new_with_defined_nonce(payload: MessagePayload, kind: MessageKind, nonce: u32, sender_id: u32, sender_name: String) -> Message {
        Message { 
            payload, 
            kind, 
            nonce,
            tag: rand::thread_rng().gen(),
            sender_id,
            sender_name, 
            signatures: Vec::new() 
        }
    }
    pub fn new_with_defined_tag(payload: MessagePayload, kind: MessageKind, tag: u32, sender_id: u32, sender_name: String) -> Message {
        Message { 
            payload, 
            kind, 
            nonce: rand::thread_rng().gen(),
            tag,
            sender_id,
            sender_name, 
            signatures: Vec::new() 
        }
    }
    // Used to sign the message payload (block)
    pub fn serialize_payload(&self) -> Vec<u8> {
        return self.payload.serialize();
    }
    pub fn serialize(&self) -> Vec<u8> {
        let encoded: Vec<u8> = serialize(self).expect("Failed serialization.");
        return encoded;
    }
    pub fn deserialize(encoded: &Vec<u8>) -> Message {
        let decoded: Message = deserialize(&encoded[..]).expect("Failed deserialization.");
        return decoded;
    }
    // Access functions for message signatures to avoid storing entire Siganture vector copies
    pub fn get_signatures(self) -> Vec<Signature> { self.signatures } 
    pub fn sign_message(&mut self, signature: Signature) {
        self.signatures.push(signature)
    }
    pub fn signature_count(&self) -> usize { self.signatures.len() }
}

// Wrapper for different kinds of messages (currently only blocks are supported)
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum MessagePayload {
    Block(Block),
    String(String),
    PeerAdvertisement(PeerAdvertisement),
    AppData(Vec<u8>),
    None,
}

// Useful for serializing the payload (block) so we can sign it
impl MessagePayload {
    pub fn serialize(&self) -> Vec<u8> {
        let encoded: Vec<u8> = serialize(self).unwrap();
        return encoded;
    }
    pub fn deserialize(encoded: &Vec<u8>) -> MessagePayload {
        let decoded: MessagePayload = deserialize(&encoded[..]).unwrap();
        return decoded;
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum MessageKind {
    Vote,
    Propose,
    Test,
    PeerInit,
    // Application-Streamlet config
    AppRequest,
    AppSend,
    AppBlockRequest,
    AppBlockResponse,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_message_serdes() {
        // Create random hash
        let mut hasher = Sha256::new();
        hasher.update(b"hello world");
        let result = hasher.finalize();
        let bytes: Sha256Hash = result
            .as_slice()
            .try_into()
            .expect("slice with incorrect length");

        // Create a test block
        let blk = Block::new(0, bytes, String::from("test").into_bytes(), 0, 0);

        // Create a message
        let message = Message::new(
            MessagePayload::Block(blk),
            MessageKind::Vote,
            0,
            String::from("test"),
        );

        // Serdes
        let serialized_message = message.serialize();
        let deserialized_message = Message::deserialize(&serialized_message);

        assert_eq!(message, deserialized_message);
    }
}

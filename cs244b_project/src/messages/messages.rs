use std::vec::Vec;
use bincode::{serialize, deserialize};
use serde::{Serialize, Deserialize};

use crate::utils::crypto::*;
use crate::blockchain::Block;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Message {
    pub payload: MessagePayload,
    pub kind: MessageKind,
    pub signatures: Option<Vec<Signature>>,
}

impl Message {
    // Used to sign the message payload (block)
    pub fn serialize_payload(&self) -> Vec<u8> {
        return self.payload.serialize();
    }
    pub fn serialize(&self) ->  Vec<u8> {
        let encoded: Vec<u8> = serialize(self).unwrap();  // TODO handle errors?
        return encoded;
    }
    pub fn deserialize(encoded: &Vec<u8>) -> Message {
        let decoded: Message = deserialize(&encoded[..]).unwrap();  // TODO handle errors?
        return decoded;
    }
}

// Wrapper for different kinds of messages (currently only blocks are supported)
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum MessagePayload {
    Block(Block),    
}

// Useful for serializing the payload (block) so we can sign it
impl MessagePayload {
    pub fn serialize(&self) ->  Vec<u8> {
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
        let bytes: Sha256Hash = result.as_slice().try_into().expect("slice with incorrect length");
        
        // Create a test block
        let blk = Block {
            parent_hash: bytes,
            epoch: 0,
            data: String::from("test"),
        };

        // Create a message
        let payload = MessagePayload::Block(blk); 
        let message = Message {
            payload: payload.clone(),
            kind: MessageKind::Vote,
            signatures: Some(Vec::new()),
        };

        // Serdes
        let serialized_message = message.serialize();
        let deserialized_message = Message::deserialize(&serialized_message);
       
        assert_eq!(message, deserialized_message);
    }
}
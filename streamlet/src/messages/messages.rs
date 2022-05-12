use std::vec::Vec;
use bincode::{serialize, deserialize};
use serde::{Serialize, Deserialize};

use crate::utils::crypto::*;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Message {
    pub header: MessageHeader,
    pub payload: MessagePayload,
    pub signatures: Vec<Signature>,
    pub kind: MessageKind,
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

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MessageHeader {
    pub destination: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MessagePayload {
    pub parent_hash: Sha256Hash,
    pub epoch: u32,
    pub payload_string: String,
}

// Useful for serializing the payload (blocK) so we can sign it
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

        // Create a message
        let payload = MessagePayload {
            parent_hash: bytes,
            epoch: 0,
            payload_string: String::from("test"),
        };
        let header = MessageHeader {
            destination: String::from("test destination"),
        };
        let kind = MessageKind::Vote;
        let message = Message {
            header: header.clone(),
            payload: payload.clone(),
            kind: kind,
            signatures: Vec::new(),
        };

        // Serdes
        let serialized_message = message.serialize();
        let deserialized_message = Message::deserialize(&serialized_message);
       
        assert_eq!(message, deserialized_message);
    }
}
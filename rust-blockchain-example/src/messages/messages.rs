use std::vec::Vec;
use bincode::{serialize, deserialize};
use serde::{Serialize, Deserialize};

use crate::utils::crypto::*;

#[derive(Clone)]
pub struct Message {
    pub header: MessageHeader,
    pub payload: MessagePayload,
    pub signatures: Vec<Signature>,
    pub kind: MessageKind,
}

impl Message {
    pub fn serialize_payload(&self) -> Vec<u8> {
        return self.payload.serialize();
    }
}

#[derive(Clone)]
pub struct MessageHeader {
    pub destination: String,
}

#[derive(Debug, PartialEq, Serialize, Deserialize, Clone)]
pub struct MessagePayload {
    pub parent_hash: Sha256Hash,
    pub epoch: u32,
    pub payload_string: String,
}

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

#[derive(Copy, Clone)]
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
        let encoded_payload = message.serialize_payload();
        let decoded_payload = MessagePayload::deserialize(&encoded_payload);
       
        assert_eq!(payload, decoded_payload);
    }
}
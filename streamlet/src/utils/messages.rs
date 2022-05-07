use std::vec::Vec;
use bincode::{serialize, deserialize};
use serde::{Serialize, Deserialize};
use ed25519_dalek::Signature;
use crate::utils::Sha256Hash;

#[derive(Clone)]
pub struct Message {
    pub header: MessageHeader,
    pub payload: MessagePayload,
    pub signatures: Vec<Signature>,
    pub kind: MessageType,
}

impl Message {
    pub fn serialize_payload(&self) -> Vec<u8> {
        return self.payload.serialize();
    }
}

#[derive(Copy, Clone)]
pub struct MessageHeader {
    pub destination: u32,
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
pub enum MessageType {
    Vote,
    Propose,
}

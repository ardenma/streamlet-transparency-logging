use hex;

pub struct Block {
    pub timestamp: i64,
    pub epoch: i64,
    pub hash: String,
    pub parent_hash: String,
    pub payload: String, // some stringified version of a vec<Transaction>
    pub votes: Vec<String>,
    pub nonce: u64,
}
impl Block {
    pub fn new(timestamp: i64, epoch: i64, parent_hash: String, payload: &String, nonce: u64) -> Self {
        let payload_string = String::from(payload);
        let hash = hex::encode(payload_string.clone());
        Self {
            timestamp,
            epoch,
            hash,
            parent_hash,
            payload: payload_string,
            votes: vec![],
            nonce,
        }
    }
}




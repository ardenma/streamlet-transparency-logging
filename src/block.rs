
pub struct Block {
    pub id: u64,
    pub timestamp: i64,
    pub hash: String,
    pub prev_hash: String,
    pub payload: String,
}
impl Block {
    fn new(id: u64, timestamp: i64, hash: String, prev_hash: String, payload: String) -> Self {
        Self {
            id,
            timestamp,
            hash,
            prev_hash,
            payload,
        }
    }

    fn fetch_previous_block(&self) -> Block {
        todo!()
    }

}




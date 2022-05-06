use crate::utils::Sha256Hash;

pub struct Vote {
    parent_hash: Sha256Hash,
    epoch: i32,
    payload_string: String,
}
pub struct Propose {
    parent_hash: Sha256Hash,
    epoch: i32,
    payload_string: String,
}
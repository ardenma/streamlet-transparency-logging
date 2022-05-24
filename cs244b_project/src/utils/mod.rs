pub mod crypto {
    pub use ed25519_dalek::{Keypair, PublicKey, Signature, Signer, Verifier};
    pub use sha2::{Sha256, Digest};
    pub use rand::rngs::OsRng;
    
    pub type Sha256Hash = [u8; 32];
}
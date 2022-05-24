pub mod crypto {
    pub use ed25519_dalek::{Keypair, PublicKey, Signature, Signer, Verifier};
    pub use rand::rngs::OsRng;
    pub use sha2::{Digest, Sha256};

    pub type Sha256Hash = [u8; 32];
}

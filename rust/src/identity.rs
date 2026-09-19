//! Device Identity & Key Management
//! Generates and persists Ed25519 device keys and human-readable names.

use ed25519_dalek::{SigningKey, VerifyingKey};
use rand::rngs::OsRng;
use sha2::{Digest, Sha256};

pub struct DeviceIdentity {
    signing_key: SigningKey,
    verifying_key: VerifyingKey,
}

impl DeviceIdentity {
    pub fn generate() -> Self {
        let mut csprng = OsRng;
        let signing_key = SigningKey::generate(&mut csprng);
        let verifying_key = signing_key.verifying_key();
        Self {
            signing_key,
            verifying_key,
        }
    }

    pub fn from_bytes(bytes: &[u8; 32]) -> Self {
        let signing_key = SigningKey::from_bytes(bytes);
        let verifying_key = signing_key.verifying_key();
        Self {
            signing_key,
            verifying_key,
        }
    }

    pub fn public_key_bytes(&self) -> [u8; 32] {
        self.verifying_key.to_bytes()
    }

    pub fn public_key_hex(&self) -> String {
        hex::encode(self.public_key_bytes())
    }

    /// Derives a clean 6-digit numeric device ID from the public key SHA-256 hash.
    pub fn device_id(&self) -> String {
        let mut hasher = Sha256::new();
        hasher.update(self.verifying_key.as_bytes());
        let hash = hasher.finalize();

        let num = u32::from_be_bytes([hash[0], hash[1], hash[2], hash[3]]);
        let six_digits = (num % 900_000) + 100_000;
        format!("{:06}", six_digits)
    }

    pub fn sign(&self, message: &[u8]) -> [u8; 64] {
        use ed25519_dalek::Signer;
        self.signing_key.sign(message).to_bytes()
    }
}

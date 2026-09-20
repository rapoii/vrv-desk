//! Secure Transport
//! Encrypted framing (ChaCha20-Poly1305 AEAD), nonce sequencing, and anti-tamper authentication.

use chacha20poly1305::aead::{Aead, KeyInit};
use chacha20poly1305::{ChaCha20Poly1305, Key, Nonce};
use sha2::{Digest, Sha256};

pub const E2EE_MAGIC: &[u8; 4] = b"VE2E";
pub const MIN_E2EE_PACKET_SIZE: usize = 4 + 8 + 16; // magic (4) + seq (8) + poly1305 tag (16)

pub struct SecureTransportSession {
    cipher: ChaCha20Poly1305,
    send_seq: u64,
    recv_seq: u64,
}

impl SecureTransportSession {
    /// Create a new session with a 256-bit symmetric key
    pub fn new(key_bytes: &[u8; 32]) -> Self {
        let key = Key::from_slice(key_bytes);
        let cipher = ChaCha20Poly1305::new(key);
        Self {
            cipher,
            send_seq: 0,
            recv_seq: 0,
        }
    }

    /// Derive a 256-bit symmetric session key from a shared token / PIN hash
    pub fn from_token(token: &str) -> Self {
        let mut hasher = Sha256::new();
        hasher.update(b"VRV_DESK_E2EE_KEY_SALT_v1:");
        hasher.update(token.as_bytes());
        let hash: [u8; 32] = hasher.finalize().into();
        Self::new(&hash)
    }

    /// Check if a binary buffer begins with the VE2E header
    pub fn is_e2ee_packet(data: &[u8]) -> bool {
        data.len() >= MIN_E2EE_PACKET_SIZE && &data[0..4] == E2EE_MAGIC
    }

    /// Encrypt plaintext into an authenticated VE2E packet
    pub fn encrypt(&mut self, plaintext: &[u8]) -> Result<Vec<u8>, String> {
        let mut nonce_bytes = [0u8; 12];
        nonce_bytes[0..8].copy_from_slice(&self.send_seq.to_le_bytes());
        let nonce = Nonce::from_slice(&nonce_bytes);

        let ciphertext = self
            .cipher
            .encrypt(nonce, plaintext)
            .map_err(|e| format!("ChaCha20-Poly1305 encryption failed: {:?}", e))?;

        let mut packet = Vec::with_capacity(12 + ciphertext.len());
        packet.extend_from_slice(E2EE_MAGIC);
        packet.extend_from_slice(&self.send_seq.to_le_bytes());
        packet.extend_from_slice(&ciphertext);

        self.send_seq = self.send_seq.wrapping_add(1);
        Ok(packet)
    }

    /// Decrypt and authenticate an incoming VE2E packet
    pub fn decrypt(&mut self, packet: &[u8]) -> Result<Vec<u8>, String> {
        if !Self::is_e2ee_packet(packet) {
            return Err("Not a valid VE2E packet".to_string());
        }

        let mut seq_bytes = [0u8; 8];
        seq_bytes.copy_from_slice(&packet[4..12]);
        let seq = u64::from_le_bytes(seq_bytes);

        let mut nonce_bytes = [0u8; 12];
        nonce_bytes[0..8].copy_from_slice(&seq_bytes);
        let nonce = Nonce::from_slice(&nonce_bytes);

        let plaintext = self
            .cipher
            .decrypt(nonce, &packet[12..])
            .map_err(|e| format!("ChaCha20-Poly1305 authentication/decryption failed: {:?}", e))?;

        self.recv_seq = seq;
        Ok(plaintext)
    }
}

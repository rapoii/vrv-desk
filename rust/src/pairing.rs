//! Pairing & Trust Establishment
//! Dynamic PIN generation, X25519 key exchange, and authentication verification.

use rand::Rng;
use sha2::{Digest, Sha256};
use std::cell::RefCell;
use x25519_dalek::{EphemeralSecret, PublicKey};

#[derive(Default)]
pub struct PairingHost {
    session_key: Option<[u8; 32]>,
}

pub struct PairingClient {
    secret: EphemeralSecret,
    public_key: PublicKey,
    pin: RefCell<Option<String>>,
}

#[derive(serde::Serialize, serde::Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct ClientHello {
    pub client_pub: [u8; 32],
    pub pin_verifier: [u8; 32],
}

#[derive(serde::Serialize, serde::Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct HostResponse {
    pub host_pub: [u8; 32],
    pub confirmation_auth: [u8; 32],
}

impl PairingHost {
    pub fn new() -> Self {
        Self { session_key: None }
    }

    pub fn generate_dynamic_pin(&mut self, _ttl_secs: u64) -> String {
        let mut rng = rand::thread_rng();
        let pin: u32 = rng.gen_range(100_000..999_999);
        format!("{:06}", pin)
    }

    pub fn process_hello(
        &mut self,
        hello: &ClientHello,
        expected_pin: &str,
    ) -> Result<HostResponse, &'static str> {
        let expected_verifier = compute_pin_verifier(&hello.client_pub, expected_pin);
        if expected_verifier != hello.pin_verifier {
            return Err("Invalid PIN");
        }

        let secret = EphemeralSecret::random_from_rng(rand::thread_rng());
        let host_pub = PublicKey::from(&secret);
        let client_pub = PublicKey::from(hello.client_pub);
        let shared = secret.diffie_hellman(&client_pub);

        let session_key = derive_session_key(shared.as_bytes(), expected_pin);
        let confirmation_auth = compute_confirmation(&session_key, &host_pub.to_bytes());

        self.session_key = Some(session_key);
        Ok(HostResponse {
            host_pub: host_pub.to_bytes(),
            confirmation_auth,
        })
    }

    pub fn session_key(&self) -> Option<[u8; 32]> {
        self.session_key
    }
}

impl Default for PairingClient {
    fn default() -> Self {
        Self::new()
    }
}

impl PairingClient {
    pub fn new() -> Self {
        let secret = EphemeralSecret::random_from_rng(rand::thread_rng());
        let public_key = PublicKey::from(&secret);
        Self {
            secret,
            public_key,
            pin: RefCell::new(None),
        }
    }

    pub fn create_hello(&self, pin: &str) -> ClientHello {
        *self.pin.borrow_mut() = Some(pin.to_string());
        let client_pub = self.public_key.to_bytes();
        let pin_verifier = compute_pin_verifier(&client_pub, pin);
        ClientHello {
            client_pub,
            pin_verifier,
        }
    }

    pub fn finalize(self, response: &HostResponse) -> Result<[u8; 32], &'static str> {
        let host_pub = PublicKey::from(response.host_pub);
        let shared = self.secret.diffie_hellman(&host_pub);
        let pin = self.pin.borrow().clone().unwrap_or_default();
        let session_key = derive_session_key(shared.as_bytes(), &pin);
        let expected_confirmation = compute_confirmation(&session_key, &response.host_pub);
        if expected_confirmation != response.confirmation_auth {
            return Err("Host confirmation mismatch");
        }
        Ok(session_key)
    }
}

fn compute_pin_verifier(pubkey: &[u8; 32], pin: &str) -> [u8; 32] {
    let mut hasher = Sha256::new();
    hasher.update(pubkey);
    hasher.update(pin.as_bytes());
    hasher.finalize().into()
}

fn compute_confirmation(key: &[u8; 32], host_pub: &[u8; 32]) -> [u8; 32] {
    let mut hasher = Sha256::new();
    hasher.update(key);
    hasher.update(host_pub);
    hasher.finalize().into()
}

fn derive_session_key(shared_secret: &[u8; 32], pin: &str) -> [u8; 32] {
    let mut hasher = Sha256::new();
    hasher.update(shared_secret);
    hasher.update(pin.as_bytes());
    hasher.finalize().into()
}

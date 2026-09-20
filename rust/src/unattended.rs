//! Unattended Access Configuration and Verification
//! Provides persistent static password storage with SHA-256 + salt.

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::path::PathBuf;

#[derive(Serialize, Deserialize, Debug, Clone, Default, PartialEq, Eq)]
pub struct UnattendedConfig {
    pub enabled: bool,
    #[serde(default)]
    pub salt: Option<String>,
    #[serde(default)]
    pub password_hash: Option<String>,
}

impl UnattendedConfig {
    pub fn new() -> Self {
        Self::default()
    }

    /// Resolves the persistent config directory for VrV Desk.
    pub fn get_config_dir() -> PathBuf {
        if let Ok(local_app_data) = std::env::var("LOCALAPPDATA") {
            let path = PathBuf::from(local_app_data).join("VrVDesk");
            let _ = std::fs::create_dir_all(&path);
            path
        } else if let Ok(home) = std::env::var("USERPROFILE").or_else(|_| std::env::var("HOME")) {
            let path = PathBuf::from(home).join(".vrvdesk");
            let _ = std::fs::create_dir_all(&path);
            path
        } else {
            PathBuf::from(".")
        }
    }

    pub fn get_config_path() -> PathBuf {
        Self::get_config_dir().join("unattended.json")
    }

    /// Loads the configuration from disk, with environment variable override support.
    pub fn load() -> Self {
        // Environment variable override (ideal for headless / automation)
        if let Ok(env_pass) = std::env::var("VRV_UNATTENDED_PASSWORD") {
            let pass = env_pass.trim();
            if !pass.is_empty() {
                let mut cfg = Self::default();
                cfg.set_password(pass);
                return cfg;
            }
        }

        let path = Self::get_config_path();
        if path.exists() {
            if let Ok(data) = std::fs::read_to_string(&path) {
                if let Ok(cfg) = serde_json::from_str::<Self>(&data) {
                    return cfg;
                }
            }
        }

        Self::default()
    }

    /// Saves the current configuration to disk.
    pub fn save(&self) -> std::io::Result<()> {
        let path = Self::get_config_path();
        if let Some(parent) = path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        let json = serde_json::to_string_pretty(self)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;
        std::fs::write(&path, json)
    }

    /// Sets and hashes a new unattended access permanent password.
    pub fn set_password(&mut self, password: &str) {
        let salt_bytes: [u8; 16] = rand::random();
        let salt_hex = hex::encode(salt_bytes);
        let hash_hex = Self::hash_password(&salt_hex, password);
        self.salt = Some(salt_hex);
        self.password_hash = Some(hash_hex);
        self.enabled = true;
    }

    /// Clears the permanent password and disables unattended access.
    pub fn clear_password(&mut self) {
        self.salt = None;
        self.password_hash = None;
        self.enabled = false;
    }

    /// Verifies if a given input candidate matches the unattended password.
    pub fn verify(&self, candidate: &str) -> bool {
        if !self.enabled {
            return false;
        }

        // Check env override first if present
        if let Ok(env_pass) = std::env::var("VRV_UNATTENDED_PASSWORD") {
            if !env_pass.trim().is_empty() && candidate == env_pass.trim() {
                return true;
            }
        }

        if let (Some(salt), Some(expected_hash)) = (&self.salt, &self.password_hash) {
            let candidate_hash = Self::hash_password(salt, candidate);
            candidate_hash == *expected_hash
        } else {
            false
        }
    }

    fn hash_password(salt: &str, password: &str) -> String {
        let mut hasher = Sha256::new();
        hasher.update(salt.as_bytes());
        hasher.update(password.as_bytes());
        hex::encode(hasher.finalize())
    }
}

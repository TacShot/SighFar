//! Automatic RSA-2048 key-pair management with encrypted on-disk storage.
//!
//! Keys are stored in `~/.sighfar/keys.enc`, encrypted with a per-device AES-256-GCM
//! key held in `~/.sighfar/keys.key`.  A new key pair is generated automatically the
//! first time one is requested.

use std::{
    fs,
    path::{Path, PathBuf},
};

use aes_gcm::{
    Aes256Gcm, Nonce,
    aead::{Aead, KeyInit},
};
use anyhow::{Context, Result, anyhow};
use base64::{Engine as _, engine::general_purpose::STANDARD as BASE64};
use chrono::Utc;
use rand::{Rng, rng};
use rand_core::OsRng;
use rsa::{
    Oaep, RsaPrivateKey, RsaPublicKey,
    pkcs8::{DecodePrivateKey, DecodePublicKey, EncodePrivateKey, EncodePublicKey},
    sha2::Sha256,
};
use sha2::Digest;

use crate::models::KeyEntry;

pub struct RsaKeyStore {
    root: PathBuf,
    key_file: PathBuf,
    store_file: PathBuf,
}

impl Default for RsaKeyStore {
    fn default() -> Self {
        let root = default_support_directory();
        Self {
            key_file: root.join("keys.key"),
            store_file: root.join("keys.enc"),
            root,
        }
    }
}

impl RsaKeyStore {
    /// Return all stored key entries.
    pub fn list(&self) -> Result<Vec<KeyEntry>> {
        self.ensure_root()?;
        if !self.store_file.exists() {
            return Ok(Vec::new());
        }
        self.load_entries()
    }

    /// Return the primary key pair, generating and persisting one if none exists yet.
    pub fn primary(&self) -> Result<KeyEntry> {
        let entries = self.list()?;
        if let Some(entry) = entries.into_iter().find(|e| e.label == "primary") {
            return Ok(entry);
        }
        self.generate("primary")
    }

    /// Generate a new RSA-2048 key pair with the given label and persist it.
    pub fn generate(&self, label: &str) -> Result<KeyEntry> {
        let mut entries = self.list()?;
        let private_key = RsaPrivateKey::new(&mut OsRng, 2048)
            .map_err(|err| anyhow!("failed to generate RSA key: {err}"))?;
        let public_key = RsaPublicKey::from(&private_key);

        let private_der = private_key
            .to_pkcs8_der()
            .map_err(|err| anyhow!("failed to encode private key: {err}"))?;
        let public_der = public_key
            .to_public_key_der()
            .map_err(|err| anyhow!("failed to encode public key: {err}"))?;

        let fingerprint = {
            let mut h = sha2::Sha256::new();
            h.update(public_der.as_bytes());
            format!("sha256:{}", hex::encode(h.finalize()))
        };

        let entry = KeyEntry {
            label: label.to_string(),
            fingerprint,
            private_key_b64: BASE64.encode(private_der.as_bytes()),
            public_key_b64: BASE64.encode(public_der.as_bytes()),
            created_at: Utc::now(),
        };
        entries.push(entry.clone());
        self.save_entries(&entries)?;
        Ok(entry)
    }

    /// Delete the key with the given label.  Returns `true` if a key was removed.
    pub fn delete(&self, label: &str) -> Result<bool> {
        let mut entries = self.list()?;
        let before = entries.len();
        entries.retain(|e| e.label != label);
        if entries.len() < before {
            self.save_entries(&entries)?;
            return Ok(true);
        }
        Ok(false)
    }

    /// Encrypt `plaintext` bytes with the public key of the named entry (OAEP/SHA-256).
    pub fn encrypt(&self, label: &str, plaintext: &[u8]) -> Result<Vec<u8>> {
        let entry = self
            .list()?
            .into_iter()
            .find(|e| e.label == label)
            .ok_or_else(|| anyhow!("RSA key '{label}' not found"))?;
        let der = BASE64
            .decode(&entry.public_key_b64)
            .context("failed to decode public key base64")?;
        let public_key = RsaPublicKey::from_public_key_der(&der)
            .map_err(|err| anyhow!("failed to parse RSA public key: {err}"))?;
        public_key
            .encrypt(&mut OsRng, Oaep::new::<Sha256>(), plaintext)
            .map_err(|err| anyhow!("RSA encryption failed: {err}"))
    }

    /// Decrypt `ciphertext` bytes with the private key of the named entry (OAEP/SHA-256).
    pub fn decrypt(&self, label: &str, ciphertext: &[u8]) -> Result<Vec<u8>> {
        let entry = self
            .list()?
            .into_iter()
            .find(|e| e.label == label)
            .ok_or_else(|| anyhow!("RSA key '{label}' not found"))?;
        let der = BASE64
            .decode(&entry.private_key_b64)
            .context("failed to decode private key base64")?;
        let private_key = RsaPrivateKey::from_pkcs8_der(&der)
            .map_err(|err| anyhow!("failed to parse RSA private key: {err}"))?;
        private_key
            .decrypt(Oaep::new::<Sha256>(), ciphertext)
            .map_err(|err| anyhow!("RSA decryption failed: {err}"))
    }

    // ── internal ──────────────────────────────────────────────────────────

    fn load_entries(&self) -> Result<Vec<KeyEntry>> {
        let payload = fs::read(&self.store_file).context("failed to read key store")?;
        if payload.len() < 12 {
            return Ok(Vec::new());
        }
        let (nonce_bytes, ciphertext) = payload.split_at(12);
        let cipher = Aes256Gcm::new_from_slice(&self.aes_key()?)
            .map_err(|_| anyhow!("invalid AES key for key store"))?;
        let plaintext = cipher
            .decrypt(Nonce::from_slice(nonce_bytes), ciphertext)
            .map_err(|_| anyhow!("failed to decrypt key store"))?;
        serde_json::from_slice(&plaintext).context("failed to parse key store JSON")
    }

    fn save_entries(&self, entries: &[KeyEntry]) -> Result<()> {
        self.ensure_root()?;
        let plaintext = serde_json::to_vec(entries).context("failed to serialise key store")?;
        let cipher = Aes256Gcm::new_from_slice(&self.aes_key()?)
            .map_err(|_| anyhow!("invalid AES key for key store"))?;
        let nonce = random_nonce();
        let ciphertext = cipher
            .encrypt(Nonce::from_slice(&nonce), plaintext.as_ref())
            .map_err(|_| anyhow!("failed to encrypt key store"))?;
        let mut payload = nonce.to_vec();
        payload.extend(ciphertext);
        fs::write(&self.store_file, payload).context("failed to write key store")
    }

    fn aes_key(&self) -> Result<[u8; 32]> {
        self.ensure_root()?;
        if self.key_file.exists() {
            let existing = fs::read(&self.key_file).context("failed to read key store key")?;
            if existing.len() != 32 {
                return Err(anyhow!("key store key has unexpected length"));
            }
            let mut key = [0u8; 32];
            key.copy_from_slice(&existing);
            return Ok(key);
        }
        let mut key = [0u8; 32];
        rng().fill(&mut key);
        fs::write(&self.key_file, key).context("failed to write key store key")?;
        Ok(key)
    }

    fn ensure_root(&self) -> Result<()> {
        fs::create_dir_all(&self.root).context("failed to create support directory")
    }
}

fn default_support_directory() -> PathBuf {
    // $HOME on Unix, %USERPROFILE% on Windows, fall back to the current directory.
    std::env::var_os("HOME")
        .or_else(|| std::env::var_os("USERPROFILE"))
        .map(PathBuf::from)
        .unwrap_or_else(|| Path::new(".").to_path_buf())
        .join(".sighfar")
}

fn random_nonce() -> [u8; 12] {
    let mut nonce = [0u8; 12];
    rng().fill(&mut nonce);
    nonce
}

#[cfg(test)]
mod tests {
    use super::RsaKeyStore;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn temp_store() -> RsaKeyStore {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let root = std::env::temp_dir().join(format!("sighfar-keys-test-{unique}"));
        RsaKeyStore {
            key_file: root.join("keys.key"),
            store_file: root.join("keys.enc"),
            root,
        }
    }

    #[test]
    fn rsa_key_generate_and_round_trip() {
        let store = temp_store();
        let entry = store.generate("test").unwrap();
        assert_eq!(entry.label, "test");
        assert!(entry.fingerprint.starts_with("sha256:"));

        let plaintext = b"hello sighfar";
        let ciphertext = store.encrypt("test", plaintext).unwrap();
        let recovered = store.decrypt("test", &ciphertext).unwrap();
        assert_eq!(recovered, plaintext);
    }

    #[test]
    fn rsa_primary_auto_generates() {
        let store = temp_store();
        let primary = store.primary().unwrap();
        assert_eq!(primary.label, "primary");
        // Calling again should return the same key (same fingerprint).
        let primary2 = store.primary().unwrap();
        assert_eq!(primary.fingerprint, primary2.fingerprint);
    }
}

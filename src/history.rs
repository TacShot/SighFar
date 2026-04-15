use std::{
    fs,
    path::{Path, PathBuf},
};

use aes_gcm::{
    Aes256Gcm, Nonce,
    aead::{Aead, KeyInit},
};
use anyhow::{Context, Result, anyhow, bail};
use rand::{Rng, rng};

use crate::models::HistoryEntry;

pub struct HistoryStore {
    root: PathBuf,
    key_file: PathBuf,
    history_file: PathBuf,
}

impl Default for HistoryStore {
    fn default() -> Self {
        let root = default_support_directory();
        Self {
            key_file: root.join("history.key"),
            history_file: root.join("history.enc"),
            root,
        }
    }
}

impl HistoryStore {
    pub fn append(&self, entry: HistoryEntry) -> Result<()> {
        let mut items = self.load()?;
        items.insert(0, entry);
        self.save(&items)
    }

    pub fn load(&self) -> Result<Vec<HistoryEntry>> {
        self.ensure_root()?;
        if !self.history_file.exists() {
            return Ok(Vec::new());
        }

        let payload = fs::read(&self.history_file).context("failed to read encrypted history")?;
        if payload.is_empty() {
            return Ok(Vec::new());
        }
        if payload.len() < 12 {
            bail!("history payload is malformed");
        }

        let (nonce_bytes, ciphertext) = payload.split_at(12);
        let cipher = Aes256Gcm::new_from_slice(&self.history_key()?)
            .map_err(|_| anyhow!("invalid AES key"))?;
        let plaintext = cipher
            .decrypt(Nonce::from_slice(nonce_bytes), ciphertext)
            .map_err(|_| anyhow!("failed to decrypt history file"))?;

        serde_json::from_slice(&plaintext).context("failed to parse history JSON")
    }

    pub fn diagnostics(&self) -> String {
        format!(
            "storage: {}\nkey: {}\nhistory: {}",
            self.root.display(),
            self.key_file.display(),
            self.history_file.display()
        )
    }

    pub fn export_encrypted_blob(&self) -> Result<Vec<u8>> {
        self.ensure_root()?;
        if !self.history_file.exists() {
            return Ok(Vec::new());
        }
        fs::read(&self.history_file).context("failed to read encrypted history blob")
    }

    pub fn import_encrypted_blob(&self, blob: &[u8]) -> Result<()> {
        self.ensure_root()?;
        fs::write(&self.history_file, blob).context("failed to write imported encrypted history blob")
    }

    fn save(&self, entries: &[HistoryEntry]) -> Result<()> {
        self.ensure_root()?;
        let plaintext = serde_json::to_vec_pretty(entries).context("failed to encode history")?;
        let cipher = Aes256Gcm::new_from_slice(&self.history_key()?)
            .map_err(|_| anyhow!("invalid AES key"))?;
        let nonce = random_nonce();
        let ciphertext = cipher
            .encrypt(Nonce::from_slice(&nonce), plaintext.as_ref())
            .map_err(|_| anyhow!("failed to encrypt history"))?;

        let mut payload = nonce.to_vec();
        payload.extend(ciphertext);
        fs::write(&self.history_file, payload).context("failed to write history file")
    }

    fn history_key(&self) -> Result<[u8; 32]> {
        self.ensure_root()?;
        if self.key_file.exists() {
            let existing = fs::read(&self.key_file).context("failed to read history key")?;
            if existing.len() != 32 {
                bail!("history key has unexpected length");
            }
            let mut key = [0u8; 32];
            key.copy_from_slice(&existing);
            return Ok(key);
        }

        let mut key = [0u8; 32];
        rng().fill(&mut key);
        fs::write(&self.key_file, key).context("failed to write history key")?;
        Ok(key)
    }

    fn ensure_root(&self) -> Result<()> {
        fs::create_dir_all(&self.root).context("failed to create support directory")
    }
}

fn default_support_directory() -> PathBuf {
    std::env::var_os("HOME")
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
    use std::time::{SystemTime, UNIX_EPOCH};

    use chrono::{TimeZone, Utc};

    use super::HistoryStore;
    use crate::models::{HistoryEntry, OperationKind, TechniqueDescriptor};

    #[test]
    fn history_store_persists_entries() {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let root = std::env::temp_dir().join(format!("sighfar-test-{unique}"));
        let store = HistoryStore {
            key_file: root.join("history.key"),
            history_file: root.join("history.enc"),
            root,
        };

        let entry = HistoryEntry {
            id: "entry-1".to_string(),
            timestamp: Utc.timestamp_opt(12_345, 0).unwrap(),
            operation: OperationKind::Encode,
            input_preview: "hello".to_string(),
            output_preview: "uryyb".to_string(),
            techniques: vec![TechniqueDescriptor::Caesar { shift: 13 }],
            used_secure_envelope: false,
        };

        store.append(entry).unwrap();
        let items = store.load().unwrap();

        assert_eq!(items.len(), 1);
        assert_eq!(items[0].input_preview, "hello");
        assert!(matches!(
            items[0].techniques.as_slice(),
            [TechniqueDescriptor::Caesar { shift: 13 }]
        ));
    }
}

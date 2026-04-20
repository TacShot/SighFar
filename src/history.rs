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
    pub fn with_root(root: PathBuf) -> Self {
        Self {
            key_file: root.join("history.key"),
            history_file: root.join("history.enc"),
            root,
        }
    }
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

    fn unique_store(label: &str) -> HistoryStore {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let root = std::env::temp_dir().join(format!("sighfar-test-{label}-{unique}"));
        HistoryStore::with_root(root)
    }

    fn make_entry(id: &str, preview: &str) -> HistoryEntry {
        HistoryEntry {
            id: id.to_string(),
            timestamp: Utc.timestamp_opt(12_345, 0).unwrap(),
            operation: OperationKind::Encode,
            input_preview: preview.to_string(),
            output_preview: "out".to_string(),
            techniques: vec![TechniqueDescriptor::Caesar { shift: 13 }],
            used_secure_envelope: false,
        }
    }

    #[test]
    fn history_store_persists_entries() {
        let store = unique_store("basic");

        let entry = make_entry("entry-1", "hello");

        store.append(entry).unwrap();
        let items = store.load().unwrap();

        assert_eq!(items.len(), 1);
        assert_eq!(items[0].input_preview, "hello");
        assert!(matches!(
            items[0].techniques.as_slice(),
            [TechniqueDescriptor::Caesar { shift: 13 }]
        ));
    }

    #[test]
    fn load_returns_empty_when_no_file() {
        let store = unique_store("empty");
        let items = store.load().unwrap();
        assert!(items.is_empty());
    }

    #[test]
    fn append_prepends_newest_first() {
        let store = unique_store("prepend");

        store.append(make_entry("first", "first message")).unwrap();
        store.append(make_entry("second", "second message")).unwrap();
        store.append(make_entry("third", "third message")).unwrap();

        let items = store.load().unwrap();
        assert_eq!(items.len(), 3);
        assert_eq!(items[0].input_preview, "third message");
        assert_eq!(items[1].input_preview, "second message");
        assert_eq!(items[2].input_preview, "first message");
    }

    #[test]
    fn export_and_import_blob_round_trip() {
        // Export from store_a, then import into another store that shares
        // the same encryption key (same-device restore scenario).
        let store_a = unique_store("export");
        store_a.append(make_entry("entry-x", "exported data")).unwrap();

        let blob = store_a.export_encrypted_blob().unwrap();
        assert!(!blob.is_empty());

        // store_b shares the key file from store_a so it can decrypt the blob.
        let store_b = HistoryStore {
            key_file: store_a.key_file.clone(),
            history_file: store_a.root.join("restored.enc"),
            root: store_a.root.clone(),
        };
        store_b.import_encrypted_blob(&blob).unwrap();

        let items = store_b.load().unwrap();
        assert_eq!(items.len(), 1);
        assert_eq!(items[0].input_preview, "exported data");
    }

    #[test]
    fn export_returns_empty_when_no_history() {
        let store = unique_store("noexport");
        let blob = store.export_encrypted_blob().unwrap();
        assert!(blob.is_empty());
    }

    #[test]
    fn diagnostics_contains_paths() {
        let store = unique_store("diag");
        let diag = store.diagnostics();
        assert!(diag.contains("storage:"));
        assert!(diag.contains("key:"));
        assert!(diag.contains("history:"));
    }

    #[test]
    fn history_persists_decode_operation() {
        let store = unique_store("decode-op");
        let entry = HistoryEntry {
            id: "decode-entry".to_string(),
            timestamp: Utc.timestamp_opt(99_999, 0).unwrap(),
            operation: OperationKind::Decode,
            input_preview: "cipher".to_string(),
            output_preview: "plain".to_string(),
            techniques: vec![TechniqueDescriptor::Reverse],
            used_secure_envelope: true,
        };
        store.append(entry).unwrap();
        let items = store.load().unwrap();
        assert_eq!(items.len(), 1);
        assert!(matches!(items[0].operation, OperationKind::Decode));
        assert!(items[0].used_secure_envelope);
    }
}

use anyhow::{Result, anyhow, bail};
use base64::{Engine as _, engine::general_purpose::STANDARD as BASE64};
use chrono::Utc;

use crate::{
    cipher::CipherPipeline,
    history::HistoryStore,
    keys::RsaKeyStore,
    models::{EncodedMessage, HistoryEntry, KeyEntry, OperationKind, SecureKeyPair, TechniqueDescriptor},
    secure::SecureEnvelope,
};

pub struct SighFarCore {
    pipeline: CipherPipeline,
    secure_envelope: SecureEnvelope,
    history_store: HistoryStore,
    key_store: RsaKeyStore,
}

impl Default for SighFarCore {
    fn default() -> Self {
        Self {
            pipeline: CipherPipeline,
            secure_envelope: SecureEnvelope,
            history_store: HistoryStore::default(),
            key_store: RsaKeyStore::default(),
        }
    }
}

impl SighFarCore {
    pub fn encode(
        &self,
        message: &str,
        chain: &str,
        use_secure_envelope: bool,
        passphrase: Option<&str>,
    ) -> Result<EncodedMessage> {
        let techniques = parse_techniques(chain)?;
        let transformed = self.pipeline.encode(message, &techniques)?;

        let mut key_pair = None;
        let mut secure_payload = None;
        if use_secure_envelope {
            let passphrase = passphrase
                .filter(|value| !value.trim().is_empty())
                .ok_or_else(|| anyhow!("Primary passphrase is required for secure envelope mode."))?;
            let generated = self.secure_envelope.make_key_pair(passphrase);
            secure_payload = Some(self.secure_envelope.seal(&transformed, &generated)?);
            key_pair = Some(generated);
        }

        let result = EncodedMessage {
            original_input: message.to_string(),
            transformed_text: transformed,
            secure_payload,
            techniques,
            used_secure_envelope: key_pair.is_some(),
            key_pair,
        };

        self.history_store
            .append(history_entry_for(&result, OperationKind::Encode))?;
        Ok(result)
    }

    pub fn decode(
        &self,
        raw_input: &str,
        chain: &str,
        secure_payload: bool,
        passphrase: Option<&str>,
        companion_code: Option<&str>,
    ) -> Result<EncodedMessage> {
        let resolved_input = if secure_payload {
            let pair = SecureKeyPair {
                passphrase: passphrase
                    .filter(|value| !value.trim().is_empty())
                    .ok_or_else(|| anyhow!("Primary passphrase is required."))?
                    .to_string(),
                companion_code: companion_code
                    .filter(|value| !value.trim().is_empty())
                    .ok_or_else(|| anyhow!("Companion code is required."))?
                    .to_string(),
            };
            self.secure_envelope.open(raw_input, &pair)?
        } else {
            raw_input.to_string()
        };

        let techniques = parse_techniques(chain)?;
        let decoded = self.pipeline.decode(&resolved_input, &techniques)?;

        let result = EncodedMessage {
            original_input: resolved_input,
            transformed_text: decoded,
            secure_payload: None,
            techniques,
            used_secure_envelope: secure_payload,
            key_pair: None,
        };

        self.history_store
            .append(history_entry_for(&result, OperationKind::Decode))?;
        Ok(result)
    }

    // ── RSA operations ────────────────────────────────────────────────────

    /// Encrypt a UTF-8 message with the RSA public key for the given label.
    /// Returns base64-encoded ciphertext.
    pub fn rsa_encrypt(&self, message: &str, key_label: &str) -> Result<String> {
        let ciphertext = self.key_store.encrypt(key_label, message.as_bytes())?;
        Ok(BASE64.encode(ciphertext))
    }

    /// Decrypt a base64-encoded RSA ciphertext with the private key for the given label.
    pub fn rsa_decrypt(&self, ciphertext_b64: &str, key_label: &str) -> Result<String> {
        let ciphertext = BASE64
            .decode(ciphertext_b64.trim())
            .map_err(|_| anyhow!("RSA ciphertext is not valid base64"))?;
        let plaintext = self.key_store.decrypt(key_label, &ciphertext)?;
        String::from_utf8(plaintext).map_err(|_| anyhow!("RSA plaintext is not valid UTF-8"))
    }

    // ── Key management ────────────────────────────────────────────────────

    /// List all stored RSA key entries.
    pub fn list_keys(&self) -> Result<Vec<KeyEntry>> {
        self.key_store.list()
    }

    /// Ensure the primary RSA key pair exists (generates one on first call).
    pub fn ensure_primary_key(&self) -> Result<KeyEntry> {
        self.key_store.primary()
    }

    /// Generate a new named RSA-2048 key pair.
    pub fn generate_key(&self, label: &str) -> Result<KeyEntry> {
        self.key_store.generate(label)
    }

    /// Delete the RSA key pair with the given label.
    pub fn delete_key(&self, label: &str) -> Result<bool> {
        self.key_store.delete(label)
    }

    // ── History & diagnostics ─────────────────────────────────────────────

    pub fn load_history(&self) -> Result<Vec<HistoryEntry>> {
        self.history_store.load()
    }

    pub fn diagnostics(&self) -> String {
        self.history_store.diagnostics()
    }

    pub fn export_encrypted_history_blob(&self) -> Result<Vec<u8>> {
        self.history_store.export_encrypted_blob()
    }

    pub fn import_encrypted_history_blob(&self, blob: &[u8]) -> Result<()> {
        self.history_store.import_encrypted_blob(blob)
    }
}

pub fn parse_techniques(input: &str) -> Result<Vec<TechniqueDescriptor>> {
    let items: Vec<_> = input
        .split(',')
        .map(str::trim)
        .filter(|item| !item.is_empty())
        .collect();

    if items.is_empty() {
        bail!("You need at least one technique.");
    }

    items
        .into_iter()
        .map(|item| {
            if item == "morse" {
                return Ok(TechniqueDescriptor::Morse);
            }
            if item == "reverse" {
                return Ok(TechniqueDescriptor::Reverse);
            }
            if item == "sha256" {
                return Ok(TechniqueDescriptor::Sha256);
            }
            if item == "sha512" {
                return Ok(TechniqueDescriptor::Sha512);
            }
            if let Some(shift) = item.strip_prefix("caesar:") {
                return Ok(TechniqueDescriptor::Caesar {
                    shift: shift.parse().map_err(|_| anyhow!("Caesar format is caesar:3"))?,
                });
            }
            if let Some(keyword) = item.strip_prefix("vigenere:") {
                if keyword.is_empty() {
                    bail!("Vigenere format is vigenere:keyword");
                }
                return Ok(TechniqueDescriptor::Vigenere {
                    keyword: keyword.to_string(),
                });
            }
            if let Some(rails) = item.strip_prefix("railfence:") {
                return Ok(TechniqueDescriptor::RailFence {
                    rails: rails
                        .parse()
                        .map_err(|_| anyhow!("RailFence format is railfence:3"))?,
                });
            }

            bail!("Unknown technique: {item}")
        })
        .collect()
}

pub fn history_entry_for(result: &EncodedMessage, operation: OperationKind) -> HistoryEntry {
    HistoryEntry {
        id: format!("entry-{}", Utc::now().timestamp_millis()),
        timestamp: Utc::now(),
        operation,
        input_preview: truncate(&result.original_input),
        output_preview: truncate(
            result
                .secure_payload
                .as_deref()
                .unwrap_or(&result.transformed_text),
        ),
        techniques: result.techniques.clone(),
        used_secure_envelope: result.used_secure_envelope,
    }
}

pub fn truncate(value: &str) -> String {
    let cleaned = value.replace('\n', " ");
    if cleaned.chars().count() > 80 {
        format!("{}...", cleaned.chars().take(77).collect::<String>())
    } else {
        cleaned
    }
}

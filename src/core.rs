use anyhow::{Result, anyhow, bail};
use chrono::Utc;

use crate::{
    cipher::CipherPipeline,
    history::HistoryStore,
    models::{EncodedMessage, HistoryEntry, OperationKind, SecureKeyPair, TechniqueDescriptor},
    secure::SecureEnvelope,
};

pub struct SighFarCore {
    pipeline: CipherPipeline,
    secure_envelope: SecureEnvelope,
    history_store: HistoryStore,
}

impl Default for SighFarCore {
    fn default() -> Self {
        Self {
            pipeline: CipherPipeline,
            secure_envelope: SecureEnvelope,
            history_store: HistoryStore::default(),
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

#[cfg(test)]
mod tests {
    use std::time::{SystemTime, UNIX_EPOCH};

    use super::{SighFarCore, parse_techniques, truncate};
    use crate::history::HistoryStore;
    use crate::models::TechniqueDescriptor;

    fn unique_core(label: &str) -> SighFarCore {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let root = std::env::temp_dir().join(format!("sighfar-core-{label}-{unique}"));
        SighFarCore {
            pipeline: crate::cipher::CipherPipeline,
            secure_envelope: crate::secure::SecureEnvelope,
            history_store: HistoryStore::with_root(root),
        }
    }

    // ── parse_techniques ──────────────────────────────────────────────────────

    #[test]
    fn parse_morse() {
        let t = parse_techniques("morse").unwrap();
        assert_eq!(t.len(), 1);
        assert!(matches!(t[0], TechniqueDescriptor::Morse));
    }

    #[test]
    fn parse_reverse() {
        let t = parse_techniques("reverse").unwrap();
        assert!(matches!(t[0], TechniqueDescriptor::Reverse));
    }

    #[test]
    fn parse_caesar() {
        let t = parse_techniques("caesar:7").unwrap();
        assert!(matches!(t[0], TechniqueDescriptor::Caesar { shift: 7 }));
    }

    #[test]
    fn parse_vigenere() {
        let t = parse_techniques("vigenere:secret").unwrap();
        assert!(matches!(
            &t[0],
            TechniqueDescriptor::Vigenere { keyword } if keyword == "secret"
        ));
    }

    #[test]
    fn parse_railfence() {
        let t = parse_techniques("railfence:4").unwrap();
        assert!(matches!(t[0], TechniqueDescriptor::RailFence { rails: 4 }));
    }

    #[test]
    fn parse_chained_techniques() {
        let t = parse_techniques("caesar:2,reverse,morse").unwrap();
        assert_eq!(t.len(), 3);
        assert!(matches!(t[0], TechniqueDescriptor::Caesar { shift: 2 }));
        assert!(matches!(t[1], TechniqueDescriptor::Reverse));
        assert!(matches!(t[2], TechniqueDescriptor::Morse));
    }

    #[test]
    fn parse_techniques_with_spaces() {
        let t = parse_techniques("caesar:3 , reverse").unwrap();
        assert_eq!(t.len(), 2);
    }

    #[test]
    fn parse_empty_fails() {
        let result = parse_techniques("");
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("at least one technique"));
    }

    #[test]
    fn parse_unknown_technique_fails() {
        let result = parse_techniques("rot13");
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Unknown technique"));
    }

    #[test]
    fn parse_vigenere_empty_keyword_fails() {
        let result = parse_techniques("vigenere:");
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("vigenere:keyword"));
    }

    #[test]
    fn parse_caesar_invalid_shift_fails() {
        let result = parse_techniques("caesar:abc");
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("caesar:3"));
    }

    #[test]
    fn parse_railfence_invalid_fails() {
        let result = parse_techniques("railfence:xyz");
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("railfence:3"));
    }

    // ── truncate ──────────────────────────────────────────────────────────────

    #[test]
    fn truncate_short_string_unchanged() {
        let s = "hello world";
        assert_eq!(truncate(s), s);
    }

    #[test]
    fn truncate_exactly_80_chars_unchanged() {
        let s: String = "a".repeat(80);
        assert_eq!(truncate(&s), s);
    }

    #[test]
    fn truncate_81_chars_gets_ellipsis() {
        let s: String = "b".repeat(81);
        let result = truncate(&s);
        assert!(result.ends_with("..."));
        assert_eq!(result.chars().count(), 80); // 77 + 3
    }

    #[test]
    fn truncate_replaces_newlines_with_spaces() {
        let s = "line1\nline2";
        assert_eq!(truncate(s), "line1 line2");
    }

    #[test]
    fn truncate_newlines_then_long_truncates() {
        let s = format!("{}\n{}", "a".repeat(50), "b".repeat(40));
        let result = truncate(&s);
        assert!(result.ends_with("..."));
        assert!(!result.contains('\n'));
    }

    // ── SighFarCore encode/decode ─────────────────────────────────────────────

    #[test]
    fn core_encode_and_decode_plain() {
        let core = unique_core("plain");
        let encoded = core.encode("hello", "caesar:13", false, None).unwrap();
        assert_eq!(encoded.transformed_text, "uryyb");
        assert!(!encoded.used_secure_envelope);
        assert!(encoded.secure_payload.is_none());

        let decoded = core
            .decode(&encoded.transformed_text, "caesar:13", false, None, None)
            .unwrap();
        assert_eq!(decoded.transformed_text, "hello");
    }

    #[test]
    fn core_encode_without_passphrase_fails_when_secure() {
        let core = unique_core("nopw");
        let result = core.encode("msg", "reverse", true, None);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("passphrase"));
    }

    #[test]
    fn core_encode_with_blank_passphrase_fails() {
        let core = unique_core("blankpw");
        let result = core.encode("msg", "reverse", true, Some("   "));
        assert!(result.is_err());
    }

    #[test]
    fn core_encode_with_secure_envelope() {
        let core = unique_core("secure");
        let encoded = core
            .encode("secret message", "reverse", true, Some("my-pass"))
            .unwrap();
        assert!(encoded.used_secure_envelope);
        assert!(encoded.secure_payload.is_some());
        assert!(encoded.key_pair.is_some());
        let kp = encoded.key_pair.as_ref().unwrap();
        assert_eq!(kp.passphrase, "my-pass");
    }

    #[test]
    fn core_decode_secure_payload() {
        let core = unique_core("secure-decode");
        let encoded = core
            .encode("top secret", "caesar:3", true, Some("pw"))
            .unwrap();
        let kp = encoded.key_pair.as_ref().unwrap();
        let payload = encoded.secure_payload.as_ref().unwrap();

        let decoded = core
            .decode(
                payload,
                "caesar:3",
                true,
                Some(&kp.passphrase),
                Some(&kp.companion_code),
            )
            .unwrap();
        assert_eq!(decoded.transformed_text, "top secret");
    }

    #[test]
    fn core_decode_missing_passphrase_fails() {
        let core = unique_core("decnopw");
        let result = core.decode("payload", "reverse", true, None, Some("CODE"));
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("passphrase"));
    }

    #[test]
    fn core_decode_missing_companion_code_fails() {
        let core = unique_core("decnocode");
        let result = core.decode("payload", "reverse", true, Some("pass"), None);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Companion code"));
    }

    #[test]
    fn core_load_history_returns_entries_after_encode() {
        let core = unique_core("hist");
        core.encode("test msg", "reverse", false, None).unwrap();
        let history = core.load_history().unwrap();
        assert_eq!(history.len(), 1);
        assert_eq!(history[0].input_preview, "test msg");
    }
}

use anyhow::{Result, anyhow, bail};
use chrono::Utc;

use crate::{
    cipher::CipherPipeline,
    history::HistoryStore,
    models::{EncodedMessage, HistoryEntry, OperationKind, SecureKeyPair, TechniqueDescriptor},
    secure::SecureEnvelope,
    ui::TerminalUi,
};

pub struct SighFarApp {
    ui: TerminalUi,
    pipeline: CipherPipeline,
    secure_envelope: SecureEnvelope,
    history_store: HistoryStore,
}

impl Default for SighFarApp {
    fn default() -> Self {
        Self {
            ui: TerminalUi,
            pipeline: CipherPipeline,
            secure_envelope: SecureEnvelope,
            history_store: HistoryStore::default(),
        }
    }
}

impl SighFarApp {
    pub fn run(&self) -> Result<()> {
        loop {
            self.ui.render_header();
            match self.ui.prompt("Choose a module:")?.as_str() {
                "1" => self.run_encode_flow()?,
                "2" => self.run_decode_flow()?,
                "3" => self.show_history()?,
                "4" => self.show_settings()?,
                "5" => self.show_roadmap()?,
                "0" | "q" | "quit" => break,
                other => {
                    self.ui.print_panel("Input", &format!("Unknown option: {other}"));
                    self.ui.pause()?;
                }
            }
        }

        Ok(())
    }

    fn run_encode_flow(&self) -> Result<()> {
        self.ui.clear_screen();
        self.ui.print_panel(
            "Encode",
            "Stack one or more techniques in order.\nAvailable: morse, caesar, vigenere, railfence, reverse\nExample: morse,caesar:4,reverse",
        );

        let message = self.ui.prompt("Message:")?;
        let chain = self.ui.prompt("Technique chain:")?;
        let secure = self.ui.prompt("Wrap in secure paired-key envelope? (y/N):")?;
        let techniques = parse_techniques(&chain)?;
        let transformed = self.pipeline.encode(&message, &techniques)?;

        let mut key_pair = None;
        let mut secure_payload = None;
        if secure.to_lowercase().starts_with('y') {
            let passphrase = self.ui.prompt("Primary passphrase:")?;
            let generated = self.secure_envelope.make_key_pair(&passphrase);
            secure_payload = Some(self.secure_envelope.seal(&transformed, &generated)?);
            key_pair = Some(generated);
        }

        let result = EncodedMessage {
            original_input: message,
            transformed_text: transformed,
            secure_payload,
            techniques,
            used_secure_envelope: key_pair.is_some(),
            key_pair,
        };

        self.history_store
            .append(history_entry_for(&result, OperationKind::Encode))?;
        self.show_encode_result(&result)?;
        Ok(())
    }

    fn run_decode_flow(&self) -> Result<()> {
        self.ui.clear_screen();
        self.ui.print_panel(
            "Decode",
            "Enter the same technique chain used during encoding.\nIf the message was wrapped in a secure envelope, provide both key parts.",
        );

        let secure_wrapped = self.ui.prompt("Is this a secure payload? (y/N):")?;
        let raw_input = if secure_wrapped.to_lowercase().starts_with('y') {
            let payload = self.ui.prompt("Secure payload:")?;
            let passphrase = self.ui.prompt("Primary passphrase:")?;
            let companion_code = self.ui.prompt("Companion code:")?;
            self.secure_envelope.open(
                &payload,
                &SecureKeyPair {
                    passphrase,
                    companion_code,
                },
            )?
        } else {
            self.ui.prompt("Cipher text:")?
        };

        let techniques = parse_techniques(&self.ui.prompt("Technique chain:")?)?;
        let decoded = self.pipeline.decode(&raw_input, &techniques)?;

        let result = EncodedMessage {
            original_input: raw_input,
            transformed_text: decoded.clone(),
            secure_payload: None,
            techniques,
            used_secure_envelope: secure_wrapped.to_lowercase().starts_with('y'),
            key_pair: None,
        };

        self.history_store
            .append(history_entry_for(&result, OperationKind::Decode))?;
        self.ui.print_panel("Decoded", &decoded);
        self.ui.pause()?;
        Ok(())
    }

    fn show_history(&self) -> Result<()> {
        self.ui.clear_screen();
        let entries = self.history_store.load()?;
        if entries.is_empty() {
            self.ui
                .print_panel("History", "No entries yet. Encode or decode a message first.");
        } else {
            let body = entries
                .iter()
                .take(12)
                .enumerate()
                .map(|(idx, entry)| {
                    let techniques = entry
                        .techniques
                        .iter()
                        .map(TechniqueDescriptor::title)
                        .collect::<Vec<_>>()
                        .join(" -> ");
                    format!(
                        "{:>2}. {} [{:?}]\n    in: {}\n    out: {}\n    chain: {}\n    secure: {}",
                        idx + 1,
                        entry.timestamp.to_rfc3339(),
                        entry.operation,
                        entry.input_preview,
                        entry.output_preview,
                        techniques,
                        if entry.used_secure_envelope { "yes" } else { "no" }
                    )
                })
                .collect::<Vec<_>>()
                .join("\n\n");
            self.ui.print_panel("Encrypted History", &body);
        }
        self.ui.pause()?;
        Ok(())
    }

    fn show_settings(&self) -> Result<()> {
        self.ui.clear_screen();
        self.ui.print_panel(
            "Settings",
            &format!(
                "github oauth: planned\nupdate channel: planned\nfile hiding / carrier mode: planned\n\nlocal encrypted history:\n{}\n\nnote:\nthis rust branch is aimed at future cross-platform parity. the next production step is moving local secrets into os key stores per platform.",
                self.history_store.diagnostics()
            ),
        );
        self.ui.pause()?;
        Ok(())
    }

    fn show_roadmap(&self) -> Result<()> {
        self.ui.clear_screen();
        self.ui.print_panel(
            "Roadmap",
            "Phase 1\n- offline terminal workbench in Rust\n- stacked ciphers + secure paired-key envelope\n- encrypted local history\n\nPhase 2\n- SmileOS-like GUI skin with egui or Tauri shell\n- drag-and-drop file carrier workflows\n- export/import key bundles\n\nPhase 3\n- GitHub OAuth in settings\n- signed release packaging per platform\n- updater behavior for macOS, Linux, Windows, Android, and FreeBSD release channels",
        );
        self.ui.pause()?;
        Ok(())
    }

    fn show_encode_result(&self, result: &EncodedMessage) -> Result<()> {
        let mut lines = vec![
            "Transformed text:".to_string(),
            result.transformed_text.clone(),
        ];

        if let (Some(payload), Some(key_pair)) = (&result.secure_payload, &result.key_pair) {
            lines.push(String::new());
            lines.push("Secure payload:".to_string());
            lines.push(payload.clone());
            lines.push(String::new());
            lines.push("Share these separately:".to_string());
            lines.push(format!("Primary passphrase: {}", key_pair.passphrase));
            lines.push(format!("Companion code: {}", key_pair.companion_code));
        }

        self.ui.print_panel("Encoded", &lines.join("\n"));
        self.ui.pause()?;
        Ok(())
    }
}

fn parse_techniques(input: &str) -> Result<Vec<TechniqueDescriptor>> {
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

fn history_entry_for(result: &EncodedMessage, operation: OperationKind) -> HistoryEntry {
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

fn truncate(value: &str) -> String {
    let cleaned = value.replace('\n', " ");
    if cleaned.chars().count() > 80 {
        format!("{}...", cleaned.chars().take(77).collect::<String>())
    } else {
        cleaned
    }
}

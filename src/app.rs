use anyhow::Result;

use crate::{core::SighFarCore, models::TechniqueDescriptor, ui::TerminalUi};

pub struct SighFarApp {
    ui: TerminalUi,
    core: SighFarCore,
}

impl Default for SighFarApp {
    fn default() -> Self {
        Self {
            ui: TerminalUi,
            core: SighFarCore::default(),
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
            "Stack one or more techniques in order.\nAvailable: morse, caesar, vigenere, railfence, reverse, sha256, sha512\nExample: caesar:4,reverse,sha256",
        );

        let message = self.ui.prompt("Message:")?;
        let chain = self.ui.prompt("Technique chain:")?;
        let secure = self.ui.prompt("Wrap in secure paired-key envelope? (y/N):")?;
        let passphrase = if secure.to_lowercase().starts_with('y') {
            Some(self.ui.prompt("Primary passphrase:")?)
        } else {
            None
        };
        let result = self
            .core
            .encode(&message, &chain, secure.to_lowercase().starts_with('y'), passphrase.as_deref())?;
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
        let input = if secure_wrapped.to_lowercase().starts_with('y') {
            self.ui.prompt("Secure payload:")?
        } else {
            self.ui.prompt("Cipher text:")?
        };
        let passphrase = if secure_wrapped.to_lowercase().starts_with('y') {
            Some(self.ui.prompt("Primary passphrase:")?)
        } else {
            None
        };
        let companion_code = if secure_wrapped.to_lowercase().starts_with('y') {
            Some(self.ui.prompt("Companion code:")?)
        } else {
            None
        };
        let chain = self.ui.prompt("Technique chain:")?;
        let result = self.core.decode(
            &input,
            &chain,
            secure_wrapped.to_lowercase().starts_with('y'),
            passphrase.as_deref(),
            companion_code.as_deref(),
        )?;
        self.ui.print_panel("Decoded", &result.transformed_text);
        self.ui.pause()?;
        Ok(())
    }

    fn show_history(&self) -> Result<()> {
        self.ui.clear_screen();
        let entries = self.core.load_history()?;
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
                "github oauth: configure via the GUI settings tab\nfile carrier mode: use the GUI carrier tab\n\nlocal encrypted history:\n{}",
                self.core.diagnostics()
            ),
        );
        self.ui.pause()?;
        Ok(())
    }

    fn show_encode_result(&self, result: &crate::models::EncodedMessage) -> Result<()> {
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

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OperationKind {
    Encode,
    Decode,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecureKeyPair {
    pub passphrase: String,
    pub companion_code: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EncodedMessage {
    pub original_input: String,
    pub transformed_text: String,
    pub secure_payload: Option<String>,
    pub techniques: Vec<TechniqueDescriptor>,
    pub used_secure_envelope: bool,
    pub key_pair: Option<SecureKeyPair>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HistoryEntry {
    pub id: String,
    pub timestamp: DateTime<Utc>,
    pub operation: OperationKind,
    pub input_preview: String,
    pub output_preview: String,
    pub techniques: Vec<TechniqueDescriptor>,
    pub used_secure_envelope: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum TechniqueDescriptor {
    Morse,
    Caesar { shift: i32 },
    Vigenere { keyword: String },
    RailFence { rails: usize },
    Reverse,
}

impl TechniqueDescriptor {
    pub fn title(&self) -> String {
        match self {
            Self::Morse => "Morse".to_string(),
            Self::Caesar { shift } => format!("Caesar({shift})"),
            Self::Vigenere { keyword } => format!("Vigenere({keyword})"),
            Self::RailFence { rails } => format!("RailFence({rails})"),
            Self::Reverse => "Reverse".to_string(),
        }
    }
}

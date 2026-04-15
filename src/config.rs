use std::{fs, path::{Path, PathBuf}};

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct AppConfig {
    pub github_client_id: String,
    pub github_repo_name: String,
}

pub struct ConfigStore {
    file_path: PathBuf,
}

impl Default for ConfigStore {
    fn default() -> Self {
        Self {
            file_path: default_support_directory().join("config.json"),
        }
    }
}

impl ConfigStore {
    pub fn load(&self) -> Result<AppConfig> {
        if !self.file_path.exists() {
            return Ok(AppConfig {
                github_client_id: String::new(),
                github_repo_name: "sighfar-secure-sync".to_string(),
            });
        }
        let bytes = fs::read(&self.file_path)
            .with_context(|| format!("failed to read config file: {}", self.file_path.display()))?;
        serde_json::from_slice(&bytes).context("failed to decode app config")
    }

    pub fn save(&self, config: &AppConfig) -> Result<()> {
        if let Some(parent) = self.file_path.parent() {
            fs::create_dir_all(parent)
                .with_context(|| format!("failed to create config directory: {}", parent.display()))?;
        }
        let bytes = serde_json::to_vec_pretty(config).context("failed to encode app config")?;
        fs::write(&self.file_path, bytes)
            .with_context(|| format!("failed to write config file: {}", self.file_path.display()))
    }
}

fn default_support_directory() -> PathBuf {
    std::env::var_os("HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|| Path::new(".").to_path_buf())
        .join(".sighfar")
}

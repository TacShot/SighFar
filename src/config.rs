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

#[cfg(test)]
mod tests {
    use std::time::{SystemTime, UNIX_EPOCH};

    use super::{AppConfig, ConfigStore};

    fn unique_store(label: &str) -> ConfigStore {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let path = std::env::temp_dir()
            .join(format!("sighfar-config-{label}-{unique}"))
            .join("config.json");
        ConfigStore { file_path: path }
    }

    #[test]
    fn load_returns_default_when_no_file() {
        let store = unique_store("nofile");
        let config = store.load().unwrap();
        assert_eq!(config.github_client_id, "");
        assert_eq!(config.github_repo_name, "sighfar-secure-sync");
    }

    #[test]
    fn save_and_load_round_trip() {
        let store = unique_store("roundtrip");
        let config = AppConfig {
            github_client_id: "my-client-id".to_string(),
            github_repo_name: "my-repo".to_string(),
        };
        store.save(&config).unwrap();
        let loaded = store.load().unwrap();
        assert_eq!(loaded.github_client_id, "my-client-id");
        assert_eq!(loaded.github_repo_name, "my-repo");
    }

    #[test]
    fn save_creates_parent_directories() {
        let store = unique_store("mkdirs");
        let config = AppConfig::default();
        // Parent dir does not exist yet; save should create it
        assert!(!store.file_path.parent().unwrap().exists());
        store.save(&config).unwrap();
        assert!(store.file_path.exists());
    }

    #[test]
    fn load_fails_on_corrupted_json() {
        let store = unique_store("corrupt");
        std::fs::create_dir_all(store.file_path.parent().unwrap()).unwrap();
        std::fs::write(&store.file_path, b"not valid json {{{{").unwrap();
        let result = store.load();
        assert!(result.is_err());
    }

    #[test]
    fn save_overwrites_existing_config() {
        let store = unique_store("overwrite");
        let first = AppConfig {
            github_client_id: "first".to_string(),
            github_repo_name: "repo-first".to_string(),
        };
        store.save(&first).unwrap();

        let second = AppConfig {
            github_client_id: "second".to_string(),
            github_repo_name: "repo-second".to_string(),
        };
        store.save(&second).unwrap();

        let loaded = store.load().unwrap();
        assert_eq!(loaded.github_client_id, "second");
        assert_eq!(loaded.github_repo_name, "repo-second");
    }
}

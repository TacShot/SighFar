use std::{thread, time::{Duration, SystemTime}};

use anyhow::{Context, Result, anyhow, bail};
use base64::{Engine as _, engine::general_purpose::STANDARD as BASE64};
use reqwest::blocking::Client;
use serde::{Deserialize, Serialize};

const DEVICE_CODE_URL: &str = "https://github.com/login/device/code";
const ACCESS_TOKEN_URL: &str = "https://github.com/login/oauth/access_token";
const API_ROOT: &str = "https://api.github.com";
const API_VERSION: &str = "2022-11-28";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeviceAuthState {
    pub device_code: String,
    pub user_code: String,
    pub verification_uri: String,
    pub expires_in: u64,
    pub interval: u64,
    pub created_at_epoch: u64,
}

#[derive(Debug, Clone)]
pub struct GitHubSession {
    pub access_token: String,
    pub username: String,
}

#[derive(Debug, Clone)]
pub enum DevicePollStatus {
    Authorized(GitHubSession),
    Pending(String),
}

pub struct GitHubSyncClient {
    client: Client,
}

impl Default for GitHubSyncClient {
    fn default() -> Self {
        let client = Client::builder()
            .user_agent("SighFar/0.1")
            .build()
            .expect("GitHub HTTP client should build");
        Self { client }
    }
}

impl GitHubSyncClient {
    pub fn start_device_flow(&self, client_id: &str) -> Result<DeviceAuthState> {
        let response = self.client
            .post(DEVICE_CODE_URL)
            .header("Accept", "application/json")
            .form(&[("client_id", client_id), ("scope", "repo")])
            .send()
            .context("failed to contact GitHub device authorization endpoint")?;

        if !response.status().is_success() {
            bail!("device authorization failed with status {}", response.status());
        }

        let payload: DeviceCodeResponse = response.json().context("failed to decode device authorization response")?;
        Ok(DeviceAuthState {
            device_code: payload.device_code,
            user_code: payload.user_code,
            verification_uri: payload.verification_uri,
            expires_in: payload.expires_in,
            interval: payload.interval.max(5),
            created_at_epoch: now_epoch(),
        })
    }

    pub fn poll_device_flow(&self, client_id: &str, device: &DeviceAuthState) -> Result<DevicePollStatus> {
        if now_epoch() > device.created_at_epoch + device.expires_in {
            bail!("device code expired, start sign-in again");
        }

        let response = self.client
            .post(ACCESS_TOKEN_URL)
            .header("Accept", "application/json")
            .form(&[
                ("client_id", client_id),
                ("device_code", device.device_code.as_str()),
                ("grant_type", "urn:ietf:params:oauth:grant-type:device_code"),
            ])
            .send()
            .context("failed to poll GitHub for device authorization status")?;

        if !response.status().is_success() {
            bail!("device token poll failed with status {}", response.status());
        }

        let payload: AccessTokenResponse = response.json().context("failed to decode access token response")?;
        if let Some(error) = payload.error {
            return Ok(DevicePollStatus::Pending(match error.as_str() {
                "authorization_pending" => "Waiting for GitHub authorization.".to_string(),
                "slow_down" => {
                    thread::sleep(Duration::from_secs(device.interval + 5));
                    "GitHub asked the app to slow down polling.".to_string()
                }
                "expired_token" => "Device code expired, start sign-in again.".to_string(),
                other => format!("GitHub returned device flow error: {other}"),
            }));
        }

        let access_token = payload
            .access_token
            .ok_or_else(|| anyhow!("GitHub did not return an access token"))?;
        let username = self.fetch_authenticated_username(&access_token)?;
        Ok(DevicePollStatus::Authorized(GitHubSession {
            access_token,
            username,
        }))
    }

    pub fn ensure_private_repo(&self, session: &GitHubSession, repo_name: &str) -> Result<String> {
        let existing = self
            .client
            .get(format!("{API_ROOT}/repos/{}/{}", session.username, repo_name))
            .bearer_auth(&session.access_token)
            .header("Accept", "application/vnd.github+json")
            .header("X-GitHub-Api-Version", API_VERSION)
            .send()
            .context("failed to check sync repository")?;

        if existing.status().is_success() {
            return Ok(format!("{}/{}", session.username, repo_name));
        }
        if existing.status().as_u16() != 404 {
            bail!("failed to inspect sync repository: {}", existing.status());
        }

        let response = self
            .client
            .post(format!("{API_ROOT}/user/repos"))
            .bearer_auth(&session.access_token)
            .header("Accept", "application/vnd.github+json")
            .header("X-GitHub-Api-Version", API_VERSION)
            .json(&serde_json::json!({
                "name": repo_name,
                "private": true,
                "description": "Encrypted SighFar sync storage",
                "auto_init": false
            }))
            .send()
            .context("failed to create private sync repository")?;

        if !response.status().is_success() {
            bail!("GitHub repository creation failed with status {}", response.status());
        }

        Ok(format!("{}/{}", session.username, repo_name))
    }

    pub fn push_history_blob(&self, session: &GitHubSession, repo_name: &str, blob: &[u8]) -> Result<()> {
        self.ensure_private_repo(session, repo_name)?;
        let path = "history/history.enc";
        let existing_sha = self.get_repo_content_sha(session, repo_name, path)?;

        let response = self
            .client
            .put(format!("{API_ROOT}/repos/{}/{}/contents/{}", session.username, repo_name, path))
            .bearer_auth(&session.access_token)
            .header("Accept", "application/vnd.github+json")
            .header("X-GitHub-Api-Version", API_VERSION)
            .json(&serde_json::json!({
                "message": "Sync encrypted SighFar history",
                "content": BASE64.encode(blob),
                "sha": existing_sha
            }))
            .send()
            .context("failed to upload encrypted history to GitHub")?;

        if !response.status().is_success() {
            bail!("GitHub history upload failed with status {}", response.status());
        }
        Ok(())
    }

    pub fn pull_history_blob(&self, session: &GitHubSession, repo_name: &str) -> Result<Vec<u8>> {
        let response = self
            .client
            .get(format!("{API_ROOT}/repos/{}/{}/contents/history/history.enc", session.username, repo_name))
            .bearer_auth(&session.access_token)
            .header("Accept", "application/vnd.github+json")
            .header("X-GitHub-Api-Version", API_VERSION)
            .send()
            .context("failed to download encrypted history from GitHub")?;

        if response.status().as_u16() == 404 {
            bail!("no encrypted history file exists in the sync repository yet");
        }
        if !response.status().is_success() {
            bail!("GitHub history download failed with status {}", response.status());
        }

        let payload: ContentResponse = response.json().context("failed to decode GitHub contents response")?;
        let normalized = payload.content.replace('\n', "");
        BASE64
            .decode(normalized)
            .context("failed to decode GitHub history blob")
    }

    fn get_repo_content_sha(&self, session: &GitHubSession, repo_name: &str, path: &str) -> Result<Option<String>> {
        let response = self
            .client
            .get(format!("{API_ROOT}/repos/{}/{}/contents/{}", session.username, repo_name, path))
            .bearer_auth(&session.access_token)
            .header("Accept", "application/vnd.github+json")
            .header("X-GitHub-Api-Version", API_VERSION)
            .send()
            .context("failed to inspect GitHub file content")?;

        if response.status().as_u16() == 404 {
            return Ok(None);
        }
        if !response.status().is_success() {
            bail!("failed to inspect remote history file: {}", response.status());
        }

        let payload: ContentResponse = response.json().context("failed to decode content metadata")?;
        Ok(Some(payload.sha))
    }

    fn fetch_authenticated_username(&self, token: &str) -> Result<String> {
        let response = self
            .client
            .get(format!("{API_ROOT}/user"))
            .bearer_auth(token)
            .header("Accept", "application/vnd.github+json")
            .header("X-GitHub-Api-Version", API_VERSION)
            .send()
            .context("failed to fetch authenticated GitHub user")?;

        if !response.status().is_success() {
            bail!("GitHub user lookup failed with status {}", response.status());
        }

        let payload: UserResponse = response.json().context("failed to decode GitHub user profile")?;
        Ok(payload.login)
    }
}

#[derive(Debug, Deserialize)]
struct DeviceCodeResponse {
    device_code: String,
    user_code: String,
    verification_uri: String,
    expires_in: u64,
    interval: u64,
}

#[derive(Debug, Deserialize)]
struct AccessTokenResponse {
    access_token: Option<String>,
    error: Option<String>,
}

#[derive(Debug, Deserialize)]
struct UserResponse {
    login: String,
}

#[derive(Debug, Deserialize)]
struct ContentResponse {
    sha: String,
    #[serde(default)]
    content: String,
}

fn now_epoch() -> u64 {
    SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

// SPDX-License-Identifier: MIT

use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

/// Per-profile configuration fields.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Profile {
    pub aws_region: String,
    #[serde(default)]
    pub cognito_domain: String,
    #[serde(default)]
    pub cognito_app_client_id: String,
    #[serde(default)]
    pub cognito_identity_pool_id: String,
    #[serde(default)]
    pub cognito_user_pool_id: String,
    #[serde(default)]
    pub sqs_queue_url: String,
    #[serde(default)]
    pub api_function_arn: String,
    #[serde(default)]
    pub migration_function_arn: String,
}

/// Top-level config stored in `~/.config/rat/config.toml`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RatConfig {
    pub default: Profile,
    #[serde(default)]
    pub profiles: HashMap<String, Profile>,
}

/// Return `~/.config/rat/`.
pub fn config_dir() -> Result<PathBuf> {
    let home = dirs::home_dir().context("cannot determine home directory")?;
    Ok(home.join(".config").join("rat"))
}

/// Return `~/.config/rat/config.toml`.
pub fn config_path() -> Result<PathBuf> {
    Ok(config_dir()?.join("config.toml"))
}

/// Load config from disk. Returns `None` if the file does not exist.
pub fn load_config() -> Result<Option<RatConfig>> {
    let path = config_path()?;
    if !path.exists() {
        return Ok(None);
    }
    let text = fs::read_to_string(&path)
        .with_context(|| format!("failed to read {}", path.display()))?;
    let cfg: RatConfig =
        toml::from_str(&text).with_context(|| format!("failed to parse {}", path.display()))?;
    Ok(Some(cfg))
}

/// Save config to disk, creating parent directories as needed.
pub fn save_config(cfg: &RatConfig) -> Result<()> {
    let path = config_path()?;
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("failed to create {}", parent.display()))?;
    }
    let text = toml::to_string_pretty(cfg).context("failed to serialize config")?;
    fs::write(&path, text)
        .with_context(|| format!("failed to write {}", path.display()))?;
    Ok(())
}

/// Resolve a profile by name. `None` means the default profile.
pub fn resolve_profile(cfg: &RatConfig, name: Option<&str>) -> Option<Profile> {
    match name {
        None | Some("default") => Some(cfg.default.clone()),
        Some(n) => cfg.profiles.get(n).cloned(),
    }
}

// ── Credentials (tokens) ──

/// OAuth token set stored per profile in `~/.config/rat/credentials.json`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenSet {
    pub id_token: String,
    pub access_token: String,
    pub refresh_token: String,
    pub expires_at: i64,
}

/// Return `~/.config/rat/credentials.toml`.
pub fn credentials_path() -> Result<PathBuf> {
    Ok(config_dir()?.join("credentials.toml"))
}

/// Load all credentials. Returns empty map if file does not exist.
pub fn load_credentials() -> Result<HashMap<String, TokenSet>> {
    let path = credentials_path()?;
    if !path.exists() {
        return Ok(HashMap::new());
    }
    let text = fs::read_to_string(&path)
        .with_context(|| format!("failed to read {}", path.display()))?;
    let creds: HashMap<String, TokenSet> =
        toml::from_str(&text).with_context(|| format!("failed to parse {}", path.display()))?;
    Ok(creds)
}

/// Save credentials to disk with 0600 permissions.
pub fn save_credentials(creds: &HashMap<String, TokenSet>) -> Result<()> {
    let path = credentials_path()?;
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("failed to create {}", parent.display()))?;
    }
    let text = toml::to_string_pretty(creds).context("failed to serialize credentials")?;
    fs::write(&path, &text)
        .with_context(|| format!("failed to write {}", path.display()))?;

    // Set file permissions to 0600 (owner read/write only)
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(&path, fs::Permissions::from_mode(0o600))?;
    }

    Ok(())
}

/// Load token set for a specific profile.
pub fn load_token(profile_name: Option<&str>) -> Result<Option<TokenSet>> {
    let key = profile_name.unwrap_or("default");
    let creds = load_credentials()?;
    Ok(creds.get(key).cloned())
}

/// Load token, refreshing automatically if expired.
/// Returns `None` if no token is stored.
pub async fn load_valid_token(
    profile: &Profile,
    profile_name: Option<&str>,
) -> Result<Option<TokenSet>> {
    let token = match load_token(profile_name)? {
        Some(t) => t,
        None => return Ok(None),
    };

    let now = chrono::Utc::now().timestamp();
    // Refresh if token expires within 60 seconds
    if now < token.expires_at - 60 {
        return Ok(Some(token));
    }

    eprintln!("Token expired, refreshing...");
    let refreshed = crate::auth::refresh_token(profile, &token.refresh_token).await?;
    save_token(profile_name, &refreshed)?;
    Ok(Some(refreshed))
}

/// Save token set for a specific profile.
pub fn save_token(profile_name: Option<&str>, token: &TokenSet) -> Result<()> {
    let key = profile_name.unwrap_or("default").to_string();
    let mut creds = load_credentials()?;
    creds.insert(key, token.clone());
    save_credentials(&creds)
}

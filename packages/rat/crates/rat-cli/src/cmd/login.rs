// SPDX-License-Identifier: MIT

use anyhow::{bail, Result};
use crate::auth;
use crate::config;

pub async fn handle(profile_name: Option<&str>, status: bool) -> Result<()> {
    let cfg = config::load_config()?;
    let Some(cfg) = cfg else {
        bail!("No configuration found. Run `rat configure` first.");
    };

    let label = profile_name.unwrap_or("default");
    let Some(profile) = config::resolve_profile(&cfg, profile_name) else {
        bail!("Profile '{}' not found.", label);
    };

    if status {
        return show_status(profile_name);
    }

    eprintln!("Logging in (profile: {})...", label);
    let token = auth::login_pkce(&profile).await?;
    config::save_token(profile_name, &token)?;
    eprintln!("Login successful. Credentials saved.");
    Ok(())
}

fn show_status(profile_name: Option<&str>) -> Result<()> {
    let label = profile_name.unwrap_or("default");
    let token = config::load_token(profile_name)?;

    match token {
        None => {
            eprintln!("[{}] Not logged in.", label);
        }
        Some(t) => {
            let now = chrono::Utc::now().timestamp();
            if now < t.expires_at {
                let remaining = t.expires_at - now;
                let mins = remaining / 60;
                eprintln!("[{}] Logged in. Token expires in {} min.", label, mins);
            } else {
                eprintln!("[{}] Token expired. Run `rat login` to re-authenticate.", label);
            }
        }
    }
    Ok(())
}

pub fn logout(profile_name: Option<&str>) -> Result<()> {
    let label = profile_name.unwrap_or("default");
    let mut creds = config::load_credentials()?;
    if creds.remove(label).is_some() {
        config::save_credentials(&creds)?;
        eprintln!("[{}] Logged out.", label);
    } else {
        eprintln!("[{}] No credentials found.", label);
    }
    Ok(())
}

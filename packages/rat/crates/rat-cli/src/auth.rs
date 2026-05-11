// SPDX-License-Identifier: MIT

use std::io::{BufRead, BufReader, Write};
use std::net::TcpListener;

use anyhow::{bail, Context, Result};
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use base64::Engine;
use rand::RngExt;
use sha2::{Digest, Sha256};

use crate::config::{Profile, TokenSet};

const CALLBACK_PORT: u16 = 9876;
const REDIRECT_URI: &str = "http://localhost:9876/callback";
const REDIRECT_URI_ENCODED: &str = "http%3A%2F%2Flocalhost%3A9876%2Fcallback";

/// Generate a random 32-byte code_verifier (base64url-encoded).
fn generate_code_verifier() -> String {
    let random_bytes: Vec<u8> = (0..32).map(|_| rand::rng().random::<u8>()).collect();
    URL_SAFE_NO_PAD.encode(&random_bytes)
}

/// Derive code_challenge = BASE64URL(SHA256(code_verifier)).
fn code_challenge(verifier: &str) -> String {
    let digest = Sha256::digest(verifier.as_bytes());
    URL_SAFE_NO_PAD.encode(digest)
}

/// Build the Cognito authorize URL.
fn authorize_url(profile: &Profile, challenge: Option<&str>) -> String {
    let base = format!(
        "https://{domain}/oauth2/authorize\
         ?response_type=code\
         &client_id={client_id}\
         &redirect_uri={redirect_uri}\
         &scope=openid%20profile%20email",
        domain = profile.cognito_domain,
        client_id = profile.cognito_app_client_id,
        redirect_uri = REDIRECT_URI_ENCODED,
    );
    match challenge {
        Some(c) => format!("{}&code_challenge={}&code_challenge_method=S256", base, c),
        None => base,
    }
}

/// Run the full browser-based PKCE login flow and return a TokenSet.
pub async fn login_pkce(profile: &Profile) -> Result<TokenSet> {
    let verifier = generate_code_verifier();
    let challenge = code_challenge(&verifier);
    let auth_url = authorize_url(profile, Some(&challenge));

    // Bind the callback server BEFORE opening the browser
    let listener = TcpListener::bind(("127.0.0.1", CALLBACK_PORT))
        .context("Port 9876 is already in use. Please try again later.")?;

    eprintln!("Opening browser for authentication...");
    eprintln!("URL: {}", auth_url);
    open::that(&auth_url).context("failed to open browser")?;

    // Wait for the callback (single request)
    let (mut stream, _) = listener.accept().context("failed to accept callback")?;

    let reader = BufReader::new(&stream);
    let request_line = reader
        .lines()
        .next()
        .context("no request received")?
        .context("failed to read request")?;

    // Parse "GET /callback?code=AUTH_CODE HTTP/1.1"
    let auth_code = parse_code_from_request(&request_line)?;

    // Respond with a success page, then close
    let html = "<html><body><h2>Authentication complete!</h2><p>You can close this window.</p></body></html>";
    let response = format!(
        "HTTP/1.1 200 OK\r\nContent-Type: text/html; charset=utf-8\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
        html.len(),
        html
    );
    let _ = stream.write_all(response.as_bytes());

    // Exchange code for tokens
    let token = exchange_code(profile, &auth_code, &verifier).await?;
    Ok(token)
}

/// Extract the `code` query parameter from the HTTP request line.
fn parse_code_from_request(request_line: &str) -> Result<String> {
    // "GET /callback?code=XXXX&... HTTP/1.1"
    let path = request_line
        .split_whitespace()
        .nth(1)
        .context("malformed HTTP request")?;

    let query = path
        .split('?')
        .nth(1)
        .context("no query string in callback")?;

    for pair in query.split('&') {
        if let Some(value) = pair.strip_prefix("code=") {
            return Ok(value.to_string());
        }
    }

    // Check for error
    for pair in query.split('&') {
        if let Some(value) = pair.strip_prefix("error=") {
            bail!("authentication error: {}", value);
        }
    }

    bail!("no authorization code in callback")
}

/// Token endpoint response from Cognito.
#[derive(serde::Deserialize)]
struct TokenResponse {
    id_token: String,
    access_token: String,
    refresh_token: String,
    expires_in: i64,
}

/// Exchange authorization code for tokens via the Cognito token endpoint.
async fn exchange_code(profile: &Profile, code: &str, verifier: &str) -> Result<TokenSet> {
    let token_url = format!("https://{}/oauth2/token", profile.cognito_domain);

    let client = reqwest::Client::new();
    let resp = client
        .post(&token_url)
        .form(&[
            ("grant_type", "authorization_code"),
            ("client_id", &profile.cognito_app_client_id),
            ("code", code),
            ("redirect_uri", REDIRECT_URI),
            ("code_verifier", verifier),
        ])
        .send()
        .await
        .context("failed to call token endpoint")?;

    if !resp.status().is_success() {
        let status = resp.status();
        let body = resp.text().await.unwrap_or_default();
        bail!("token endpoint returned {}: {}", status, body);
    }

    let token_resp: TokenResponse = resp.json().await.context("failed to parse token response")?;
    let expires_at = chrono::Utc::now().timestamp() + token_resp.expires_in;

    Ok(TokenSet {
        id_token: token_resp.id_token,
        access_token: token_resp.access_token,
        refresh_token: token_resp.refresh_token,
        expires_at,
    })
}

/// Refresh tokens using the refresh_token grant.
pub async fn refresh_token(profile: &Profile, refresh_tok: &str) -> Result<TokenSet> {
    let token_url = format!("https://{}/oauth2/token", profile.cognito_domain);

    let client = reqwest::Client::new();
    let resp = client
        .post(&token_url)
        .form(&[
            ("grant_type", "refresh_token"),
            ("client_id", &profile.cognito_app_client_id),
            ("refresh_token", refresh_tok),
        ])
        .send()
        .await
        .context("failed to call token endpoint")?;

    if !resp.status().is_success() {
        let status = resp.status();
        let body = resp.text().await.unwrap_or_default();
        bail!("token refresh failed {}: {}", status, body);
    }

    // Cognito refresh response doesn't include refresh_token; keep the old one
    #[derive(serde::Deserialize)]
    struct RefreshResponse {
        id_token: String,
        access_token: String,
        expires_in: i64,
    }

    let r: RefreshResponse = resp.json().await.context("failed to parse refresh response")?;
    let expires_at = chrono::Utc::now().timestamp() + r.expires_in;

    Ok(TokenSet {
        id_token: r.id_token,
        access_token: r.access_token,
        refresh_token: refresh_tok.to_string(),
        expires_at,
    })
}

// Phase 6 — Google OAuth 2.0 PKCE installed-app flow.
// Spec §13 row 6 + §14. PKCE means no client_secret ships in the binary;
// user registers their own OAuth client (type: Web application,
// redirect URI: http://127.0.0.1:<port>/callback) and sets the client_id
// constant below before building.

use crate::error::{AppError, AppResult};
use base64::Engine;
use rand::RngCore;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::io::{Read, Write};
use std::net::TcpListener;
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

/// EDIT THIS for your own OAuth client. See README "Google Calendar setup".
/// PKCE eliminates the need for a client_secret counterpart.
pub const OAUTH_CLIENT_ID: &str = "REPLACE_WITH_YOUR_CLIENT_ID.apps.googleusercontent.com";
pub const OAUTH_AUTH_URL: &str = "https://accounts.google.com/o/oauth2/v2/auth";
pub const OAUTH_TOKEN_URL: &str = "https://oauth2.googleapis.com/token";
pub const OAUTH_SCOPES: &str = "https://www.googleapis.com/auth/calendar.events";

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct StoredToken {
    pub access_token: String,
    pub refresh_token: String,
    pub expires_at: i64,
}

/// Generate a PKCE verifier (43–128 chars) and its S256 challenge.
pub fn pkce_pair() -> (String, String) {
    let mut buf = [0u8; 32];
    rand::thread_rng().fill_bytes(&mut buf);
    let verifier = base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(buf);
    let mut hasher = Sha256::new();
    hasher.update(verifier.as_bytes());
    let challenge = base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(hasher.finalize());
    (verifier, challenge)
}

/// Start a localhost listener on 127.0.0.1, build the Google auth URL with the
/// verifier challenge + matching redirect URI, open the user's browser, and
/// return the captured authorization code (also returns the verifier so the
/// caller can pass it to `exchange_code`).
pub async fn authorize(http: &reqwest::Client) -> AppResult<(String, String, String)> {
    let (verifier, challenge) = pkce_pair();
    let listener = TcpListener::bind("127.0.0.1:0")
        .map_err(|e| AppError::Auth(format!("bind: {e}")))?;
    let port = listener.local_addr().map_err(|e| AppError::Auth(format!("addr: {e}")))?.port();
    let redirect_uri = format!("http://127.0.0.1:{port}/callback");
    let url = format!(
        "{auth}?client_id={cid}&redirect_uri={ru}&response_type=code&scope={sc}&code_challenge={ch}&code_challenge_method=S256&access_type=offline&prompt=consent",
        auth = OAUTH_AUTH_URL,
        cid = urlencoding(OAUTH_CLIENT_ID),
        ru = urlencoding(&redirect_uri),
        sc = urlencoding(OAUTH_SCOPES),
        ch = urlencoding(&challenge),
    );

    let _ = open_browser(&url);

    listener.set_nonblocking(false).ok();
    let (mut stream, _) = listener.accept().map_err(|e| AppError::Auth(format!("accept: {e}")))?;
    stream.set_read_timeout(Some(std::time::Duration::from_secs(5))).ok();
    let mut req = [0u8; 4096];
    let n = stream.read(&mut req).map_err(|e| AppError::Auth(format!("read: {e}")))?;
    let req_str = String::from_utf8_lossy(&req[..n]);
    let path = req_str.lines().next().and_then(|l| l.split_whitespace().nth(1)).unwrap_or("");
    let code = parse_code(path).ok_or_else(|| AppError::Auth("no code in redirect".into()))?;

    let html = "<html><body><h3>Notias authorized. You can close this tab.</h3></body></html>";
    let resp = format!(
        "HTTP/1.1 200 OK\r\nContent-Type: text/html; charset=utf-8\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
        html.len(), html
    );
    let _ = stream.write_all(resp.as_bytes());

    let _ = http; // accepted for future use; currently the listener does the work
    Ok((code, verifier, redirect_uri))
}

/// Exchange the authorization code + PKCE verifier for tokens.
pub async fn exchange_code(
    http: &reqwest::Client,
    code: &str,
    verifier: &str,
    redirect_uri: &str,
) -> AppResult<StoredToken> {
    let resp = http.post(OAUTH_TOKEN_URL)
        .form(&[
            ("client_id", OAUTH_CLIENT_ID),
            ("code", code),
            ("code_verifier", verifier),
            ("grant_type", "authorization_code"),
            ("redirect_uri", redirect_uri),
        ])
        .send().await.map_err(|e| AppError::Auth(format!("token post: {e}")))?;
    let token: TokenResponse = resp.error_for_status().map_err(|e| AppError::Auth(format!("token status: {e}")))?
        .json().await.map_err(|e| AppError::Auth(format!("token json: {e}")))?;
    let now = SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_secs() as i64).unwrap_or(0);
    Ok(StoredToken {
        access_token: token.access_token,
        refresh_token: token.refresh_token.unwrap_or_default(),
        expires_at: now + token.expires_in.unwrap_or(3600),
    })
}

/// Refresh an expired access token using the stored refresh token.
pub async fn refresh(http: &reqwest::Client, refresh_token: &str) -> AppResult<StoredToken> {
    let resp = http.post(OAUTH_TOKEN_URL)
        .form(&[
            ("client_id", OAUTH_CLIENT_ID),
            ("refresh_token", refresh_token),
            ("grant_type", "refresh_token"),
        ])
        .send().await.map_err(|e| AppError::Auth(format!("refresh post: {e}")))?;
    let token: TokenResponse = resp.error_for_status().map_err(|e| AppError::Auth(format!("refresh status: {e}")))?
        .json().await.map_err(|e| AppError::Auth(format!("refresh json: {e}")))?;
    let now = SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_secs() as i64).unwrap_or(0);
    Ok(StoredToken {
        access_token: token.access_token,
        refresh_token: refresh_token.into(),
        expires_at: now + token.expires_in.unwrap_or(3600),
    })
}

#[derive(Deserialize)]
struct TokenResponse {
    access_token: String,
    refresh_token: Option<String>,
    expires_in: Option<i64>,
}

fn parse_code(path: &str) -> Option<String> {
    let q = path.split('?').nth(1)?;
    for kv in q.split('&') {
        let mut it = kv.splitn(2, '=');
        let k = it.next()?;
        let v = it.next()?;
        if k == "code" { return Some(url_decode(v)); }
    }
    None
}

fn url_decode(s: &str) -> String {
    let mut out = Vec::with_capacity(s.len());
    let bytes = s.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%' && i + 2 < bytes.len() {
            if let Ok(b) = u8::from_str_radix(std::str::from_utf8(&bytes[i+1..i+3]).unwrap_or("00"), 16) {
                out.push(b);
                i += 3;
                continue;
            }
        }
        out.push(bytes[i]);
        i += 1;
    }
    String::from_utf8_lossy(&out).into_owned()
}

fn urlencoding(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for b in s.bytes() {
        match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'.' | b'_' | b'~' => out.push(b as char),
            _ => out.push_str(&format!("%{:02X}", b)),
        }
    }
    out
}

fn open_browser(url: &str) -> std::io::Result<()> {
    #[cfg(target_os = "windows")]
    { Command::new("cmd").args(["/C", "start", "", url]).spawn()?; }
    #[cfg(target_os = "macos")]
    { Command::new("open").arg(url).spawn()?; }
    #[cfg(target_os = "linux")]
    { Command::new("xdg-open").arg(url).spawn()?; }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pkce_pair_is_unique_and_well_formed() {
        let (v, c) = pkce_pair();
        assert!(v.len() >= 43);
        assert!(c.len() >= 43);
        let (v2, _) = pkce_pair();
        assert_ne!(v, v2, "verifier must differ across calls");
    }

    #[test]
    fn parse_code_extracts() {
        assert_eq!(parse_code("/callback?code=abc&scope=x"), Some("abc".into()));
        assert_eq!(parse_code("/callback?error=access_denied"), None);
    }

    #[test]
    fn url_encode_decode_roundtrip() {
        let s = "hello world/foo?bar=baz~";
        let e = urlencoding(s);
        assert_eq!(url_decode(&e), s);
    }
}
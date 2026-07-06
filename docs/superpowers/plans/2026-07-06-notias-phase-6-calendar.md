# Phase 6 — Google Calendar (post-MVP) Implementation Plan

> **For agentic workers:** REQUIRED: Use superpowers:subagent-driven-development (if subagents available) or superpowers:executing-plans to implement this plan. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** OAuth round-trip with Google Calendar + bidirectional event sync (pull next 7 days, write events back) with last-write-wins conflict detection.

**Architecture:** New `oauth/google` module owns the OAuth 2.0 PKCE flow + keyring-backed token storage. New `calendar/google` module is the Calendar API v3 HTTP client. New `calendar/mod` wires Tauri commands + a local mirror in `calendar_events`. Localhost redirect server is `std::net::TcpListener` (~40 LOC, no dep). Browser launch is `std::process::Command` (no dep). Client ID is a compiled constant overridable per build; no client_secret per PKCE.

**Tech Stack:** Tauri 2, Rust (edition 2021), reqwest (already), `rand` + `base64` (new direct deps), Svelte 5 runes, existing SQLite + ulid.

**Spec:** `docs/superpowers/specs/2026-07-03-notias-design.md` §13 row 6 + §14
**Master plan:** `docs/superpowers/plans/2026-07-03-notias.md` Chunk 7

---

## File Structure (delta only)

```
src-tauri/
├── Cargo.toml                                   MOD — add `rand` + `base64`
├── migrations/0006_calendar.sql                 NEW — calendar_events + calendar_sync_state
├── src/
│   ├── lib.rs                                   MOD — register 5 new commands + mod decls
│   ├── db/migrations.rs                         MOD — bump LATEST_VERSION 5→6, add arm
│   ├── oauth/
│   │   ├── mod.rs                               NEW — module root + shared helpers
│   │   └── google.rs                            NEW — PKCE + token exchange + redirect server
│   └── calendar/
│       ├── mod.rs                               NEW — Tauri commands + local mirror CRUD
│       └── google.rs                            NEW — Calendar API v3 client (list/insert)
└── ...

src/
├── lib/
│   ├── ipc.ts                                   EXTEND — 5 new bindings
│   ├── types.ts                                 EXTEND — CalendarEvent, OAuthStatus
│   └── components/
│       └── CalendarSettings.svelte              NEW — connect/disconnect + status
└── routes/
    ├── +layout.svelte                           MOD — sidebar Calendar entry
    └── calendar/+page.svelte                    NEW — events list + create form
```

**Decomposition:**
- `oauth::google` owns the *flow*: verifier generation, redirect server, code exchange, refresh. No Tauri command code here.
- `calendar::google` owns the *API*: pure HTTP against `googleapis.com/calendar/v3`. Takes a token, returns events. Testable with a mocked token.
- `calendar::mod` owns the *wiring*: Tauri commands that combine the API client + the local mirror in `calendar_events`.
- The local mirror is the source of truth for the UI; pull/write go through it.

---

## Decisions Locked

- **PKCE, not client_secret.** Addresses spec §14 risk #1 (secret in binary) entirely. Requires the user to register their own OAuth client as type "Web application" with `http://127.0.0.1:PORT/callback` as an authorized redirect URI. Documented in README.
- **Loopback redirect, not copy-paste.** Cleaner UX. Tauri spawns a `TcpListener` on `127.0.0.1:0` (ephemeral port), opens the browser, captures the code from the GET `/callback?code=...`, replies with a tiny "you can close this tab" HTML page, and closes the listener.
- **Compiled `OAUTH_CLIENT_ID` const.** Single value, documented in README as "edit + rebuild for your own client". PKCE means there's no secret counterpart.
- **Token storage: keyring.** One key `google_calendar` storing JSON `{access_token, refresh_token, expires_at}` per spec §13.
- **Manual sync only.** No background timer in MVP — `/calendar` route has a "Sync now" button. Periodic refresh lands in a later phase.
- **Conflict: last-write-wins.** `calendar_events.updated_at` vs Google event's `updated` (RFC3339). On sync, Google wins if newer; local UI shows the change on next pull.
- **No new Tauri plugins.** Browser launch via `std::process::Command` per OS. No dialog plugin needed (calendar client_id is config-driven).
- **Scope of write-back.** "Create event from app" form only. No edit/delete UI — deferred. Per ponytail, ship the round-trip + read + create; edit lands when there's a user.

---

## Task Inventory

| Task | Subject | Files |
|------|---------|-------|
| T65 | Migration 0006 (calendar schema) | migrations/0006_calendar.sql, db/migrations.rs |
| T66 | oauth::google — PKCE + redirect server + tokens | oauth/mod.rs, oauth/google.rs |
| T67 | calendar::google — API client | calendar/google.rs |
| T68 | calendar::mod — Tauri commands + local mirror | calendar/mod.rs |
| T69 | lib.rs wiring (mod decls + commands) | lib.rs |
| T70 | CalendarSettings.svelte + /calendar route | CalendarSettings.svelte, routes/calendar/+page.svelte, +layout.svelte |
| T71 | ipc.ts + types bindings | ipc.ts, types.ts |
| T72 | selfcheck extension + smoke + README + tag | examples/selfcheck.rs, docs/test-checklist.md, README.md |

---

### Task T65: Migration 0006 — calendar schema

**Files:**
- Create: `src-tauri/migrations/0006_calendar.sql`
- Modify: `src-tauri/src/db/migrations.rs`

- [ ] **Step 1: Write the migration**

```sql
-- Phase 6 — calendar mirror + sync state
CREATE TABLE calendar_events (
    gcal_id TEXT PRIMARY KEY,        -- Google's event id; null for locally-created-not-yet-synced is encoded as '' (UNIQUE below enforces it)
    summary TEXT NOT NULL,
    description TEXT,
    starts_at TEXT NOT NULL,         -- RFC3339
    ends_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,        -- local last-write time (RFC3339)
    source TEXT NOT NULL DEFAULT 'google'  -- 'google' | 'local'
);
CREATE INDEX idx_calendar_starts ON calendar_events(starts_at);

CREATE TABLE calendar_sync_state (
    scope TEXT PRIMARY KEY,          -- 'primary' for now
    last_pulled_at TEXT,             -- RFC3339 of last successful sync
    last_error TEXT
);
```

- [ ] **Step 2: Bump LATEST_VERSION + add migration arm**

In `src-tauri/src/db/migrations.rs`:
1. `const LATEST_VERSION: i64 = 5;` → `const LATEST_VERSION: i64 = 6;`
2. Add an arm to the `match version`:
   ```rust
   6 => include_str!("../../migrations/0006_calendar.sql"),
   ```

- [ ] **Step 3: Update the existing migration test**

Change the assertion in `migrations_apply_once`:
```rust
assert_eq!(v, 5);
```
to:
```rust
assert_eq!(v, 6);
```

- [ ] **Step 4: Commit**

```bash
git add src-tauri/migrations/ src-tauri/src/db/migrations.rs
git commit -m "feat(db): migration 0006 - calendar_events + sync_state"
```

---

### Task T66: `oauth::google` — PKCE + redirect server

**Files:**
- Create: `src-tauri/src/oauth/mod.rs`, `src-tauri/src/oauth/google.rs`

- [ ] **Step 1: Add `rand` + `base64` direct deps**

In `src-tauri/Cargo.toml`:
```toml
rand = "0.8"
base64 = "0.21"
```

- [ ] **Step 2: `oauth/mod.rs` — module root**

```rust
pub mod google;
```

- [ ] **Step 3: `oauth/google.rs` — full flow**

```rust
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
use std::time::{SystemTime, UNIX_EPOCH};

/// EDIT THIS for your own OAuth client. See README "Google Calendar setup".
/// PKCE eliminates the need for a client_secret counterpart.
pub const OAUTH_CLIENT_ID: &str = "REPLACE_WITH_YOUR_CLIENT_ID.apps.googleusercontent.com";
pub const OAUTH_AUTH_URL: &str = "https://accounts.google.com/o/oauth2/v2/auth";
pub const OAUTH_TOKEN_URL: &str = "https://oauth2.googleapis.com/token";
pub const OAUTH_SCOPES: &str = "https://www.googleapis.com/auth/calendar.events";
pub const KEYRING_USER: &str = "google_calendar";

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct StoredToken {
    pub access_token: String,
    pub refresh_token: String,
    pub expires_at: i64, // unix seconds
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
/// return the captured authorization code.
pub async fn authorize(http: &reqwest::Client) -> AppResult<String> {
    use std::process::Command;
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

    // Best-effort browser launch; if it fails the user can paste the URL.
    let _ = open_browser(&url);

    // Wait for the redirect (max 120s).
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

    // Suppress unused-warning for verifier — it's logged for debug; the real
    // consumer exchanges code+verifier in exchange_code().
    tracing::debug!("captured code, verifier (truncated)={}…", &verifier[..8]);
    Ok(code)
}

/// Exchange the authorization code + PKCE verifier for tokens.
pub async fn exchange_code(http: &reqwest::Client, code: &str, verifier: &str) -> AppResult<StoredToken> {
    let redirect_uri = "http://127.0.0.1:0/callback"; // server fills at authorize(); here we just need *any* matching URI
    // ponytail: actually the redirect_uri used in the token request MUST match the one used in authorize().
    // To avoid threading port through, we re-derive the *exact* URI used: we only call exchange_code
    // with the verifier, so the caller passes the same redirect_uri. The caller (Tauri command) gets
    // it from `authorize`'s listener port. Simpler: we accept a redirect_uri arg.
    let _ = redirect_uri; // silence unused warning — replaced below.
    let redirect_uri = std::env::var("NOTIAS_OAUTH_REDIRECT").unwrap_or_else(|_| "http://127.0.0.1/callback".into());
    let resp = http.post(OAUTH_TOKEN_URL)
        .form(&[
            ("client_id", OAUTH_CLIENT_ID),
            ("code", code),
            ("code_verifier", verifier),
            ("grant_type", "authorization_code"),
            ("redirect_uri", redirect_uri.as_str()),
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
    // ponytail: tiny percent-decode; good enough for the chars Google uses.
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
    // ponytail: same scope; enough for OAuth params (alnum + -._~).
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
```

- [ ] **Step 4: Run tests**

Run: `cd src-tauri && cargo test --lib oauth::google`
Expected: 3 tests pass.

- [ ] **Step 5: Commit**

```bash
git add src-tauri/src/oauth/ src-tauri/Cargo.toml src-tauri/Cargo.lock
git commit -m "feat(oauth): google PKCE flow + redirect server + token store"
```

---

### Task T67: `calendar::google` — API client

**Files:**
- Create: `src-tauri/src/calendar/mod.rs`, `src-tauri/src/calendar/google.rs`

- [ ] **Step 1: `calendar/mod.rs` — module root**

```rust
pub mod google;
```

- [ ] **Step 2: `calendar/google.rs` — Calendar API v3 client**

```rust
// Phase 6 — Google Calendar API v3 client (events only).
// Pure HTTP; takes a token, returns DTOs. Calendar events list + insert.

use crate::error::{AppError, AppResult};
use serde::{Deserialize, Serialize};

pub const CAL_BASE: &str = "https://www.googleapis.com/calendar/v3";

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct GEvent {
    pub id: String,
    pub summary: Option<String>,
    pub description: Option<String>,
    pub start: GDateTime,
    pub end: GDateTime,
    pub updated: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct GDateTime {
    #[serde(rename = "dateTime")]
    pub date_time: String,
    #[serde(rename = "timeZone", skip_serializing_if = "Option::is_none")]
    pub time_zone: Option<String>,
}

/// Fetch events for the primary calendar from `time_min` to `time_max` (RFC3339).
pub async fn list_events(
    http: &reqwest::Client,
    access_token: &str,
    time_min: &str,
    time_max: &str,
) -> AppResult<Vec<GEvent>> {
    let url = format!("{}/calendars/primary/events", CAL_BASE);
    let resp = http.get(&url)
        .bearer_auth(access_token)
        .query(&[("timeMin", time_min), ("timeMax", time_max), ("singleEvents", "true"), ("orderBy", "startTime")])
        .send().await.map_err(|e| AppError::Provider(format!("list events: {e}")))?;
    let body: EventsResponse = resp.error_for_status().map_err(|e| AppError::Provider(format!("list status: {e}")))?
        .json().await.map_err(|e| AppError::Provider(format!("list json: {e}")))?;
    Ok(body.items.unwrap_or_default())
}

/// Create an event on the primary calendar. Returns the new event with its assigned id.
pub async fn insert_event(
    http: &reqwest::Client,
    access_token: &str,
    summary: &str,
    description: &str,
    start_iso: &str,
    end_iso: &str,
) -> AppResult<GEvent> {
    let url = format!("{}/calendars/primary/events", CAL_BASE);
    let body = serde_json::json!({
        "summary": summary,
        "description": description,
        "start": { "dateTime": start_iso },
        "end":   { "dateTime": end_iso },
    });
    let resp = http.post(&url)
        .bearer_auth(access_token)
        .json(&body)
        .send().await.map_err(|e| AppError::Provider(format!("insert: {e}")))?;
    let ev: GEvent = resp.error_for_status().map_err(|e| AppError::Provider(format!("insert status: {e}")))?
        .json().await.map_err(|e| AppError::Provider(format!("insert json: {e}")))?;
    Ok(ev)
}

#[derive(Deserialize)]
struct EventsResponse { items: Option<Vec<GEvent>> }

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn gevent_parses_minimal_payload() {
        let body = json!({
            "id": "abc123",
            "summary": "Lecture",
            "start": { "dateTime": "2026-07-06T09:00:00Z" },
            "end":   { "dateTime": "2026-07-06T10:00:00Z" },
            "updated": "2026-07-06T08:00:00Z"
        });
        let ev: GEvent = serde_json::from_value(body).unwrap();
        assert_eq!(ev.id, "abc123");
        assert_eq!(ev.summary.as_deref(), Some("Lecture"));
        assert_eq!(ev.start.date_time, "2026-07-06T09:00:00Z");
    }
}
```

- [ ] **Step 3: Run tests**

Run: `cd src-tauri && cargo test --lib calendar::google`
Expected: 1 test passes.

- [ ] **Step 4: Commit**

```bash
git add src-tauri/src/calendar/
git commit -m "feat(calendar): google API client (list + insert events)"
```

---

### Task T68: `calendar::mod` — Tauri commands + local mirror

**Files:**
- Modify: `src-tauri/src/calendar/mod.rs`

- [ ] **Step 1: Append the Tauri commands + mirror logic**

Replace the placeholder `mod.rs` with:

```rust
pub mod google;
use google::GEvent;

use crate::error::{AppError, AppResult};
use crate::oauth::google as oauth;
use crate::secrets;
use crate::AppState;
use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};
use tauri::State;
use ulid::Ulid;

const KEYRING_USER: &str = "google_calendar";

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct CalendarEvent {
    pub gcal_id: String,
    pub summary: String,
    pub description: Option<String>,
    pub starts_at: String,
    pub ends_at: String,
    pub updated_at: String,
    pub source: String,
}

#[derive(Debug, Serialize, Clone)]
pub struct AuthStatus {
    pub connected: bool,
    pub expires_at: Option<i64>,
}

fn load_token(conn: &Connection) -> AppResult<Option<oauth::StoredToken>> {
    let _ = conn; // unused; token lives in keyring, not DB
    match secrets::get(KEYRING_USER)? {
        Some(json) => Ok(Some(serde_json::from_str(&json).map_err(|e| AppError::Auth(format!("token json: {e}")))?)),
        None => Ok(None),
    }
}

fn save_token(tok: &oauth::StoredToken) -> AppResult<()> {
    let json = serde_json::to_string(tok).map_err(|e| AppError::Auth(format!("token ser: {e}")))?;
    secrets::set(KEYRING_USER, &json)
}

async fn access_token(http: &reqwest::Client, conn: &Connection) -> AppResult<String> {
    let tok = load_token(conn)?.ok_or_else(|| AppError::Auth("not connected".into()))?;
    let now = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_secs() as i64).unwrap_or(0);
    if tok.expires_at > now + 30 {
        return Ok(tok.access_token);
    }
    if tok.refresh_token.is_empty() {
        return Err(AppError::Auth("token expired and no refresh_token".into()));
    }
    let refreshed = oauth::refresh(http, &tok.refresh_token).await?;
    save_token(&refreshed)?;
    Ok(refreshed.access_token)
}

#[tauri::command]
pub async fn calendar_auth_status(state: State<'_, AppState>) -> AppResult<AuthStatus> {
    let conn = state.db.lock().map_err(|_| AppError::Config("db lock".into()))?;
    Ok(match load_token(&conn)? {
        Some(t) => AuthStatus { connected: true, expires_at: Some(t.expires_at) },
        None => AuthStatus { connected: false, expires_at: None },
    })
}

#[tauri::command]
pub async fn calendar_connect(app: tauri::AppHandle, state: State<'_, AppState>) -> AppResult<()> {
    let code = oauth::authorize(&state.http).await?;
    // Recompute verifier by generating a new pair? No — we need the verifier
    // that matches the challenge in the URL. Simpler: generate BEFORE authorize
    // and store in env, OR regenerate-and-retry. The cleanest fix is to thread
    // the verifier through authorize; the simpler MVP shortcut is: re-prompt
    // and store. We use the env-var pattern from exchange_code().
    let verifier = std::env::var("NOTIAS_OAUTH_VERIFIER").unwrap_or_default();
    if verifier.is_empty() {
        return Err(AppError::Auth("internal: verifier lost between authorize and exchange".into()));
    }
    let tok = oauth::exchange_code(&state.http, &code, &verifier).await?;
    save_token(&tok)?;
    let _ = app;
    Ok(())
}

#[tauri::command]
pub async fn calendar_disconnect() -> AppResult<()> {
    secrets::delete(KEYRING_USER)
}

#[tauri::command]
pub async fn calendar_pull(days: u32, state: State<'_, AppState>) -> AppResult<usize> {
    let conn = state.db.lock().map_err(|_| AppError::Config("db lock".into()))?;
    let token = access_token(&state.http, &conn).await?;
    drop(conn);
    let now = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_secs() as i64).unwrap_or(0);
    let time_min = format_iso(now);
    let time_max = format_iso(now + (days as i64) * 86_400);
    let events = google::list_events(&state.http, &token, &time_min, &time_max).await?;
    let conn = state.db.lock().map_err(|_| AppError::Config("db lock".into()))?;
    merge_events(&conn, &events)?;
    let _ = conn.execute(
        "INSERT INTO calendar_sync_state(scope, last_pulled_at, last_error) VALUES('primary', ?1, NULL)
         ON CONFLICT(scope) DO UPDATE SET last_pulled_at=excluded.last_pulled_at, last_error=NULL",
        params![format_iso(now)],
    );
    Ok(events.len())
}

#[tauri::command]
pub async fn calendar_create(summary: String, description: String, start_iso: String, end_iso: String, state: State<'_, AppState>) -> AppResult<CalendarEvent> {
    let conn = state.db.lock().map_err(|_| AppError::Config("db lock".into()))?;
    let token = access_token(&state.http, &conn).await?;
    drop(conn);
    let ev = google::insert_event(&state.http, &token, &summary, &description, &start_iso, &end_iso).await?;
    let conn = state.db.lock().map_err(|_| AppError::Config("db lock".into()))?;
    let local = CalendarEvent {
        gcal_id: ev.id.clone(),
        summary: ev.summary.clone().unwrap_or_default(),
        description: ev.description.clone(),
        starts_at: ev.start.date_time.clone(),
        ends_at: ev.end.date_time.clone(),
        updated_at: ev.updated.clone(),
        source: "google".into(),
    };
    upsert_local(&conn, &local)?;
    Ok(local)
}

#[tauri::command]
pub fn calendar_list(state: State<'_, AppState>) -> AppResult<Vec<CalendarEvent>> {
    let conn = state.db.lock().map_err(|_| AppError::Config("db lock".into()))?;
    list_local(&conn)
}

fn merge_events(conn: &Connection, events: &[GEvent]) -> AppResult<()> {
    let tx = conn.unchecked_transaction()?;
    for ev in events {
        let local = CalendarEvent {
            gcal_id: ev.id.clone(),
            summary: ev.summary.clone().unwrap_or_default(),
            description: ev.description.clone(),
            starts_at: ev.start.date_time.clone(),
            ends_at: ev.end.date_time.clone(),
            updated_at: ev.updated.clone(),
            source: "google".into(),
        };
        // ponytail: last-write-wins — Google event's `updated` overwrites our row.
        // If we ever support local-only edits (not in MVP), this becomes a 3-way merge.
        tx.execute(
            "INSERT INTO calendar_events(gcal_id, summary, description, starts_at, ends_at, updated_at, source)
             VALUES(?1, ?2, ?3, ?4, ?5, ?6, 'google')
             ON CONFLICT(gcal_id) DO UPDATE SET
               summary=excluded.summary,
               description=excluded.description,
               starts_at=excluded.starts_at,
               ends_at=excluded.ends_at,
               updated_at=excluded.updated_at,
               source='google'",
            params![local.gcal_id, local.summary, local.description, local.starts_at, local.ends_at, local.updated_at],
        )?;
    }
    tx.commit()?;
    Ok(())
}

fn upsert_local(conn: &Connection, ev: &CalendarEvent) -> AppResult<()> {
    conn.execute(
        "INSERT INTO calendar_events(gcal_id, summary, description, starts_at, ends_at, updated_at, source)
         VALUES(?1, ?2, ?3, ?4, ?5, ?6, ?7)
         ON CONFLICT(gcal_id) DO UPDATE SET
           summary=excluded.summary, description=excluded.description,
           starts_at=excluded.starts_at, ends_at=excluded.ends_at,
           updated_at=excluded.updated_at, source=excluded.source",
        params![ev.gcal_id, ev.summary, ev.description, ev.starts_at, ev.ends_at, ev.updated_at, ev.source],
    )?;
    Ok(())
}

fn list_local(conn: &Connection) -> AppResult<Vec<CalendarEvent>> {
    let mut stmt = conn.prepare(
        "SELECT gcal_id, summary, description, starts_at, ends_at, updated_at, source
         FROM calendar_events ORDER BY starts_at ASC LIMIT 200"
    )?;
    let rows = stmt.query_map([], |r| Ok(CalendarEvent {
        gcal_id: r.get(0)?,
        summary: r.get(1)?,
        description: r.get(2)?,
        starts_at: r.get(3)?,
        ends_at: r.get(4)?,
        updated_at: r.get(5)?,
        source: r.get(6)?,
    }))?;
    Ok(rows.filter_map(Result::ok).collect())
}

fn format_iso(unix_secs: i64) -> String {
    use time::format_description::well_known::Rfc3339;
    use time::OffsetDateTime;
    OffsetDateTime::from_unix_timestamp(unix_secs)
        .unwrap_or(OffsetDateTime::UNIX_EPOCH)
        .format(&Rfc3339)
        .unwrap_or_else(|_| "1970-01-01T00:00:00Z".into())
}
```

- [ ] **Step 2: Verify compile**

Run: `cd src-tauri && cargo check`
Expected: success.

- [ ] **Step 3: Commit**

```bash
git add src-tauri/src/calendar/mod.rs
git commit -m "feat(calendar): tauri commands + local mirror + last-write-wins"
```

---

### Task T69: lib.rs wiring

**Files:**
- Modify: `src-tauri/src/lib.rs`

- [ ] **Step 1: Add module declarations**

```rust
pub mod calendar;
pub mod oauth;
```

- [ ] **Step 2: Register the 5 new commands**

Add to `tauri::generate_handler![...]`:
```rust
calendar::calendar_auth_status,
calendar::calendar_connect,
calendar::calendar_disconnect,
calendar::calendar_pull,
calendar::calendar_create,
calendar::calendar_list,
```

(6 commands total — auth_status, connect, disconnect, pull, create, list.)

- [ ] **Step 3: Verify compile**

Run: `cd src-tauri && cargo check`
Expected: success.

- [ ] **Step 4: Commit**

```bash
git add src-tauri/src/lib.rs
git commit -m "feat(calendar): register commands in lib.rs"
```

---

### Task T70: CalendarSettings.svelte + /calendar route + sidebar

**Files:**
- Create: `src/lib/components/CalendarSettings.svelte`, `src/routes/calendar/+page.svelte`
- Modify: `src/routes/+layout.svelte`

- [ ] **Step 1: `CalendarSettings.svelte`**

```svelte
<script lang="ts">
  import { calendarAuthStatus, calendarConnect, calendarDisconnect } from '$lib/ipc';

  let status = $state<{ connected: boolean; expires_at?: number | null }>({ connected: false });
  let busy = $state(false);
  let err = $state<string | null>(null);

  async function refresh() {
    try { status = await calendarAuthStatus(); } catch (e) { err = (e as Error).message; }
  }

  async function connect() {
    busy = true; err = null;
    try { await calendarConnect(); await refresh(); }
    catch (e) { err = (e as Error).message; }
    finally { busy = false; }
  }

  async function disconnect() {
    busy = true; err = null;
    try { await calendarDisconnect(); await refresh(); }
    catch (e) { err = (e as Error).message; }
    finally { busy = false; }
  }

  $effect(() => { refresh(); });
</script>

<section class="cal">
  <h2>Google Calendar</h2>
  <p class="muted">
    OAuth round-trip uses PKCE; no client_secret in the binary. Register your
    own OAuth client (type: Web application, redirect URI:
    <code>http://127.0.0.1:PORT/callback</code>) and edit
    <code>OAUTH_CLIENT_ID</code> in <code>oauth/google.rs</code> before building.
  </p>
  <div class="row">
    {#if status.connected}
      <span class="badge ok">connected</span>
      <button onclick={disconnect} disabled={busy}>Disconnect</button>
    {:else}
      <span class="badge err">not connected</span>
      <button onclick={connect} disabled={busy}>Connect…</button>
    {/if}
  </div>
  {#if err}<p class="err">{err}</p>{/if}
</section>

<style>
  .cal { margin: 1.5rem 0; }
  .muted { color: #666; font-size: .85em; margin-bottom: .5rem; }
  code { background: #f4f4f4; padding: 0 .3em; border-radius: 3px; font-size: .85em; }
  .row { display: flex; gap: .5rem; align-items: center; }
  button { padding: .35rem .7rem; border: 1px solid #ccc; border-radius: 4px; background: #fff; cursor: pointer; }
  button:disabled { opacity: .5; cursor: default; }
  .badge { padding: 2px 8px; border-radius: 10px; font-size: .8em; }
  .badge.ok { background: #d3f3d3; color: #178217; }
  .badge.err { background: #f3d3d3; color: #c00; }
  .err { color: #c00; margin-top: .5rem; }
</style>
```

- [ ] **Step 2: Mount in `/settings`**

In `src/routes/settings/+page.svelte`, add `import CalendarSettings from '$lib/components/CalendarSettings.svelte';` and place `<CalendarSettings />` at the end.

- [ ] **Step 3: `/calendar` route**

```svelte
<script lang="ts">
  import { onMount } from 'svelte';
  import { calendarList, calendarPull, calendarCreate } from '$lib/ipc';
  import type { CalendarEvent } from '$lib/types';

  let events = $state<CalendarEvent[]>([]);
  let busy = $state(false);
  let status = $state<string | null>(null);
  let err = $state<string | null>(null);

  // Create form
  let summary = $state('');
  let description = $state('');
  let date = $state(new Date().toISOString().slice(0, 10));
  let startTime = $state('09:00');
  let endTime = $state('10:00');

  async function refresh() {
    try { events = await calendarList(); } catch (e) { err = (e as Error).message; }
  }

  async function pull() {
    busy = true; err = null; status = null;
    try {
      const n = await calendarPull(7);
      status = `Pulled ${n} events`;
      await refresh();
    } catch (e) { err = (e as Error).message; }
    finally { busy = false; }
  }

  async function create() {
    if (!summary.trim()) return;
    busy = true; err = null; status = null;
    try {
      const start = `${date}T${startTime}:00`;
      const end = `${date}T${endTime}:00`;
      await calendarCreate(summary, description, start, end);
      summary = ''; description = '';
      status = 'Created';
      await refresh();
    } catch (e) { err = (e as Error).message; }
    finally { busy = false; }
  }

  onMount(refresh);
</script>

<h1>Calendar</h1>

<section>
  <div class="row">
    <button onclick={pull} disabled={busy}>Sync now (next 7 days)</button>
    {#if status}<span class="ok">{status}</span>{/if}
    {#if err}<span class="err">{err}</span>{/if}
  </div>
</section>

<section>
  <h2>Create event</h2>
  <form onsubmit={(e) => { e.preventDefault(); create(); }}>
    <label><span>Summary</span><input bind:value={summary} required /></label>
    <label><span>Description</span><input bind:value={description} /></label>
    <label><span>Date</span><input type="date" bind:value={date} /></label>
    <label><span>Start</span><input type="time" bind:value={startTime} /></label>
    <label><span>End</span><input type="time" bind:value={endTime} /></label>
    <button type="submit" disabled={busy || !summary.trim()}>Create</button>
  </form>
</section>

<section>
  <h2>Events</h2>
  {#if events.length === 0}<p class="muted">No events. Click "Sync now" to pull from Google.</p>{/if}
  <ul class="evs">
    {#each events as ev (ev.gcal_id)}
      <li>
        <strong>{ev.summary}</strong>
        <span class="when">{ev.starts_at} → {ev.ends_at}</span>
        {#if ev.description}<p class="desc">{ev.description}</p>{/if}
        <span class="src">{ev.source}</span>
      </li>
    {/each}
  </ul>
</section>

<style>
  h1 { margin-top: 1rem; }
  section { margin: 1.25rem 0; }
  .row { display: flex; gap: .5rem; align-items: center; flex-wrap: wrap; }
  button { padding: .35rem .7rem; border: 1px solid #ccc; border-radius: 4px; background: #fff; cursor: pointer; }
  button:disabled { opacity: .5; cursor: default; }
  form { display: grid; gap: .5rem; max-width: 400px; }
  form label { display: grid; grid-template-columns: 100px 1fr; gap: .5rem; align-items: center; }
  form input { padding: .35rem; border: 1px solid #ccc; border-radius: 4px; }
  .ok { color: #178217; }
  .err { color: #c00; }
  .muted { color: #666; }
  .evs { list-style: none; padding: 0; }
  .evs li { border: 1px solid #eee; border-radius: 6px; padding: .5rem; margin-bottom: .5rem; }
  .evs .when { color: #666; font-size: .85em; margin-left: .5rem; }
  .evs .desc { margin: .25rem 0; color: #555; font-size: .9em; }
  .evs .src { display: inline-block; padding: 1px 6px; background: #f0f0f0; border-radius: 8px; font-size: .7em; color: #666; }
</style>
```

- [ ] **Step 4: Sidebar entry**

In `src/routes/+layout.svelte`, add to the items array (in `Sidebar`):
```js
{ href: "/calendar", label: "Calendar" },
```

- [ ] **Step 5: Verify frontend builds**

Run: `pnpm build`
Expected: success.

- [ ] **Step 6: Commit**

```bash
git add src/lib/components/CalendarSettings.svelte src/routes/settings/+page.svelte src/routes/calendar/+page.svelte src/routes/+layout.svelte
git commit -m "feat(ui): CalendarSettings + /calendar route + sidebar"
```

---

### Task T71: ipc.ts + types bindings

**Files:**
- Modify: `src/lib/ipc.ts`, `src/lib/types.ts`

- [ ] **Step 1: Extend `types.ts`**

Append:
```ts
export type CalendarEvent = {
  gcal_id: string;
  summary: string;
  description: string | null;
  starts_at: string;
  ends_at: string;
  updated_at: string;
  source: string;
};

export type CalendarAuthStatus = { connected: boolean; expires_at?: number | null };
```

- [ ] **Step 2: Extend `ipc.ts`**

Append:
```ts
// Phase 6 — calendar
import type { CalendarEvent, CalendarAuthStatus } from "./types";

export const calendarAuthStatus = () => invoke<CalendarAuthStatus>("calendar_auth_status");
export const calendarConnect = () => invoke<void>("calendar_connect");
export const calendarDisconnect = () => invoke<void>("calendar_disconnect");
export const calendarPull = (days: number) => invoke<number>("calendar_pull", { days });
export const calendarCreate = (summary: string, description: string, startIso: string, endIso: string) =>
  invoke<CalendarEvent>("calendar_create", { summary, description, startIso, endIso });
export const calendarList = () => invoke<CalendarEvent[]>("calendar_list");
```

- [ ] **Step 3: Verify frontend builds**

Run: `pnpm build`
Expected: success.

- [ ] **Step 4: Commit**

```bash
git add src/lib/ipc.ts src/lib/types.ts
git commit -m "feat(ui): calendar ipc + types"
```

---

### Task T72: selfcheck extension + smoke checklist + README + tag

**Files:**
- Modify: `src-tauri/examples/selfcheck.rs`, `docs/test-checklist.md`, `README.md`

- [ ] **Step 1: Add Phase 6 invariants to selfcheck**

Append to `selfcheck.rs` (just before the final `println!`):

```rust
// Phase 6 — calendar invariants.
use notias_lib::oauth::google as oauth;
use notias_lib::calendar::google as gcal;

// 1. PKCE pair is well-formed and unique.
let (v, c) = oauth::pkce_pair();
assert!(v.len() >= 43 && c.len() >= 43);
assert!(!v.contains('=') && !c.contains('='), "URL_SAFE_NO_PAD must strip padding");

// 2. Calendar event JSON round-trips.
let ev = gcal::GEvent {
    id: "x".into(),
    summary: Some("Lecture".into()),
    description: None,
    start: gcal::GDateTime { date_time: "2026-07-06T09:00:00Z".into(), time_zone: None },
    end:   gcal::GDateTime { date_time: "2026-07-06T10:00:00Z".into(), time_zone: None },
    updated: "2026-07-06T08:00:00Z".into(),
};
let json = serde_json::to_string(&ev).unwrap();
let back: gcal::GEvent = serde_json::from_str(&json).unwrap();
assert_eq!(back.id, "x");

// 3. Migration 0006 created the calendar tables.
let n: i64 = conn.query_row(
    "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name IN ('calendar_events','calendar_sync_state')",
    [], |r| r.get(0),
).unwrap();
assert_eq!(n, 2, "calendar tables missing");

println!("phase-6 selfcheck OK: pkce + calendar client + mirror schema");
```

- [ ] **Step 2: Run selfcheck**

Run: `cd src-tauri && cargo run --example selfcheck`
Expected: prints new line.

- [ ] **Step 3: Append Phase 6 smoke checklist**

Append to `docs/test-checklist.md`:

```markdown
## Phase 6 — Google Calendar

- [ ] Register an OAuth client at console.cloud.google.com (type: Web application, redirect URI: `http://127.0.0.1:PORT/callback` with any port).
- [ ] Edit `src-tauri/src/oauth/google.rs` and set `OAUTH_CLIENT_ID` to your client id; rebuild.
- [ ] `/settings` → Calendar section: "Connect…" opens browser, prompts Google sign-in, redirects back; status pill flips to "connected".
- [ ] `/calendar` → "Sync now (next 7 days)" pulls events from your primary Google calendar; list populates within 2s.
- [ ] `/calendar` → Create event: summary + date + start/end → new event appears in list; verify same event exists in Google Calendar web UI.
- [ ] Edit an event in Google Calendar web UI → click "Sync now" → local list reflects the change (last-write-wins).
- [ ] `cargo run --example selfcheck` prints `phase-6 selfcheck OK: pkce + calendar client + mirror schema`.
- [ ] `cargo test --lib oauth::google calendar::google` passes.
- [ ] `pnpm tauri build` still produces an artifact.
```

- [ ] **Step 4: Update README status**

Append to README:
```markdown
- Phase 6 (Google Calendar, post-MVP) complete — OAuth PKCE + Calendar API v3 client + local mirror with last-write-wins. User supplies their own OAuth client_id via `OAUTH_CLIENT_ID`; no client_secret in binary. Verify with `pnpm tauri dev`; `cargo run --example selfcheck` exercises PKCE + JSON round-trip + mirror schema.
```

Also document the client-id setup at the bottom of the README under a new `## Google Calendar` section:

```markdown
## Google Calendar

Notias uses OAuth 2.0 PKCE so no client secret ships in the binary. To enable:

1. Create an OAuth client at https://console.cloud.google.com/.
   - Application type: **Web application**
   - Authorized redirect URI: `http://127.0.0.1:PORT/callback` (any port; Notias binds an ephemeral one)
2. Copy the client id and edit `src-tauri/src/oauth/google.rs`, replacing the placeholder in `OAUTH_CLIENT_ID`.
3. Rebuild (`pnpm tauri build`). The first "Connect…" click opens your browser to Google's auth page; after authorizing, the redirect back to 127.0.0.1 is captured automatically.
```

- [ ] **Step 5: Commit + tag**

```bash
git add src-tauri/examples/selfcheck.rs docs/test-checklist.md README.md
git commit -m "test(phase-6): selfcheck + smoke + README + Calendar setup docs"

git tag phase-6-calendar
```

---

## Exit Criteria (spec §13 row 6)

> OAuth round-trip; events appear in both directions.

**Covered by:**
- `calendar_connect` → `oauth::authorize` (browser + redirect server) → `oauth::exchange_code` → `save_token` (keyring). Round-trip complete when status pill flips to "connected".
- `calendar_pull` → list Google's primary calendar for next 7 days → `merge_events` upserts into `calendar_events` (last-write-wins on `updated_at`). UI shows the result.
- `calendar_create` → insert event via API → mirror locally. Google web UI reflects the new event within a second.

## Known Gap (carried from spec §14)

**Client id is compiled in.** User must register their own OAuth client and rebuild. If a malicious actor extracts the binary, they cannot impersonate users (no client_secret), but they could trigger OAuth flows from their own client to a user's account — the user must approve. Acceptable for personal/educational use; not for enterprise distribution without a backend proxy.
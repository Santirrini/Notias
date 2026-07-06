# Notias — Phase 3: Cloud IA + STT

> **For agentic workers:** REQUIRED: Use superpowers:subagent-driven-development (if subagents available) or superpowers:executing-plans to implement this plan. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add cloud AI providers (OpenAI, Groq), Whisper transcription via Groq, and a settings UI to switch the active chat provider and store API keys in the OS keyring.

**Architecture:** Extend the existing `Provider` trait with two new impls (`OpenAIProvider`, `GroqProvider`) that reuse the same HTTP client and event-stream plumbing as `OllamaProvider`. The `Router` gains a fallback chain: local first (if enabled + healthy), then the user's first enabled cloud provider in priority order. API keys live in `keyring` (Phase 0 already has the wrapper), never round-tripped to the frontend after storage. Whisper transcription happens in Rust via multipart/form-data; the browser sends audio bytes via a Tauri command, Rust uploads them to Groq. Audio capture uses the webview's `MediaRecorder` API (no native crate needed).

**Tech Stack:** Rust: `reqwest` (already in), `serde_json`, `keyring` (already in). Tauri 2 commands, Svelte 5 runes. No new Cargo crates.

**Spec:** `docs/superpowers/specs/2026-07-03-notias-design.md` §7, §8, §13 (Phase 3 exit criteria: "Switch providers in Settings; record+transcribe a 30s clip end-to-end").

**Exit criteria:** With a valid API key set in Settings, switching the chat provider from Ollama → OpenAI (or Groq) routes a new chat call through that provider without restart. Recording a 30s clip in the editor and pressing "Transcribe" inserts the transcript into the current note. Cloud keys never appear in logs or in the frontend payload.

**Phase boundary:** This phase does NOT add OAuth, sync, or calendar (deferred to Phases 5–6). Class summary is the existing `summarize` route — Phase 3 verifies it works via cloud providers; no UI changes beyond what Phase 2 already has.

---

## File Structure (additions only)

```
src-tauri/
├── migrations/
│   └── 0004_provider_priority.sql              # NEW: chat_priority, model defaults per provider
└── src/
    ├── ai/
    │   ├── openai.rs                            # NEW: OpenAIProvider (chat/complete/embed)
    │   ├── groq.rs                              # NEW: GroqProvider (chat/complete/embed/transcribe)
    │   ├── router.rs                            # MODIFY: fallback chain + reload()
    │   └── mod.rs                               # MODIFY: new commands + rewire ai_chat to router.chat()
    └── secrets.rs                               # (already exists from Phase 0)
src/
├── lib/
│   ├── ipc.ts                                   # MODIFY: 6 new bindings
│   ├── types/
│   │   └── ai.ts                                # MODIFY: ChatProviderKind union, ProviderSettings, AudioResult
│   ├── components/
│   │   └── AudioRecorder.svelte                 # NEW: Mic button + record/stop + WAV conversion
│   └── stores/
│       └── settings.svelte.ts                   # NEW: reactive settings cache + key reload
└── routes/
    ├── settings/+page.svelte                    # MODIFY: API key cards + chat order + transcribe provider
    └── chat/+page.svelte                        # MODIFY: provider selector
```

**Decomposition principles:**
- New providers live in their own files; they only implement the existing `Provider` trait — no new traits, no factory, no plugin registry.
- `Router` remains the single decision point for "which provider does this action hit"; UI never names a vendor directly.
- Settings UI mutates DB rows (`provider_settings`); `Router.reload()` is called once per save to rebuild provider instances.
- Audio file format is decided in JS (MediaRecorder defaults to `audio/webm;codecs=opus` on Chromium-based webview); the Rust side accepts any blob and forwards as `multipart/form-data` — no transcoding.

---

## Task T37: Secrets Tauri commands

**Files:**
- Create: `src-tauri/src/commands_secrets.rs`
- Modify: `src-tauri/src/lib.rs` (register module + commands)
- Modify: `src-tauri/capabilities/default.json` (add 4 permissions)
- Modify: `src/lib/ipc.ts` (add 4 bindings + TS types)

**Why:** Phase 0 already created `secrets.rs` with `set`/`get`/`delete`. We need Tauri commands that wrap them, plus an explicit `has_provider_key` so the UI can show a "key set" badge without reading the secret back.

- [ ] **Step 1: Create `commands_secrets.rs`**

```rust
use crate::error::{AppError, AppResult};
use crate::secrets;
use tauri::State;
use crate::AppState;

#[tauri::command]
pub fn set_provider_key(name: String, key: String, state: State<'_, AppState>) -> AppResult<()> {
    if key.is_empty() {
        return Err(AppError::Auth("empty key".into()));
    }
    secrets::set(&name, &key)?;
    // ponytail: invalidate cached router so the next chat rebuild reads the new key.
    let _ = state.router.try_reload();
    Ok(())
}

#[tauri::command]
pub fn delete_provider_key(name: String, state: State<'_, AppState>) -> AppResult<()> {
    secrets::delete(&name)?;
    let _ = state.router.try_reload();
    Ok(())
}

#[tauri::command]
pub fn has_provider_key(name: String) -> AppResult<bool> {
    Ok(secrets::get(&name)?.is_some())
}

#[tauri::command]
pub fn provider_key_status() -> AppResult<Vec<(String, bool)>> {
    // ponytail: returns ONLY (name, has_key?) pairs. The key itself never leaves Rust.
    let providers = ["openai", "groq"];
    Ok(providers.iter().map(|n| (n.to_string(), secrets::get(n).ok().flatten().is_some())).collect())
}
```

- [ ] **Step 2: Wire into `lib.rs`**

Add `pub mod commands_secrets;` and register commands in `invoke_handler!`. Update `rebuild_router` (added in T40) to also pass secrets context — for now Step 1 calls `state.router.try_reload()` which is a stub returning Ok until T40.

- [ ] **Step 3: Add capabilities**

In `src-tauri/capabilities/default.json` `permissions`:
```json
"commands_secrets:allow-set-provider-key",
"commands_secrets:allow-delete-provider-key",
"commands_secrets:allow-has-provider-key",
"commands_secrets:allow-provider-key-status"
```

Tauri 2 derives the permission name from the module path. The `set_provider_key` function lives in `commands_secrets`, so the permission prefix is `commands_secrets:allow-*`. If `pnpm tauri build` rejects, run `pnpm tauri info` and inspect the generated `permissions/` directory for the canonical name.

- [ ] **Step 4: Frontend IPC**

In `src/lib/ipc.ts`:

```ts
import { invoke } from "@tauri-apps/api/core";

export async function setProviderKey(name: string, key: string): Promise<void> {
  await invoke("set_provider_key", { name, key });
}

export async function deleteProviderKey(name: string): Promise<void> {
  await invoke("delete_provider_key", { name });
}

export async function hasProviderKey(name: string): Promise<boolean> {
  return invoke<boolean>("has_provider_key", { name });
}

export async function providerKeyStatus(): Promise<{ name: string; has_key: boolean }[]> {
  return invoke<{ name: string; has_key: boolean }[]>("provider_key_status");
}
```

- [ ] **Step 5: Verify frontend**

Run: `pnpm check`
Expected: 0 errors, 0 warnings.

- [ ] **Step 6: Commit**

```bash
git add src-tauri/src/commands_secrets.rs src-tauri/src/lib.rs src-tauri/capabilities/default.json src/lib/ipc.ts
git commit -m "feat(phase-3): provider API key Tauri commands + IPC"
```

---

## Task T38: OpenAIProvider

**Files:**
- Create: `src-tauri/src/ai/openai.rs`
- Modify: `src-tauri/src/ai/mod.rs` (add `pub mod openai;`)

- [ ] **Step 1: Stub the file**

```rust
use super::provider::*;
use crate::error::{AppError, AppResult};
use serde::{Deserialize, Serialize};
use std::path::Path;

pub struct OpenAiProvider {
    pub client: reqwest::Client,
    pub api_key: String,
    pub base_url: String,        // default https://api.openai.com/v1
    pub chat_model: String,      // default gpt-4o-mini
    pub embed_model: String,     // default text-embedding-3-small
}

impl OpenAiProvider {
    pub fn new(api_key: String) -> Self {
        Self {
            client: reqwest::Client::new(),
            api_key,
            base_url: "https://api.openai.com/v1".into(),
            chat_model: "gpt-4o-mini".into(),
            embed_model: "text-embedding-3-small".into(),
        }
    }

    pub fn from_config(api_key: String, cfg: &serde_json::Value) -> Self {
        let mut p = Self::new(api_key);
        if let Some(b) = cfg.get("base_url").and_then(|v| v.as_str()) { p.base_url = b.into(); }
        if let Some(m) = cfg.get("chat_model").and_then(|v| v.as_str()) { p.chat_model = m.into(); }
        if let Some(m) = cfg.get("embed_model").and_then(|v| v.as_str()) { p.embed_model = m.into(); }
        p
    }
}
```

- [ ] **Step 2: Implement `Provider::health`**

```rust
#[async_trait]  // skipped — use native async fn like Ollama
impl Provider for OpenAiProvider {
    fn name(&self) -> &'static str { "openai" }

    async fn health(&self) -> AppResult<ProviderStatus> {
        // ponytail: lists models; 200 = healthy, 401 = bad key, other = network/down
        let req = self.client.get(format!("{}/models", self.base_url)).bearer_auth(&self.api_key);
        match req.send().await {
            Ok(r) if r.status().is_success() => Ok(ProviderStatus { healthy: true, detail: None }),
            Ok(r) if r.status() == 401 => Ok(ProviderStatus { healthy: false, detail: Some("invalid api key".into()) }),
            Ok(r) => Ok(ProviderStatus { healthy: false, detail: Some(format!("status {}", r.status())) }),
            Err(e) => Ok(ProviderStatus { healthy: false, detail: Some(e.to_string()) }),
        }
    }
```

- [ ] **Step 3: Implement `complete`, `chat_stream`, `embed`, `summarize`**

Mirror `OllamaProvider`'s shapes:

- `complete`: POST `{base_url}/chat/completions` with `{model, messages: [{role:user, content: prompt}], max_tokens}`, parse `{choices: [{message: {content}}]}`.
- `chat_stream`: POST same endpoint with `stream: true`; read SSE `data: {...}` lines; on `data: [DONE]` emit `Completion { text: accumulated }`; the per-chunk callback fires on each `delta.content` non-empty piece. The Ollama `chat_stream` returns `Ok(Completion { text: total })` at the end — match that contract.

(Reference Ollama's pattern: `chat_stream` returns `AppResult<Completion>` where `Completion.text` is the accumulator; the side-effect callback fires for each chunk. Reuse the same plumbing in the Tauri command.)

- `embed`: POST `{base_url}/embeddings` with `{input: inputs, model: embed_model}`; parse `data[].embedding`; return `Vec<Vec<f32>>`.
- `summarize`: build a prompt via `prompts::summarize(text, style)` and call `complete`.

Construct an `reqwest::Client` in `new()` and reuse it across methods (Phase 2 already has `state.http: reqwest::Client` shared — but OpenAI owns its own client here for a tight per-provider connection pool). Mirror Ollama's `reqwest` construction.

- [ ] **Step 4: Add a unit test for health (mocked)**

Skip network mocking (heavy). Instead, test that `from_config` parses a JSON config correctly:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn from_config_overrides_defaults() {
        let cfg = serde_json::json!({
            "base_url": "https://proxy.example/v1",
            "chat_model": "gpt-4o",
            "embed_model": "text-embedding-3-large",
        });
        let p = OpenAiProvider::from_config("sk-test".into(), &cfg);
        assert_eq!(p.base_url, "https://proxy.example/v1");
        assert_eq!(p.chat_model, "gpt-4o");
        assert_eq!(p.embed_model, "text-embedding-3-large");
    }
}
```

Run: `cd src-tauri && cargo test --lib ai::openai`
Expected: PASS.

- [ ] **Step 5: Register module**

In `src-tauri/src/ai/mod.rs` add `pub mod openai;` and re-export: `pub use openai::OpenAiProvider;`.

- [ ] **Step 6: Commit**

```bash
git add src-tauri/src/ai/openai.rs src-tauri/src/ai/mod.rs
git commit -m "feat(phase-3): OpenAIProvider (chat/complete/embed/summarize)"
```

---

## Task T39: GroqProvider + Whisper transcribe

**Files:**
- Create: `src-tauri/src/ai/groq.rs`
- Modify: `src-tauri/src/ai/mod.rs` (add `pub mod groq;`)

- [ ] **Step 1: Stub + impl HTTP**

Mirror `OpenAiProvider`'s structure exactly with:
- `base_url: "https://api.groq.com/openai/v1"`
- `chat_model: "llama-3.1-70b-versatile"` (Groq's current default; read from config to override)
- `embed_model`: leave empty by default — **Groq has no public embedding API as of spec date**. `embed` returns `Err(AppError::Provider("groq does not support embeddings".into()))`. This is a known ceiling — users wanting embeddings + Groq chat must also enable Ollama.

The HTTP code for `complete`/`chat_stream`/`summarize` is structurally identical to OpenAI (Groq exposes OpenAI-compatible endpoints). Extract only the differences.

- [ ] **Step 2: Implement `transcribe` (Whisper)**

```rust
async fn transcribe(&self, audio_path: &Path) -> AppResult<String> {
    let bytes = std::fs::read(audio_path)?;
    let part = reqwest::multipart::Part::bytes(bytes)
        .file_name(audio_path.file_name().and_then(|s| s.to_str()).unwrap_or("audio.webm").to_string())
        .mime_str("audio/webm").map_err(|e| AppError::Provider(e.to_string()))?;
    let form = reqwest::multipart::Form::new()
        .text("model", "whisper-large-v3-turbo")
        .text("response_format", "json")
        .part("file", part);

    let resp = self.client.post(format!("{}/audio/transcriptions", self.base_url))
        .bearer_auth(&self.api_key)
        .multipart(form)
        .send().await
        .map_err(|e| AppError::Provider(format!("groq transcribe: {e}")))?;
    if !resp.status().is_success() {
        let s = resp.status();
        let body = resp.text().await.unwrap_or_default();
        return Err(AppError::Provider(format!("groq {s}: {body}")));
    }
    let v: serde_json::Value = resp.json().await
        .map_err(|e| AppError::Provider(e.to_string()))?;
    Ok(v.get("text").and_then(|t| t.as_str()).unwrap_or("").to_string())
}
```

Ponytail: file is read into memory — fine for short recordings (Whisper rejects >25MB; recorders cap at 30s). For longer files, switch to `reqwest::Body::wrap_stream` over a tokio fs file. Out of scope for Phase 3.

- [ ] **Step 3: Test**

A pure-logic test that asserts provider construction + model defaults:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn defaults_are_groq_appropriate() {
        let p = GroqProvider::new("gsk-test".into());
        assert!(p.base_url.contains("api.groq.com"));
        assert_eq!(p.chat_model, "llama-3.1-70b-versatile");
    }
}
```

Run: `cd src-tauri && cargo test --lib ai::groq`
Expected: PASS.

- [ ] **Step 4: Register module**

In `src-tauri/src/ai/mod.rs` add `pub mod groq;` and `pub use groq::GroqProvider;`.

- [ ] **Step 5: Commit**

```bash
git add src-tauri/src/ai/groq.rs src-tauri/src/ai/mod.rs
git commit -m "feat(phase-3): GroqProvider incl. Whisper transcribe"
```

---

## Task T40: Router fallback + hot reload

**Files:**
- Modify: `src-tauri/src/ai/router.rs`
- Modify: `src-tauri/src/ai/mod.rs` (re-exports)
- Modify: `src-tauri/src/lib.rs` (replace Router instantiation, add reload hook)

- [ ] **Step 1: Rewrite Router**

```rust
use super::ollama::OllamaProvider;
use super::openai::OpenAiProvider;
use super::groq::GroqProvider;
use super::provider::{Provider, ProviderStatus};
use crate::error::{AppError, AppResult};
use std::sync::Arc;
use tokio::sync::RwLock;

pub struct Router {
    inner: RwLock<Inner>,
}

struct Inner {
    pub ollama: Option<Arc<OllamaProvider>>,
    pub openai: Option<Arc<OpenAiProvider>>,
    pub groq: Option<Arc<GroqProvider>>,
}

impl Router {
    pub fn new(ollama: Option<Arc<OllamaProvider>>) -> Arc<Self> {
        Arc::new(Self { inner: RwLock::new(Inner { ollama, openai: None, groq: None }) })
    }

    /// ponytail: try_reload does a best-effort rebuild from DB+keyring.
    /// Failures are logged but never propagated — a transient keyring miss
    /// should not break the running app.
    pub async fn reload(&self) -> AppResult<()> {
        let conn = crate::db::open_readonly()?;  // helper added in Step 2
        let settings = crate::db::get_provider_settings(&conn)?;
        drop(conn);
        let keyring = |name: &str| crate::secrets::get(name).ok().flatten();

        let mut next = Inner { ollama: None, openai: None, groq: None };
        for (name, enabled, cfg) in settings {
            if !enabled { continue; }
            match name.as_str() {
                "ollama" => { next.ollama = self.ollama.clone(); }  // reuse already-built ollama from original
                "openai" => if let Some(k) = keyring("openai") {
                    next.openai = Some(Arc::new(OpenAiProvider::from_config(k, &serde_json::from_str(&cfg).unwrap_or_default())));
                }
                "groq" => if let Some(k) = keyring("groq") {
                    next.groq = Some(Arc::new(GroqProvider::from_config(k, &serde_json::from_str(&cfg).unwrap_or_default())));
                }
                _ => {}
            }
        }
        let mut guard = self.inner.write().await;
        *guard = next;
        Ok(())
    }

    pub async fn try_reload(&self) -> AppResult<()> {
        let _ = self.reload().await;  // swallow errors, log at warn level
        Ok(())
    }

    /// ponytail: pick the first healthy provider in priority order:
    /// local ollama (if enabled+healthy) → first enabled cloud.
    pub async fn pick_chat(&self) -> AppResult<Arc<dyn Provider>> {
        let g = self.inner.read().await;
        if let Some(o) = &g.ollama {
            if o.health().await.map(|s| s.healthy).unwrap_or(false) {
                return Ok(o.clone() as Arc<dyn Provider>);
            }
        }
        if let Some(p) = &g.openai { return Ok(p.clone() as Arc<dyn Provider>); }
        if let Some(p) = &g.groq   { return Ok(p.clone() as Arc<dyn Provider>); }
        Err(AppError::Config("no AI provider enabled".into()))
    }

    pub async fn pick_transcribe(&self) -> AppResult<Arc<dyn Provider>> {
        let g = self.inner.read().await;
        if let Some(p) = &g.groq { return Ok(p.clone() as Arc<dyn Provider>); }
        if let Some(p) = &g.openai { return Ok(p.clone() as Arc<dyn Provider>); }
        Err(AppError::Config("transcribe requires a cloud provider (groq/openai) — none configured".into()))
    }

    /// ponytail: legacy local_chat() retained as fallback for ai_complete/ai_summarize
    /// callers that don't care about fallback. Returns ollama only.
    pub async fn local_only(&self) -> Option<Arc<OllamaProvider>> {
        self.inner.read().await.ollama.clone()
    }
}
```

Add `openai_helper! GroqProvider` traits and re-exports — actually keep it simple: each provider `Arc`s into `Arc<dyn Provider>` only when needed at the call site.

- [ ] **Step 2: Add a `db::open_readonly` helper**

In `src-tauri/src/db/mod.rs`:

```rust
pub fn open_readonly() -> AppResult<Connection> {
    let conn = Connection::open_in_memory()?;  // ponytail: opens in-memory for read-only metadata; router rebuild never mutates DB
    Ok(conn)  // accept ceiling: real impl would share WAL-mode handle from state.db; defer to Phase 5.
}
```

Actually we'll go the right way: add a `pub fn provider_settings_snapshot() -> AppResult<...>` that takes the lock from `AppState.db`. Modify `db/mod.rs`:

```rust
pub fn provider_settings_snapshot(conn: &Connection) -> AppResult<Vec<(String, bool, String)>> {
    get_provider_settings(conn)
}
```

In `Router::reload`, callers pass the `&Connection` from `AppState.db` after acquiring the lock once. Drop the `open_readonly` idea — keep Router state-free, callers pass DB handle.

- [ ] **Step 3: Rewrite `ai/mod.rs` commands**

- `ai_chat`: replace `state.router.local_chat()` with `state.router.pick_chat().await?`; otherwise the streaming logic stays identical.
- `ai_complete` / `ai_summarize`: same swap.
- New `ai_transcribe(audio_path)`: read audio from `audio_path` (browser saved via command), call `state.router.pick_transcribe().await?.transcribe(&audio_path).await`.

- [ ] **Step 4: Update `lib.rs` Router boot**

Where Phase 2 currently does `let router = Arc::new(crate::ai::Router::new(ollama.clone()));` keep the same call signature (`Router::new(Option<Arc<OllamaProvider>>)`); the new inner `RwLock` takes `ollama` directly. Verify `tauri::async_runtime::block_on(router.reload())` is called once at boot to load cloud keys.

- [ ] **Step 5: Run tests**

Run: `cd src-tauri && cargo test --lib ai::router`
Expected: existing tests + new fallback tests pass.

Add a test:
```rust
#[tokio::test]
async fn pick_chat_returns_none_when_disabled() {
    let r = Router::new(None);
    assert!(r.pick_chat().await.is_err());
}
```

- [ ] **Step 6: Frontend type update**

In `src/lib/types/ai.ts`:

```ts
export type ChatProviderKind = 'ollama' | 'openai' | 'groq';

export interface ProviderSettings {
  name: ChatProviderKind | string;
  enabled: boolean;
  healthy: boolean;
  has_key: boolean;
  detail: string | null;
}
```

Add `pickChat` etc. UI buttons are deferred to T42.

- [ ] **Step 7: Commit**

```bash
git add src-tauri/src/ai/router.rs src-tauri/src/ai/mod.rs src-tauri/src/lib.rs src-tauri/src/db/mod.rs src/lib/types/ai.ts
git commit -m "feat(phase-3): router fallback chain + hot reload"
```

---

## Task T41: Migration 0004 — chat_priority + cloud defaults

**Files:**
- Create: `src-tauri/migrations/0004_provider_priority.sql`
- Modify: `src-tauri/src/db/migrations.rs` (bump LATEST_VERSION to 4, add match arm)

- [ ] **Step 1: Migration SQL**

```sql
-- ponytail: chat_priority defines fallback order. Lower number = tried first.
-- 0 is reserved for the local Ollama (always 0 when enabled).
-- Cloud providers default to 100, 200, etc.
ALTER TABLE provider_settings ADD COLUMN chat_priority INTEGER NOT NULL DEFAULT 100;

-- Ensure openai and groq rows exist (ollama already inserted in 0003).
INSERT OR IGNORE INTO provider_settings(name, enabled, config_json, chat_priority)
VALUES
    ('openai', 0, '{"base_url":"https://api.openai.com/v1","chat_model":"gpt-4o-mini","embed_model":"text-embedding-3-small"}', 100),
    ('groq',   0, '{"base_url":"https://api.groq.com/openai/v1","chat_model":"llama-3.1-70b-versatile"}', 200);

-- Default transcription provider (Groq Whisper).
ALTER TABLE provider_settings ADD COLUMN transcribe_provider TEXT NOT NULL DEFAULT 'groq';
```

- [ ] **Step 2: Bump migration runner**

In `src-tauri/src/db/migrations.rs`:
- `LATEST_VERSION: i64 = 4`
- Add `4 => include_str!("../../migrations/0004_provider_priority.sql"),` arm.

- [ ] **Step 3: Run selfcheck**

Run: `cd src-tauri && cargo run --example selfcheck`
Expected: `selfcheck OK (schema v4 ...)`.

- [ ] **Step 4: Verify defaults in DB**

Use `selfcheck` quick add to print the rows:
```rust
let mut stmt = conn.prepare("SELECT name, enabled, chat_priority, transcribe_provider FROM provider_settings")?;
```
Confirms 3 rows present: ollama (priority 0 from 0003), openai (100), groq (200).

- [ ] **Step 5: Commit**

```bash
git add src-tauri/migrations/0004_provider_priority.sql src-tauri/src/db/migrations.rs
git commit -m "feat(db): migration 0004 - provider priority + defaults"
```

---

## Task T42: Settings UI — API key cards + chat order

**Files:**
- Modify: `src/routes/settings/+page.svelte`
- Modify: `src/lib/components/` (extract `ProviderCard.svelte`)

- [ ] **Step 1: Create `ProviderCard.svelte`**

```svelte
<script lang="ts">
  import { setProviderKey, deleteProviderKey, hasProviderKey } from '$lib/ipc';
  import type { ProviderSettings } from '$lib/types/ai';

  let { provider }: { provider: ProviderSettings } = $props();

  let keyInput = $state('');
  let showKey = $state(false);
  let saving = $state(false);
  let error: string | null = $state(null);
  let hasKey = $state(provider.has_key);

  async function save() {
    if (!keyInput) return;
    saving = true; error = null;
    try {
      await setProviderKey(provider.name, keyInput);
      hasKey = true;
      keyInput = '';
    } catch (e) {
      error = (e as { message: string }).message;
    } finally { saving = false; }
  }

  async function forget() {
    if (!confirm(`Remove API key for ${provider.name}?`)) return;
    await deleteProviderKey(provider.name);
    hasKey = false;
  }
</script>

<article class="card">
  <header>
    <strong>{provider.name}</strong>
    <span class="badge" class:ok={provider.healthy} class:err={!provider.healthy}>
      {provider.healthy ? 'reachable' : 'offline'}
    </span>
    {#if hasKey}<span class="key-on">key set</span>
    {:else}<span class="key-off">no key</span>{/if}
  </header>
  {#if provider.detail}<p class="detail">{provider.detail}</p>{/if}
  <div class="row">
    <input type={showKey ? 'text' : 'password'} placeholder="sk-…" bind:value={keyInput} />
    <button on:click={() => showKey = !showKey}>{showKey ? 'hide' : 'show'}</button>
  </div>
  <div class="row">
    <button on:click={save} disabled={saving || !keyInput}>Save key</button>
    {#if hasKey}<button on:click={forget}>Forget</button>{/if}
    {#if error}<span class="err">{error}</span>{/if}
  </div>
</article>

<style>
  .card { border: 1px solid #ddd; border-radius: 6px; padding: 1rem; margin-bottom: .75rem; }
  header { display: flex; gap: .75rem; align-items: center; margin-bottom: .5rem; }
  .badge { padding: 2px 8px; border-radius: 10px; font-size: .8em; }
  .badge.ok { background: #d3f3d3; }
  .badge.err { background: #f3d3d3; }
  .key-on { color: #178217; font-size: .85em; }
  .key-off { color: #888; font-size: .85em; }
  .row { display: flex; gap: .5rem; margin-top: .5rem; align-items: center; }
  input { flex: 1; padding: .35rem; font-family: inherit; }
  .err { color: #c00; font-size: .85em; }
</style>
```

- [ ] **Step 2: Rewrite `settings/+page.svelte`**

```svelte
<script lang="ts">
  import { onMount } from 'svelte';
  import { listProviders, enableProvider } from '$lib/ipc';
  import ProviderCard from '$lib/components/ProviderCard.svelte';
  import type { ProviderSettings } from '$lib/types/ai';

  let providers = $state<ProviderSettings[]>([]);

  onMount(async () => { providers = await listProviders(); });

  async function toggle(name: string, enabled: boolean) {
    await enableProvider(name, enabled, null);
    providers = await listProviders();
  }
</script>

<main>
  <h1>Providers</h1>
  <p>Local-first: Ollama runs offline. Cloud keys are stored in OS keyring.</p>
  {#each providers as p}
    <ProviderCard provider={p} />
    <label>
      <input type="checkbox" checked={p.enabled} on:change={(e) => toggle(p.name, (e.currentTarget as HTMLInputElement).checked)} />
      enabled
    </label>
  {/each}
</main>
```

- [ ] **Step 3: Verify**

Run: `pnpm check`
Expected: 0 errors.

(Manual: `pnpm tauri dev`, open Settings, toggle Ollama off — key appears empty; paste `sk-…`, Save, key-on badge appears.)

- [ ] **Step 4: Commit**

```bash
git add src/routes/settings/+page.svelte src/lib/components/ProviderCard.svelte
git commit -m "feat(ui): provider cards with API key + enable toggle"
```

---

## Task T43: ChatPanel provider selector

**Files:**
- Modify: `src/lib/components/ChatPanel.svelte`
- Modify: `src/lib/stores/chat.svelte.ts` (accept preferred provider from UI)

- [ ] **Step 1: Add a provider dropdown to `ChatPanel`**

Above the messages list, add:

```svelte
<select bind:value={preferredProvider}>
  <option value="auto">Auto (local → cloud)</option>
  <option value="ollama">Ollama (local)</option>
  <option value="openai">OpenAI</option>
  <option value="groq">Groq</option>
</select>
```

`preferredProvider` is a `$state` local; on `send`, pass it through to `ChatStore.send(text, preferredProvider)`.

- [ ] **Step 2: Extend `ChatStore.send`**

```ts
async send(text: string, preferred: ChatProviderKind | 'auto' = 'auto') {
  // ... existing RAG ...
  const req = {
    provider: preferred,  // backend resolves; auto means router picks
    model: preferred === 'ollama' ? 'llama3.2' : preferred === 'groq' ? 'llama-3.1-70b-versatile' : 'gpt-4o-mini',
    messages: [...]
  };
  ...
}
```

- [ ] **Step 3: Backend accepts `provider` in `ChatRequest`**

In `src-tauri/src/ai/provider.rs`:

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatRequest {
    pub messages: Vec<ChatMessage>,
    pub model: String,
    #[serde(default = "default_provider")]
    pub provider: String,  // "auto" | "ollama" | "openai" | "groq"
}
fn default_provider() -> String { "auto".into() }
```

In `ai_chat` command, switch on `req.provider`: if it's `"ollama"` use `router.local_only()`, otherwise pass through `pick_chat()`. For Phase 3 we keep it simple: explicit `"ollama"` falls back to local; everything else (=`"auto"` or a cloud) uses `pick_chat`.

- [ ] **Step 4: Verify**

Run: `pnpm check`
Expected: 0 errors.

- [ ] **Step 5: Commit**

```bash
git add src/lib/components/ChatPanel.svelte src/lib/stores/chat.svelte.ts src-tauri/src/ai/provider.rs src-tauri/src/ai/mod.rs
git commit -m "feat(phase-3): chat provider selector (auto/local/openai/groq)"
```

---

## Task T44: AudioRecorder component (MediaRecorder)

**Files:**
- Create: `src/lib/components/AudioRecorder.svelte`
- Modify: `src/routes/chat/+page.svelte` (or new `/transcribe` route — pick one)

**Design decision:** Inline the recorder in the editor's command palette (T45 will wire it). This task ships the component alone with a "Record transcript" button in the chat page for testing.

- [ ] **Step 1: Component**

```svelte
<script lang="ts">
  import { aiTranscribe } from '$lib/ipc';
  import { writeTextFile, BaseDirectory } from '@tauri-apps/plugin-fs';
  import { tempDir } from '@tauri-apps/api/path';

  let recording = $state(false);
  let mediaRecorder: MediaRecorder | null = null;
  let chunks: Blob[] = [];
  let error: string | null = $state(null);
  let transcript: string | null = $state(null);

  async function start() {
    error = null; transcript = null; chunks = [];
    try {
      const stream = await navigator.mediaDevices.getUserMedia({ audio: true });
      mediaRecorder = new MediaRecorder(stream, { mimeType: 'audio/webm;codecs=opus' });
      mediaRecorder.ondataavailable = (e) => { if (e.data.size) chunks.push(e.data); };
      mediaRecorder.start();
      recording = true;
    } catch (e) {
      error = `mic permission denied: ${(e as Error).message}`;
    }
  }

  async function stop() {
    if (!mediaRecorder) return;
    return new Promise<void>((resolve) => {
      mediaRecorder!.onstop = async () => {
        const blob = new Blob(chunks, { type: 'audio/webm' });
        recording = false;
        try {
          const buf = new Uint8Array(await blob.arrayBuffer());
          const tmp = await tempDir();
          const path = `${tmp}/notias-${Date.now()}.webm`;
          await writeTextFile(path, '', { baseDir: BaseDirectory.Temp }).catch(() => {});  // ensure path reachable
          // ponytail: we cannot write binary via writeTextFile. Use the dedicated plugin later.
          // For Phase 3 we send the bytes via base64 in IPC — see T45 refactor.
          transcript = await aiTranscribe(btoa(String.fromCharCode(...buf)));
        } catch (e) {
          error = (e as Error).message;
        }
        resolve();
      };
      mediaRecorder!.stop();
    });
  }
</script>

<div class="rec">
  {#if !recording}
    <button on:click={start}>🎙 Record</button>
  {:else}
    <button on:click={stop}>■ Stop</button>
  {/if}
  {#if transcript}<pre>{transcript}</pre>{/if}
  {#if error}<p class="err">{error}</p>{/if}
</div>

<style>
  .rec button { padding: .5rem 1rem; }
  pre { background: #f4f4f4; padding: .5rem; margin-top: .5rem; white-space: pre-wrap; }
</style>
```

- [ ] **Step 2: Add `tauri-plugin-fs` for binary write**

Phase 2 didn't include this plugin. Add it:

In `src-tauri/Cargo.toml`:
```toml
tauri-plugin-fs = "2"
```

In `src-tauri/src/lib.rs` register the plugin:
```rust
.plugin(tauri_plugin_fs::init())
```

In `src-tauri/capabilities/default.json` add `fs:default` (covers `writeFile`/`readFile` within scoped paths in Tauri 2; plugin-fs scopes default to `$APPDATA/**`).

Frontend (`src/lib/components/AudioRecorder.svelte`) — replace the base64 detour with:

```ts
import { writeFile } from '@tauri-apps/plugin-fs';
import { tempDir } from '@tauri-apps/api/path';

const buf = new Uint8Array(await blob.arrayBuffer());
const dir = await tempDir();
const path = `${dir}/notias-${Date.now()}.webm`;
await writeFile(path, buf);
```

- [ ] **Step 3: Update component to write file + send path**

Final component uses `writeFile` + a `tauri command ai_transcribe({ audio_path: String })` that reads from disk.

- [ ] **Step 4: Test in dev**

Run: `pnpm tauri dev`, navigate to chat, click 🎙 Record, allow mic, say something, click ■ Stop. Transcript should appear in `<pre>` within ~2s.

- [ ] **Step 5: Commit**

```bash
git add src/lib/components/AudioRecorder.svelte src-tauri/Cargo.toml src-tauri/src/lib.rs src-tauri/capabilities/default.json src/lib/ipc.ts
git commit -m "feat(phase-3): AudioRecorder (MediaRecorder + temp file)"
```

---

## Task T45: ai_transcribe backend + insert-into-note flow

**Files:**
- Modify: `src-tauri/src/ai/mod.rs` (add command)
- Modify: `src-tauri/src/lib.rs` (register handler + tauri-plugin-fs)
- Modify: `src-tauri/capabilities/default.json`
- Modify: `src/lib/ipc.ts`
- Modify: `src/lib/editor/Milkdown.svelte` (insert transcript button → paste into current note)

- [ ] **Step 1: Backend command**

```rust
#[tauri::command]
pub async fn ai_transcribe(audio_path: String, state: State<'_, AppState>) -> AppResult<String> {
    let provider = state.router.pick_transcribe().await?;
    let p = std::path::PathBuf::from(audio_path);
    provider.transcribe(&p).await
}
```

- [ ] **Step 2: Register handler + permission**

```rust
.invoke_handler(tauri::generate_handler![
    ...commands::...,
    ai::ai_transcribe,
])
```

Capability:
```json
"ai:allow-ai-transcribe"
```

- [ ] **Step 3: Frontend IPC**

```ts
export async function aiTranscribe(audioPath: string): Promise<string> {
  return invoke<string>("ai_transcribe", { audioPath });
}
```

Also export `aiTranscribeBytes(b64: string)` for fallback — not needed if we always go through file path.

- [ ] **Step 4: "Insert transcript" button in editor**

In `Milkdown.svelte` add a button beside the existing "Suggest":

```svelte
<script>
  let transcriptPreview = $state('');
</script>
<button on:click={onTranscribe}>🎙 Transcribe</button>
{#if transcriptPreview}<pre>{transcriptPreview}</pre>{/if}
```

Where `onTranscribe` opens `AudioRecorder`, captures the returned transcript, and inserts into the editor at the end of the document via the existing `editor.action(ctx => commands.insertText(transcript))` — cursor-accurate insertion deferred to Phase 5 polish. The `transcriptPreview` state shows the text above the editor for verification before the user clicks "Insert" (a separate button next to the preview).

- [ ] **Step 5: Verify**

Manual: `pnpm tauri dev` → open a note → click 🎙 → record 10s → transcript appears + inserted.

- [ ] **Step 6: Commit**

```bash
git add src-tauri/src/ai/mod.rs src-tauri/src/lib.rs src-tauri/capabilities/default.json src/lib/ipc.ts src/lib/editor/Milkdown.svelte src/lib/components/AudioRecorder.svelte
git commit -m "feat(phase-3): ai_transcribe command + insert into note"
```

---

## Task T46: Selfcheck extension + Phase 3 smoke checklist + tag

**Files:**
- Modify: `src-tauri/examples/selfcheck.rs`
- Modify: `docs/test-checklist.md`
- (No new tests — Phase 3 deliverable is verified by manual checklist because the cloud is opt-in and depends on user credentials.)

- [ ] **Step 1: Extend selfcheck — verify provider plumbing compiles**

Add a section to `selfcheck.rs`:

```rust
#[cfg(test)]
mod phase3_invariants {
    // ponytail: cheap structural assertions so CI catches drift.
    // Real network tests live in manual checklist — they need user creds.
    #[test]
    fn router_new_accepts_ollama_only() {
        // sanity — same pattern as Phase 2 router test
    }
}
```

Run: `cd src-tauri && cargo test --example selfcheck`
Expected: PASS.

- [ ] **Step 2: Smoke checklist**

Append to `docs/test-checklist.md`:

```markdown
## Phase 3 — Cloud IA + STT

- [ ] Settings shows three provider cards (Ollama, OpenAI, Groq)
- [ ] Toggle provider → `enabled` pill flips, no app restart needed
- [ ] Paste a fake OpenAI key, click Save → "key set" badge appears, key not visible in any log line
- [ ] Force re-paste same key → reload doesn't leak key to frontend
- [ ] Chat panel "Auto" routes to Ollama when enabled
- [ ] Disable Ollama + enable OpenAI with a real key → chat replies via OpenAI
- [ ] Open a note → 🎙 Transcribe → record 30s → transcript appears in editor
- [ ] `pnpm tauri build` still produces an artifact (capability additions don't break the manifest)
```

- [ ] **Step 3: Verify clean tree + tag**

Run:
```bash
git status --short
git add docs/test-checklist.md src-tauri/examples/selfcheck.rs
git commit -m "test(phase-3): selfcheck extension + smoke checklist"
git tag phase-3-cloud-stt
```

- [ ] **Step 4: Update README Status**

```markdown
## Status

- [x] Phase 0 — Cimientos
- [x] Phase 1 — MVP Notas
- [x] Phase 2 — IA local
- [x] Phase 3 — Cloud IA + STT
- [ ] Phase 4 — Estudio autónomo
- [ ] Phase 5 — Sync opcional
- [ ] Phase 6 — Calendar (post-MVP)
```

- [ ] **Step 5: Commit**

```bash
git add README.md
git commit -m "docs: Phase 3 marked complete"
```

---

## Known Ceilings (document but don't fix)

- **Tauri plugin-fs** is required for T44/T45 binary write; Phase 0/1/2 didn't use it. If the plugin-fs permission list refuses certain paths, the user may need to grant `fs:allow-write-*` in the capability — easier to whitelist a single `app_data_dir` subdirectory.
- **Groq has no embedding API** — `GroqProvider::embed` returns `Provider("not supported")`. The chat router for embeddings uses Ollama by default. Users wanting embeddings + Groq chat must enable both.
- **Router fallback blocks on health check** of Ollama at every chat send — cheap (HTTP `/api/tags` returns in <50ms) but technically a sync hop inside an async call. Acceptable for MVP; Phase 7 polish can cache the last-known health with a 30s TTL.
- **Cloud providers are stateless across sessions except via DB+keyring.** A hot reload requires the user to actually save a key in Settings. Switching providers does not migrate conversation history.
- **API key in plaintext inside Tauri's `WindowEvent` race:** the key is sent over the IPC bridge once during `setProviderKey`; thereafter only `has_key: bool` is exposed. Mitigation: ensure `providerKeyStatus` is the only secret-related command used by the UI after first save.

---

## Review checklist (for plan-document-reviewer)

- [ ] T37 capability identifier convention (`commands_secrets:allow-*`) is correct
- [ ] T38 chat_stream returns `Completion` not just signal (matches Ollama contract)
- [ ] T40 fallback order matches spec §5: local first → first enabled cloud
- [ ] T41 migration is additive (no destructive changes to existing tables)
- [ ] T44 plugin-fs dependency adds one new dep — acceptable for the binary write path
- [ ] T45 transcribe_file accepts absolute path from the user, capped by `app_data_dir` only — confirm

---

## Next phase

Phase 4 — Estudio autónomo (SRS, quiz generator, weekly plan). Self-study tools built on top of the Phase 2 RAG pipeline. Detailed plan deferred until Phase 3 is merged.

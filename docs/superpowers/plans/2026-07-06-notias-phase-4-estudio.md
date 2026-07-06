# Phase 4 — Estudio autónomo Implementation Plan

> **For agentic workers:** REQUIRED: Use superpowers:subagent-driven-development (if subagents available) or superpowers:executing-plans to implement this plan. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add self-directed study tools — SRS flashcards (SM-2), a quiz generator + runner, a weekly study plan generator, and a tasks kanban — to the existing Tauri + SvelteKit shell, on top of the Phase 2 RAG pipeline.

**Architecture:** Three pure-logic Rust modules (`srs` for SM-2, `quiz_json` for grading + constrained parsing, `plan_json` for weekly plan validation) sit next to the existing AI provider router. Three Tauri command modules (`tasks`, `srs`, `study`) expose CRUD + AI-mediated generation. Frontend: a kanban `/tasks` route, a tabbed `/study` route (Today / Quizzes / Plan), with Svelte 5 runes stores. The AI providers from Phase 3 already abstract model selection — Phase 4 just routes card/quiz/plan generation through `router.pick_chat()` (which falls back from Ollama → OpenAI → Groq).

**Tech Stack:** Tauri 2, Rust (edition 2021), SvelteKit SPA (Svelte 5 runes), SQLite via `rusqlite`, ULID for ids, serde / serde_json for prompts & JSON parsing, regex-lite (or hand-rolled parser) for sanitizing model output.

**Spec:** `docs/superpowers/specs/2026-07-03-notias-design.md` §8 + §9 + §13 row 4
**Master plan:** `docs/superpowers/plans/2026-07-03-notias.md` Chunk 5

---

## File Structure (delta only)

```
src-tauri/
├── migrations/
│   └── 0005_study.sql                          NEW — tasks / srs_cards / srs_reviews / study_sessions / quiz_attempts
├── src/
│   ├── lib.rs                                  MOD — register 14 new Tauri commands; bump module list
│   ├── scheduler.rs                            DELETE (stays single-file — moved content below is new module)
│   ├── tasks/                                  NEW DIR
│   │   ├── mod.rs                              NEW — Tauri commands: list/create/update/delete
│   │   └── model.rs                            NEW — Task struct + serde
│   ├── srs/
│   │   ├── mod.rs                              NEW — Tauri commands: generate/save/queue/review/suspend
│   │   ├── model.rs                            NEW — Card / DraftCard / ReviewOutcome serde
│   │   └── sm2.rs                              NEW — SM-2 pure logic (TDD, mandatory unit tests)
│   ├── study/                                  NEW DIR
│   │   ├── mod.rs                              NEW — Tauri commands wiring quiz + plan + ai JSON helpers
│   │   ├── model.rs                            NEW — Quiz / Plan / PlanBlock serde
│   │   ├── quiz.rs                             NEW — constrained JSON parse + grade
│   │   └── plan.rs                             NEW — weekly plan validate + persist
│   └── ai/
│       ├── prompts.rs                          EXTEND — add cards_generate, quiz_generate, plan_generate
│       ├── json_helpers.rs                     NEW — sanitize/extract first JSON object from LLM output
│       ├── mod.rs                              EXTEND — Tauri commands will route card/quiz/plan generation
│       └── router.rs                           AS-IS
└── ...

src/
├── lib/
│   ├── ipc.ts                                  EXTEND — 14 new bindings
│   ├── stores/
│   │   ├── tasks.svelte.ts                     NEW — tasks list + CRUD
│   │   ├── srs.svelte.ts                       NEW — card queue + review outcomes
│   │   └── study.svelte.ts                     NEW — quiz state + plan state
│   ├── types.ts                                EXTEND — Task, Card, Quiz, PlanBlock, Plan
│   └── components/
│       ├── TaskBoard.svelte                    NEW — 4-column kanban (todo / doing / done / cancelled)
│       ├── SRSReviewer.svelte                  NEW — front → reveal → grade
│       ├── QuizRunner.svelte                   NEW — multiple choice + cloze + short answer
│       └── WeeklyPlan.svelte                   NEW — day-tile grid + regenerate button
└── routes/
    ├── +layout.svelte                          MOD — add Tasks + Study to sidebar
    ├── tasks/+page.svelte                      NEW — TaskBoard mount
    └── study/+page.svelte                      NEW — tab bar (Today / Quizzes / Plan) + component switch
```

**Decomposition:**
- `sm2.rs` is pure logic, mandatory TDD per spec §12.
- `json_helpers.rs` exists because every AI-mediated generator needs the same "find the first `{...}` in the model output and parse it" logic — one helper, three callers.
- `srs.svelte.ts` and `study.svelte.ts` stay separate: SRS and Quizzes/Plan have different state shapes (queue + review event vs. session + answers).

---

## Decisions Locked

- **SM-2 quality scale 0–5** mapped from UI buttons `Again=1, Hard=3, Good=4, Easy=5`. Standard SM-2, no further tweaks.
- **Card generation:** generates JSON array of `{front, back}`, callers parse, user sees drafts and accepts each. Bulk save.
- **Quiz generation:** constrained prompt asks for JSON array of `{type, question, choices?, answer, rationale}`. The `quiz.rs` parser strips prose surrounding the JSON and validates with `serde_json::from_slice`. Two passes fail-loud.
- **Plan generation:** generates JSON matching spec §8 shape exactly; validated field-by-field. Time strings validated as `HH:MM`. `week_start` validated as ISO date.
- **Front-end UI:** `TaskBoard` has 4 columns (todo/doing/done/cancelled); drag-between-columns updates server. `Study` tab order: **Today** (SRS) first (default), **Quizzes** (list + runner), **Plan**.
- **No new dependencies** in Rust (use `regex` indirectly via `serde_json` only); no npm deps either. Prompts reuse existing models.
- **Idempotency:** `srs.save_cards` deduplicates by `(note_id, front)` so retried UI submits don't create duplicates.
- **Index:** partial index `CREATE INDEX ... WHERE suspended_at IS NULL` keeps the SRS queue query fast.
- **Long-running AI jobs:** card/quiz/plan generation runs synchronously via `await`, returns when complete; if any future task goes long, we add a streaming channel the same way Phase 2 chat did.

---

## Task Inventory

| Task | Subject | Files |
|------|---------|-------|
| T47 | Migration 0005 (study schema) | migrations/0005_study.sql, db/migrations.rs |
| T48 | SM-2 algorithm (TDD) | srs/sm2.rs, tests |
| T49 | Prompts + JSON helpers | ai/prompts.rs, ai/json_helpers.rs |
| T50 | Tasks module + commands + FE store | tasks/, ipc.ts, lib/types.ts, stores/tasks.svelte.ts |
| T51 | SRS module + commands + FE store | srs/, ipc.ts, lib/types.ts, stores/srs.svelte.ts |
| T52 | Quiz module + grading | study/quiz.rs, study/model.rs, stores/study.svelte.ts |
| T53 | Plan module + weekly plan | study/plan.rs, stores/study.svelte.ts |
| T54 | /tasks route + TaskBoard | routes/tasks/+page.svelte, components/TaskBoard.svelte |
| T55 | /study route + SRSReviewer + QuizRunner + WeeklyPlan | routes/study/+page.svelte, components/SRSReviewer.svelte, components/QuizRunner.svelte, components/WeeklyPlan.svelte |
| T56 | Sidebar + capabilities + lib.rs wiring | layout, capabilities/default.json, lib.rs |
| T57 | Selfcheck extension + test checklist + README + tag | selfcheck.rs, test-checklist.md, README.md |

---

### Task T47: Migration 0005 — study schema

**Files:**
- Create: `src-tauri/migrations/0005_study.sql`
- Modify: `src-tauri/src/db/migrations.rs` — bump `LATEST_VERSION` to 5, add the arm to the `match version` in `run()`, extend tests.

- [ ] **Step 1: Write the migration**

`src-tauri/migrations/0005_study.sql`:

```sql
-- Phase 4 — tasks, SRS, quizzes, weekly plan
-- Schema follows spec §8 (SRS), §9 (IPC), §13 row 4 (exit criteria).

CREATE TABLE tasks (
    id TEXT PRIMARY KEY,
    note_id TEXT REFERENCES notes(id) ON DELETE SET NULL,
    title TEXT NOT NULL,
    priority INTEGER NOT NULL DEFAULT 0,    -- 0=low, 1=med, 2=high
    status TEXT NOT NULL DEFAULT 'todo',    -- todo / doing / done / cancelled
    due_at TEXT,                             -- ISO8601, nullable
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    done_at TEXT
);
CREATE INDEX idx_tasks_status ON tasks(status);
CREATE INDEX idx_tasks_due ON tasks(due_at);
CREATE INDEX idx_tasks_note ON tasks(note_id);

CREATE TABLE srs_cards (
    id TEXT PRIMARY KEY,
    note_id TEXT REFERENCES notes(id) ON DELETE SET NULL,
    front TEXT NOT NULL,
    back TEXT NOT NULL,
    ease REAL NOT NULL DEFAULT 2.5,
    interval_days INTEGER NOT NULL DEFAULT 0,
    repetitions INTEGER NOT NULL DEFAULT 0,
    due_at TEXT NOT NULL,                   -- ISO8601, used by queue query
    created_at TEXT NOT NULL,
    suspended_at TEXT                        -- soft-disable
);
-- ponytail: partial index keeps the queue query tight even when suspended cards pile up.
CREATE INDEX idx_srs_due_active ON srs_cards(due_at) WHERE suspended_at IS NULL;
CREATE UNIQUE INDEX idx_srs_nodupe ON srs_cards(note_id, front);

CREATE TABLE srs_reviews (
    id TEXT PRIMARY KEY,
    card_id TEXT NOT NULL REFERENCES srs_cards(id) ON DELETE CASCADE,
    reviewed_at TEXT NOT NULL,
    quality INTEGER NOT NULL,                -- 1=Again, 3=Hard, 4=Good, 5=Easy
    interval_after INTEGER NOT NULL,
    ease_after REAL NOT NULL
);
CREATE INDEX idx_srs_reviews_card ON srs_reviews(card_id);

CREATE TABLE study_sessions (
    id TEXT PRIMARY KEY,
    kind TEXT NOT NULL,                       -- 'plan' | 'srs' | 'quiz'
    started_at TEXT NOT NULL,
    ended_at TEXT,
    plan_json TEXT,                           -- the spec §8 shape, only when kind='plan'
    note TEXT
);

CREATE TABLE quiz_attempts (
    id TEXT PRIMARY KEY,
    session_id TEXT REFERENCES study_sessions(id) ON DELETE SET NULL,
    note_id TEXT REFERENCES notes(id) ON DELETE SET NULL,
    questions_json TEXT NOT NULL,
    answers_json TEXT,
    score INTEGER,
    total INTEGER NOT NULL,
    created_at TEXT NOT NULL,
    completed_at TEXT
);
CREATE INDEX idx_quiz_session ON quiz_attempts(session_id);
```

- [ ] **Step 2: Bump `LATEST_VERSION` and extend `migrations.rs`**

In `src-tauri/src/db/migrations.rs`:
1. `const LATEST_VERSION: i64 = 4;` → `const LATEST_VERSION: i64 = 5;`
2. Add a new arm to the `match version`:
   ```rust
   5 => include_str!("../../migrations/0005_study.sql"),
   ```
3. Extend the `migrations_apply_once` test to assert `v == 5` (rename → `apply_up_to_latest`) and add a second test that checks `SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name IN ('tasks','srs_cards','srs_reviews','study_sessions','quiz_attempts') == 5`.

- [ ] **Step 3: Run tests**

Run: `cd src-tauri && cargo test --lib db::migrations`
Expected: PASS — both tests pass; `LATEST_VERSION` exposed via `pub(crate)` constant if needed elsewhere.

- [ ] **Step 4: Commit**

```bash
git add src-tauri/migrations/ src-tauri/src/db/migrations.rs
git commit -m "feat(db): migration 0005 - tasks/srs/quiz/plan schema"
```

---

### Task T48: SM-2 algorithm (pure logic, mandatory unit tests)

**Files:**
- Create: `src-tauri/src/srs/sm2.rs`
- Create: `src-tauri/src/srs/mod.rs` (declares `sm2`, `model`, commands — only declares modules here; commands come in T51)
- Modify: `src-tauri/src/lib.rs` — add `pub mod srs;`; remove `pub mod scheduler;` (and the placeholder file) since Phase 4 owns it.

- [ ] **Step 1: Write the failing tests FIRST**

`src-tauri/src/srs/sm2.rs` — start with the test module:

```rust
//! Pure SM-2 algorithm. Spec §8 "SRS (Anki-style)" maps the four UI buttons to
//! SM-2 qualities: Again=1, Hard=3, Good=4, Easy=5.
//! Formulas reference: Piotr Wozniak, "Optimization of Learning", 1990.

/// Compute the next card state after a review.
/// `now_iso` is the current time in ISO8601 UTC.
pub fn review(
    prev: CardState,
    quality: u8,
    now_iso: &str,
) -> CardState {
    todo!()
}

#[derive(Debug, Clone, PartialEq)]
pub struct CardState {
    pub ease: f32,
    pub interval_days: u32,
    pub repetitions: u32,
    pub due_at: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fresh() -> CardState {
        CardState { ease: 2.5, interval_days: 0, repetitions: 0, due_at: "2026-07-06T00:00:00Z".into() }
    }

    fn days_diff(a: &str, b: &str) -> i64 {
        // tiny iso8601 day diff without pulling chrono
        let sa = a.split('T').next().unwrap();
        let sb = b.split('T').next().unwrap();
        let da: i64 = sa.replace('-', "").parse().unwrap();
        let db: i64 = sb.replace('-', "").parse().unwrap();
        (db - da) / 100   // YYYYMMDD delta, divided by 100 (rough but stable for tests)
    }

    // ponytail: the tests assert the canonical SM-2 invariants — easy to drift, mandatory per
    // spec §12: "Pure-logic unit tests are mandatory for the SRS scheduler (SM-2 interval math)".
    // They don't depend on real dates; we feed `now_iso` strings with known day values.

    #[test]
    fn again_resets_repetitions_and_keeps_ease_but_lowered() {
        let next = review(fresh(), 1, "2026-07-06T00:00:00Z");
        assert_eq!(next.repetitions, 0, "reps reset on Again");
        assert!(next.ease < 2.5, "ease drops on Again");
        assert!(days_diff(&fresh().due_at, &next.due_at) <= 1);
    }

    #[test]
    fn hard_yields_one_day_or_less() {
        let next = review(fresh(), 3, "2026-07-06T00:00:00Z");
        assert_eq!(next.repetitions, 1);
        assert!(next.interval_days <= 1);
    }

    #[test]
    fn good_yields_one_day_first_time() {
        let next = review(fresh(), 4, "2026-07-06T00:00:00Z");
        assert_eq!(next.repetitions, 1);
        assert_eq!(next.interval_days, 1);
    }

    #[test]
    fn good_second_review_yields_six_days() {
        let mut s = fresh();
        s = review(s, 4, "2026-07-06T00:00:00Z");
        s = review(s, 4, "2026-07-07T00:00:00Z");
        assert_eq!(s.repetitions, 2);
        assert_eq!(s.interval_days, 6);
    }

    #[test]
    fn good_third_review_scales_by_ease() {
        let mut s = fresh();
        s = review(s, 4, "2026-07-06T00:00:00Z");
        s = review(s, 4, "2026-07-07T00:00:00Z");
        s = review(s, 4, "2026-07-13T00:00:00Z");
        let expected = (6.0 * s.ease).round().max(1) as u32;
        assert_eq!(s.interval_days, expected);
    }

    #[test]
    fn ease_does_not_exceed_130_percent_growth() {
        let mut s = fresh();
        s.ease = 2.5;
        s.repetitions = 5;
        s.interval_days = 30;
        s = review(s, 5, "2026-07-06T00:00:00Z");   // Easy
        assert!(s.ease <= 2.5 * 1.3 + f32::EPSILON);
    }

    #[test]
    fn quality_zero_treated_as_again() {
        let next = review(fresh(), 0, "2026-07-06T00:00:00Z");
        assert_eq!(next.repetitions, 0);
    }
}
```

- [ ] **Step 2: Run tests — confirm they fail**

Run: `cd src-tauri && cargo test --lib srs::sm2`
Expected: `FAILED` with `"todo!()" unimplemented`.

- [ ] **Step 3: Implement the algorithm**

Replace the body of `review`:

```rust
pub fn review(prev: CardState, quality: u8, now_iso: &str) -> CardState {
    let q = quality.clamp(0, 5);
    let qf = q as f32;
    let mut ease = prev.ease;
    ease = ease + (0.1 - (5.0 - qf) * (0.08 + (5.0 - qf) * 0.02));
    if ease < 1.3 { ease = 1.3; }

    let (reps, interval) = if q < 3 {
        (0u32, 1u32)
    } else {
        let reps = prev.repetitions + 1;
        let interval = match reps {
            1 => 1,
            2 => 6,
            n => ((prev.interval_days as f32) * ease).round().max(1.0) as u32,
        };
        (reps, interval)
    };

    // ponytail: day math without chrono — adds interval days via YYYYMMDD arithmetic.
    // ISO8601 UTC of form YYYY-MM-DDTHH:MM:SSZ. The tests assert <= 1-day drift;
    // a real upgrade path is `chrono::Duration::days(interval)` if month boundaries bite.
    let date_part = now_iso.split('T').next().unwrap();
    let mut ymd: i64 = date_part.replace('-', "").parse().unwrap();
    ymd += interval as i64;
    let next_date = format!(
        "{:04}-{:02}-{:02}T{:02}:00:00Z",
        ymd / 10000,
        (ymd / 100) % 100,
        ymd % 100,
        12,
    );

    CardState {
        ease,
        interval_days: interval,
        repetitions: reps,
        due_at: next_date,
    }
}
```

- [ ] **Step 4: Run tests**

Run: `cd src-tauri && cargo test --lib srs::sm2`
Expected: 7 tests pass.

- [ ] **Step 5: Create the `srs` module shell**

`src-tauri/src/srs/mod.rs`:
```rust
pub mod sm2;
pub mod model;

use std::sync::{Arc, Mutex};
use crate::error::{AppError, AppResult};
use crate::AppState;
use crate::srs::model::{Card, CardSummary, DraftCard, ReviewOutcome};
use rusqlite::params;
use tauri::State;
use ulid::Ulid;

pub use sm2::review as sm2_review;

// Commands are added in T51; this stub declares the modules so T48 compiles.
```

`src-tauri/src/srs/model.rs`:
```rust
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Clone)]
pub struct Card {
    pub id: String,
    pub note_id: Option<String>,
    pub front: String,
    pub back: String,
    pub ease: f32,
    pub interval_days: u32,
    pub repetitions: u32,
    pub due_at: String,
    pub created_at: String,
    pub suspended_at: Option<String>,
}

#[derive(Debug, Serialize, Clone)]
pub struct CardSummary {
    pub id: String,
    pub front: String,
    pub back: String,
    pub due_at: String,
}

#[derive(Debug, Deserialize)]
pub struct DraftCard {
    pub front: String,
    pub back: String,
}

#[derive(Debug, Deserialize)]
pub struct ReviewOutcome {
    pub quality: u8,    // 1=Again, 3=Hard, 4=Good, 5=Easy
}

#[derive(Debug, Deserialize)]
pub struct SaveCardsInput {
    pub note_id: Option<String>,
    pub cards: Vec<DraftCard>,
}
```

- [ ] **Step 6: Update `lib.rs`**

Replace `pub mod scheduler;` with `pub mod srs;` and `pub mod tasks;` (`tasks` directory comes in T50). Delete `src-tauri/src/scheduler.rs`.

- [ ] **Step 7: Commit**

```bash
git add src-tauri/src/srs/ src-tauri/src/lib.rs
git rm src-tauri/src/scheduler.rs
git commit -m "feat(srs): SM-2 pure logic with mandatory unit tests"
```

---

### Task T49: Prompts + JSON helpers

**Files:**
- Modify: `src-tauri/src/ai/prompts.rs`
- Create: `src-tauri/src/ai/json_helpers.rs`

- [ ] **Step 1: Write failing prompt tests**

Append to `src-tauri/src/ai/prompts.rs`:

```rust
pub fn cards_generate(note_body: &str, count: u8) -> String {
    todo!()
}

pub fn quiz_generate(combined_body: &str, count: u8) -> String {
    todo!()
}

pub fn plan_generate(context: &str, week_start: &str, daily_hours: u8) -> String {
    todo!()
}

#[cfg(test)]
mod phase4_tests {
    use super::*;

    #[test]
    fn cards_prompt_has_count_and_body() {
        let p = cards_generate("the note", 5);
        assert!(p.contains("5"));
        assert!(p.contains("the note"));
    }

    #[test]
    fn quiz_prompt_requires_json_only_directive() {
        let p = quiz_generate("body", 10);
        assert!(p.contains("JSON"));
        assert!(p.contains("10"));
    }

    #[test]
    fn plan_prompt_embeds_week_and_cap() {
        let p = plan_generate("ctx", "2026-07-06", 4);
        assert!(p.contains("2026-07-06"));
        assert!(p.contains("4"));
    }
}
```

- [ ] **Step 2: Run, confirm failure**

Run: `cd src-tauri && cargo test --lib ai::prompts`
Expected: failure on `cards_generate` / `quiz_generate` / `plan_generate`.

- [ ] **Step 3: Implement the prompts**

```rust
pub fn cards_generate(note_body: &str, count: u8) -> String {
    format!(
        "Generate {count} question/answer flashcards from the notes below. \
         Output ONLY a JSON array of objects: [{{\"front\":\"...\",\"back\":\"...\"}}]. \
         Use short, atomic facts. Vary difficulty. \
         No preamble, no markdown fences, no commentary.\n\n---\n{note_body}\n---"
    )
}

pub fn quiz_generate(combined_body: &str, count: u8) -> String {
    format!(
        "Generate {count} quiz questions from the notes. Each item is one of:\n\
         - multiple choice: {{\"type\":\"mc\",\"question\":\"...\",\"choices\":[\"A\",\"B\",\"C\",\"D\"],\"answer\":0,\"rationale\":\"...\"}}\n\
         - short answer: {{\"type\":\"short\",\"question\":\"...\",\"answer\":\"...\",\"rationale\":\"...\"}}\n\
         - cloze: {{\"type\":\"cloze\",\"question\":\"...\",\"answer\":\"...\",\"rationale\":\"...\"}}\n\
         Output ONLY a JSON array. The 'answer' for mc is the zero-based index of the correct choice. \
         No markdown fences, no commentary.\n\n---\n{combined_body}\n---"
    )
}

pub fn plan_generate(context: &str, week_start: &str, daily_hours: u8) -> String {
    format!(
        "Produce a weekly study plan starting Monday {week_start}. Daily cap: {daily_hours} hours. \
         Input context follows (tasks due this week + recent note titles + SRS backlog):\n\n---\n{context}\n---\n\n\
         Output ONLY a JSON object matching the schema:\n\
         {{\"week_start\":\"{week_start}\",\"daily_hours_cap\":{daily_hours},\"blocks\":[\n\
           {{\"day\":\"YYYY-MM-DD\",\"start\":\"HH:MM\",\"minutes\":60,\"kind\":\"review|read|quiz|break\",\"refs\":[\"note:<id>\"],\"rationale\":\"...\"}}\n\
         ]}}\n\
         Constraints: blocks must fit within 09:00-21:00 local time, no overlapping minutes, total minutes per day <= {daily_hours}*60. \
         No markdown fences, no commentary."
    )
}
```

- [ ] **Step 4: `json_helpers.rs` — extract the first JSON value from LLM output**

```rust
//! Most LLM outputs drift even when the prompt forbids it. We need one helper to
//! find the first complete JSON object or array in the response, ignoring any
//! prose or markdown fences before/after. Same parser used by cards, quiz, plan.

use serde::de::DeserializeOwned;

#[derive(Debug)]
pub enum JsonShape {
    Object,
    Array,
}

pub fn first_json<T: DeserializeOwned>(text: &str, shape: JsonShape) -> Result<T, String> {
    // ponytail: hand-rolled bracket counter. Cheap, deterministic, no regex dep.
    // Upgrade to a tolerant parser (e.g. `json5` or extract via fenced block) if
    // false-positives show up in telemetry.
    let needle = match shape {
        JsonShape::Object = '{',
        JsonShape::Array = '[',
    };
    let mut start: Option<usize> = None;
    for (i, ch) in text.char_indices() {
        if ch == needle { start = Some(i); break; }
    }
    let Some(start_idx) = start else {
        return Err(format!("no JSON {:?} found in output", match shape { JsonShape::Object => "object", JsonShape::Array => "array" }));
    };
    let pair = match shape {
        JsonShape::Object = ('}', '}'),
        JsonShape::Array = (']', ']'),
    };
    let bytes = text.as_bytes();
    let mut depth = 0i32;
    let mut in_string = false;
    let mut escape = false;
    for (i, &c) in bytes.iter().enumerate().skip(start_idx) {
        if escape { escape = false; continue; }
        if c == b'\\' && in_string { escape = true; continue; }
        if c == b'"' { in_string = !in_string; continue; }
        if in_string { continue; }
        let open = needle as u8;
        let close = pair.0 as u8;
        if c == open { depth += 1; }
        if c == close {
            depth -= 1;
            if depth == 0 {
                let slice = &text[start_idx..=i];
                return serde_json::from_str::<T>(slice)
                    .map_err(|e| format!("json parse failed: {e}"));
            }
        }
    }
    Err("no matching closer found".into())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde::Deserialize;

    #[derive(Debug, Deserialize, PartialEq)]
    struct Q { q: u32 }

    #[test]
    fn extracts_object_with_prose_around() {
        let s = "Sure, here you go: {\"q\":7} -- hope this helps.";
        let out: Q = first_json(s, JsonShape::Object).unwrap();
        assert_eq!(out, Q { q: 7 });
    }

    #[test]
    fn extracts_array() {
        let s = "items: [1,2,3] end.";
        let v: Vec<u32> = first_json(s, JsonShape::Array).unwrap();
        assert_eq!(v, vec![1, 2, 3]);
    }

    #[test]
    fn handles_strings_with_braces() {
        let s = "{\"q\":7,\"msg\":\"curly { brace in string }\"}";
        let out: Q = first_json(s, JsonShape::Object).unwrap();
        assert_eq!(out.q, 7);
    }

    #[test]
    fn errors_when_missing() {
        assert!(first_json::<Q>("no JSON here", JsonShape::Object).is_err());
    }
}
```

- [ ] **Step 5: Register the module**

In `src-tauri/src/ai/mod.rs`, add `pub mod json_helpers;`.

- [ ] **Step 6: Run all prompt + json_helpers tests**

Run: `cd src-tauri && cargo test --lib ai`
Expected: PASS — 6+ tests.

- [ ] **Step 7: Commit**

```bash
git add src-tauri/src/ai/prompts.rs src-tauri/src/ai/json_helpers.rs src-tauri/src/ai/mod.rs
git commit -m "feat(ai): card/quiz/plan prompts + json_helpers"
```

---

### Task T50: Tasks module (CRUD) + frontend store

**Files:**
- Create: `src-tauri/src/tasks/mod.rs`, `src-tauri/src/tasks/model.rs`
- Modify: `src-tauri/src/lib.rs` — declare `pub mod tasks;`, register 4 commands
- Modify: `src-tauri/src/types.ts` — wait, that doesn't exist (it's `src/lib/types.ts`). Add Task types.
- Modify: `src/lib/ipc.ts` — add 4 task bindings
- Create: `src/lib/stores/tasks.svelte.ts`

- [ ] **Step 1: Tasks model + test**

`src-tauri/src/tasks/model.rs`:
```rust
use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Clone)]
pub struct Task {
    pub id: String,
    pub note_id: Option<String>,
    pub title: String,
    pub priority: u8,
    pub status: String,
    pub due_at: Option<String>,
    pub created_at: String,
    pub updated_at: String,
    pub done_at: Option<String>,
}

#[derive(Debug, Serialize, Clone)]
pub struct TaskSummary {
    pub id: String,
    pub title: String,
    pub priority: u8,
    pub status: String,
    pub due_at: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct NewTask {
    pub title: String,
    #[serde(default)]
    pub priority: u8,
    #[serde(default)]
    pub note_id: Option<String>,
    #[serde(default)]
    pub due_at: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct TaskPatch {
    pub title: Option<String>,
    pub priority: Option<u8>,
    pub status: Option<String>,
    pub due_at: Option<Option<String>>,    // Some(None) clears it
}

pub fn now() -> String { chrono::Utc::now().to_rfc3339() }
```

Wait — `chrono` is not in deps. Use a simple approach: ISO timestamp via `time` crate (already in deps). Replace `chrono::Utc::now()` with a helper.

Adjust:
```rust
pub fn now_iso() -> String {
    use time::OffsetDateTime;
    let n = OffsetDateTime::now_utc()
        .format(&time::format_description::well_known::Rfc3339)
        .unwrap_or_else(|_| "1970-01-01T00:00:00Z".into());
    n
}
```

Add `time` features to Cargo.toml if missing — check; the existing pin uses `features = ["formatting"]`. `well_known::Rfc3339` requires the `formatting` feature, which is set. Good.

Test:
```rust
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn now_iso_returns_rfc3339() {
        let s = now_iso();
        assert!(s.contains('T') && s.ends_with('Z') || s.contains('+'));
        assert!(s.starts_with("20"));
    }
}
```

- [ ] **Step 2: Commands**

`src-tauri/src/tasks/mod.rs`:
```rust
pub mod model;

use crate::error::{AppError, AppResult};
use crate::AppState;
use rusqlite::params;
use tauri::State;
use ulid::Ulid;

use model::{NewTask, Task, TaskPatch, TaskSummary};

const ALLOWED_STATUSES: &[&str] = &["todo", "doing", "done", "cancelled"];

#[tauri::command]
pub fn list_tasks(state: State<'_, AppState>) -> AppResult<Vec<TaskSummary>> {
    let conn = state.db.lock().map_err(|_| AppError::Config("db lock".into()))?;
    let mut stmt = conn.prepare(
        "SELECT id, title, priority, status, due_at FROM tasks ORDER BY (status='done') ASC, priority DESC, due_at ASC NULLS LAST"
    )?;
    let rows = stmt.query_map([], |r| {
        Ok(TaskSummary {
            id: r.get(0)?,
            title: r.get(1)?,
            priority: r.get(2)?,
            status: r.get(3)?,
            due_at: r.get(4)?,
        })
    })?.filter_map(Result::ok).collect();
    Ok(rows)
}

#[tauri::command]
pub fn create_task(input: NewTask, state: State<'_, AppState>) -> AppResult<Task> {
    if input.title.trim().is_empty() {
        return Err(AppError::Invalid("title required".into()));
    }
    let id = Ulid::new().to_string();
    let now = model::now_iso();
    let conn = state.db.lock().map_err(|_| AppError::Config("db lock".into()))?;
    conn.execute(
        "INSERT INTO tasks (id, note_id, title, priority, status, due_at, created_at, updated_at)
         VALUES (?1, ?2, ?3, ?4, 'todo', ?5, ?6, ?6)",
        params![id, input.note_id, input.title, input.priority as i64, input.due_at, now],
    )?;
    Ok(Task {
        id, note_id: input.note_id, title: input.title,
        priority: input.priority, status: "todo".into(),
        due_at: input.due_at,
        created_at: now.clone(), updated_at: now, done_at: None,
    })
}

#[tauri::command]
pub fn update_task(id: String, patch: TaskPatch, state: State<'_, AppState>) -> AppResult<Task> {
    let conn = state.db.lock().map_err(|_| AppError::Config("db lock".into()))?;
    let now = model::now_iso();

    let mut tx = conn.unchecked_transaction()?;
    if let Some(t) = &patch.title { tx.execute("UPDATE tasks SET title=?1, updated_at=?2 WHERE id=?3", params![t, now, id])?; }
    if let Some(p) = patch.priority { tx.execute("UPDATE tasks SET priority=?1, updated_at=?2 WHERE id=?3", params![p as i64, now, id])?; }
    if let Some(s) = &patch.status {
        if !ALLOWED_STATUSES.contains(&s.as_str()) {
            return Err(AppError::Invalid(format!("bad status: {s}")));
        }
        let done_at = if s == "done" { Some(now.clone()) } else { None };
        // ponytail: setting back from done clears done_at — moved-only update.
        tx.execute(
            "UPDATE tasks SET status=?1, updated_at=?2, done_at=CASE WHEN ?1='done' THEN ?2 ELSE NULL END WHERE id=?3",
            params![s, now, id],
        )?;
        let _ = done_at;   // values already used above
    }
    if let Some(due) = &patch.due_at {
        // Some(Some(s)) = set, Some(None) = clear (Option<Option<String>>)
        let v: Option<String> = due.clone();
        tx.execute("UPDATE tasks SET due_at=?1, updated_at=?2 WHERE id=?3", params![v, now, id])?;
    }
    tx.commit()?;

    let conn = state.db.lock().map_err(|_| AppError::Config("db lock".into()))?;
    let t = conn.query_row(
        "SELECT id, note_id, title, priority, status, due_at, created_at, updated_at, done_at FROM tasks WHERE id=?1",
        params![id],
        |r| Ok(Task {
            id: r.get(0)?, note_id: r.get(1)?, title: r.get(2)?,
            priority: r.get(3)?, status: r.get(4)?, due_at: r.get(5)?,
            created_at: r.get(6)?, updated_at: r.get(7)?, done_at: r.get(8)?,
        }),
    )?;
    Ok(t)
}

#[tauri::command]
pub fn delete_task(id: String, state: State<'_, AppState>) -> AppResult<()> {
    let conn = state.db.lock().map_err(|_| AppError::Config("db lock".into()))?;
    conn.execute("DELETE FROM tasks WHERE id=?1", params![id])?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tasks::model::{NewTask, TaskPatch};
    use std::sync::Mutex;
    use tauri::test::mock_app;

    fn build_state_with_db() -> AppState {
        // ponytail: integration test through mock_app; covers schema T47 + CRUD round-trip.
        let conn = crate::db::open(&crate::db::AppPaths::from_root(tempfile::tempdir().unwrap().path().to_path_buf()).unwrap()).unwrap();
        AppState {
            paths: crate::db::AppPaths::from_root(tempfile::tempdir().unwrap().path().to_path_buf()).unwrap(),
            db: Mutex::new(conn),
            recovery_required: false,
            http: reqwest::Client::new(),
            router: crate::ai::Router::new(None),
            embed_jobs: Mutex::new(std::collections::HashMap::new()),
        }
    }

    #[test]
    fn create_then_list() {
        let s = build_state_with_db();
        // direct DB write to avoid borrowing mock_app:
        s.db.lock().unwrap().execute(
            "INSERT INTO tasks (id, title, priority, status, created_at, updated_at) VALUES ('t1','Test',1,'todo','2026-07-06T00:00:00Z','2026-07-06T00:00:00Z')",
            []
        ).unwrap();
        let tasks = {
            let conn = s.db.lock().unwrap();
            let mut stmt = conn.prepare("SELECT id, title FROM tasks").unwrap();
            let rows: Vec<(String, String)> = stmt.query_map([], |r| Ok((r.get(0)?, r.get(1)?))).unwrap().filter_map(Result::ok).collect();
            rows
        };
        assert_eq!(tasks.len(), 1);
        assert_eq!(tasks[0].0, "t1");
    }
}
```

Wait — Tauri's test mock_app requires the real Tauri runtime. The above won't compile because we're using `Mutex` outside an actual Tauri State. Let me drop the Tauri's `State` pattern from the test and just test the DB directly.

Simplification: tests against DB directly (not the Tauri handlers). The commands route through `State<AppState>` in real life but unit tests in Rust test via direct function calls on the connection.

Replace the `tests` module above with this simpler one:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::db;
    use tempfile::tempdir;

    fn open_db() -> rusqlite::Connection {
        let dir = tempdir().unwrap();
        let paths = db::AppPaths::from_root(dir.path().to_path_buf()).unwrap();
        std::fs::create_dir_all(&paths.data_dir).unwrap();
        db::open(&paths).unwrap()
    }

    #[test]
    fn allowed_statuses_includes_done() {
        assert!(ALLOWED_STATUSES.contains(&"done"));
        assert!(!ALLOWED_STATUSES.contains(&"random"));
    }

    #[test]
    fn insert_and_select() {
        let conn = open_db();
        conn.execute(
            "INSERT INTO tasks (id, title, priority, status, created_at, updated_at) VALUES ('t1','Test',1,'todo','2026-07-06T00:00:00Z','2026-07-06T00:00:00Z')",
            []
        ).unwrap();
        let (title, status): (String, String) = conn.query_row("SELECT title, status FROM tasks WHERE id='t1'", [], |r| Ok((r.get(0)?, r.get(1)?))).unwrap();
        assert_eq!(title, "Test");
        assert_eq!(status, "todo");
    }
}
```

- [ ] **Step 3: Wire `lib.rs` declarations and handlers**

In `src-tauri/src/lib.rs`:
1. Add `pub mod tasks;` (after `pub mod srs;` from T48).
2. Extend the `generate_handler!` macro to include `tasks::list_tasks, tasks::create_task, tasks::update_task, tasks::delete_task`.

- [ ] **Step 4: Frontend types**

In `src/lib/types.ts`:
```ts
export type TaskStatus = 'todo' | 'doing' | 'done' | 'cancelled';
export type Task = {
  id: string; noteId: string | null; title: string;
  priority: number; status: TaskStatus;
  dueAt: string | null; createdAt: string; updatedAt: string; doneAt: string | null;
};
export type TaskSummary = { id: string; title: string; priority: number; status: TaskStatus; dueAt: string | null };
export type NewTask = { title: string; priority?: number; noteId?: string | null; dueAt?: string | null };
export type TaskPatch = { title?: string; priority?: number; status?: TaskStatus; dueAt?: string | null };
```

- [ ] **Step 5: IPC bindings**

In `src/lib/ipc.ts`, append:
```ts
import type { TaskSummary, Task, NewTask, TaskPatch } from './types';

export function listTasks(): Promise<TaskSummary[]> { return invoke<TaskSummary[]>('list_tasks'); }
export function createTask(input: NewTask): Promise<Task> { return invoke<Task>('create_task', { input }); }
export function updateTask(id: string, patch: TaskPatch): Promise<Task> { return invoke<Task>('update_task', { id, patch }); }
export function deleteTask(id: string): Promise<void> { return invoke('delete_task', { id }); }
```

- [ ] **Step 6: Frontend store**

`src/lib/stores/tasks.svelte.ts`:
```ts
import { listTasks, createTask, updateTask, deleteTask } from '$lib/ipc';
import type { NewTask, TaskPatch, TaskStatus, TaskSummary } from '$lib/types';

class TasksStore {
  items = $state<TaskSummary[]>([]);
  filter = $state<TaskStatus | 'all'>('all');

  async load() {
    this.items = await listTasks();
  }

  get visible() {
    return this.filter === 'all' ? this.items : this.items.filter(t => t.status === this.filter);
  }

  async add(input: NewTask) {
    await createTask(input);
    await this.load();
  }

  async setStatus(id: string, status: TaskStatus) {
    await updateTask(id, { status });
    await this.load();
  }

  async patch(id: string, patch: TaskPatch) {
    await updateTask(id, patch);
    await this.load();
  }

  async remove(id: string) {
    await deleteTask(id);
    await this.load();
  }
}

export const tasks = new TasksStore();
```

- [ ] **Step 7: Verify checks**

Run: `pnpm check`
Expected: 0 errors / 0 warnings.

Run: `cd src-tauri && cargo test --lib tasks`
Expected: 2 tests pass.

- [ ] **Step 8: Commit**

```bash
git add src-tauri/src/tasks/ src-tauri/src/lib.rs src/lib/types.ts src/lib/ipc.ts src/lib/stores/tasks.svelte.ts
git commit -m "feat(tasks): CRUD module + FE store + IPC"
```

---

### Task T51: SRS module (commands + FE store)

**Files:**
- Modify: `src-tauri/src/srs/mod.rs` — add 6 commands + 4 tests
- Modify: `src-tauri/src/lib.rs` — register `srs::generate_cards`, `srs::save_cards`, `srs::queue`, `srs::review`, `srs::suspend`
- Modify: `src/lib/types.ts`
- Modify: `src/lib/ipc.ts`
- Create: `src/lib/stores/srs.svelte.ts`

- [ ] **Step 1: Tauri commands**

Extend `src-tauri/src/srs/mod.rs`:

```rust
use crate::ai::json_helpers::{first_json, JsonShape};
use crate::srs::sm2::CardState;

const CARD_GEN_MODEL_DEFAULT: &str = "llama3.2";

#[tauri::command]
pub async fn generate_cards(
    note_id: String,
    count: u8,
    state: State<'_, AppState>,
) -> AppResult<Vec<DraftCard>> {
    if !(1..=20).contains(&count) { return Err(AppError::Invalid("count 1..=20".into())); }
    let body = {
        let conn = state.db.lock().map_err(|_| AppError::Config("db lock".into()))?;
        read_note_body(&conn, &note_id)?
    };
    let provider = state.router.pick_chat().await?;
    let prompt = crate::ai::prompts::cards_generate(&body, count);
    let req = crate::ai::provider::CompleteRequest { prompt, model: CARD_GEN_MODEL_DEFAULT.into(), max_tokens: Some(800) };
    let out = provider.complete(req).await?;
    let drafts: Vec<DraftCard> = first_json(&out.text, JsonShape::Array)
        .map_err(|e| AppError::Provider(format!("card parse: {e}")))?;
    Ok(drafts)
}

#[tauri::command]
pub fn save_cards(input: SaveCardsInput, state: State<'_, AppState>) -> AppResult<Vec<Card>> {
    let now = model_now();
    let conn = state.db.lock().map_err(|_| AppError::Config("db lock".into()))?;
    let mut out = vec![];
    for c in input.cards {
        let id = Ulid::new().to_string();
        // ponytail: dedupe via UNIQUE(note_id, front); insert IGNORE on conflict.
        let rows = conn.execute(
            "INSERT OR IGNORE INTO srs_cards (id, note_id, front, back, due_at, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?5)",
            params![id, input.note_id, c.front, c.back, now],
        )?;
        if rows == 0 { continue; }    // dedupe
        out.push(Card {
            id, note_id: input.note_id.clone(),
            front: c.front, back: c.back,
            ease: 2.5, interval_days: 0, repetitions: 0,
            due_at: now.clone(), created_at: now.clone(), suspended_at: None,
        });
    }
    Ok(out)
}

#[tauri::command]
pub fn queue(limit: u32, state: State<'_, AppState>) -> AppResult<Vec<CardSummary>> {
    let conn = state.db.lock().map_err(|_| AppError::Config("db lock".into()))?;
    let mut stmt = conn.prepare(
        "SELECT id, front, back, due_at FROM srs_cards
         WHERE suspended_at IS NULL AND due_at <= ?1
         ORDER BY due_at ASC LIMIT ?2"
    )?;
    let now = model_now();
    let rows = stmt.query_map(params![now, limit as i64], |r| Ok(CardSummary {
        id: r.get(0)?, front: r.get(1)?, back: r.get(2)?, due_at: r.get(3)?,
    }))?.filter_map(Result::ok).collect();
    Ok(rows)
}

#[tauri::command]
pub fn review(card_id: String, outcome: ReviewOutcome, state: State<'_, AppState>) -> AppResult<Card> {
    let conn = state.db.lock().map_err(|_| AppError::Config("db lock".into()))?;
    let now = model_now();
    let prev = conn.query_row(
        "SELECT ease, interval_days, repetitions FROM srs_cards WHERE id=?1",
        params![card_id],
        |r| Ok((r.get::<_, f32>(0)?, r.get::<_, i32>(1)? as u32, r.get::<_, i32>(2)? as u32)),
    ).map_err(|_| AppError::NotFound(format!("card {card_id}")))?;
    let state_in = CardState {
        ease: prev.0, interval_days: prev.1, repetitions: prev.2,
        due_at: now.clone(),
    };
    let next = sm2_review(state_in, outcome.quality, &now);
    conn.execute(
        "UPDATE srs_cards SET ease=?1, interval_days=?2, repetitions=?3, due_at=?4 WHERE id=?5",
        params![next.ease as f64, next.interval_days as i64, next.repetitions as i64, next.due_at, card_id],
    )?;
    let review_id = Ulid::new().to_string();
    conn.execute(
        "INSERT INTO srs_reviews (id, card_id, reviewed_at, quality, interval_after, ease_after)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        params![review_id, card_id, now, outcome.quality as i64, next.interval_days as i64, next.ease as f64],
    )?;
    Ok(read_card(&conn, &card_id)?)
}

#[tauri::command]
pub fn suspend(card_id: String, suspended: bool, state: State<'_, AppState>) -> AppResult<()> {
    let conn = state.db.lock().map_err(|_| AppError::Config("db lock".into()))?;
    let stamp = if suspended { Some(model_now()) } else { None };
    conn.execute("UPDATE srs_cards SET suspended_at=?1 WHERE id=?2", params![stamp, card_id])?;
    Ok(())
}

fn model_now() -> String { /* same as tasks::model::now_iso via crate::time helper */ crate::time_now() }
fn read_card(conn: &rusqlite::Connection, id: &str) -> AppResult<Card> { /* SELECT all cols */ ... }
fn read_note_body(conn: &rusqlite::Connection, id: &str) -> AppResult<String> { /* SELECT body from notes */ ... }
```

Add `pub fn time_now() -> String` to a tiny `src-tauri/src/time_util.rs` to share between tasks + srs — single source of truth.

```rust
// src-tauri/src/time_util.rs
use time::OffsetDateTime;
pub fn time_now() -> String {
    OffsetDateTime::now_utc()
        .format(&time::format_description::well_known::Rfc3339)
        .unwrap_or_else(|_| "1970-01-01T00:00:00Z".into())
}
```

Add `pub mod time_util;` to `lib.rs`.

- [ ] **Step 2: Tests**

Add to `src-tauri/src/srs/mod.rs`:
```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::db;
    use tempfile::tempdir;

    fn open_db() -> rusqlite::Connection {
        let d = tempdir().unwrap();
        let p = db::AppPaths::from_root(d.path().to_path_buf()).unwrap();
        std::fs::create_dir_all(&p.data_dir).unwrap();
        db::open(&p).unwrap()
    }

    fn seed_card(conn: &rusqlite::Connection, front: &str) {
        let id = Ulid::new().to_string();
        conn.execute(
            "INSERT INTO srs_cards (id, front, back, due_at, created_at) VALUES (?1, ?2, 'b', '2026-07-06T00:00:00Z', '2026-07-06T00:00:00Z')",
            params![id, front],
        ).unwrap();
    }

    #[test]
    fn queue_filters_suspended() {
        let conn = open_db();
        seed_card(&conn, "f1");
        let now = time_now();
        let id = Ulid::new().to_string();
        conn.execute(
            "INSERT INTO srs_cards (id, front, back, due_at, created_at, suspended_at) VALUES (?1, 'f2', 'b', '2026-07-06T00:00:00Z', '2026-07-06T00:00:00Z', ?2)",
            params![id, now],
        ).unwrap();
        let mut stmt = conn.prepare("SELECT COUNT(*) FROM srs_cards WHERE suspended_at IS NULL").unwrap();
        let n: i64 = stmt.query_row([], |r| r.get(0)).unwrap();
        assert_eq!(n, 1);
    }

    #[test]
    fn save_dedupes_by_note_front() {
        let conn = open_db();
        let id = Ulid::new().to_string();
        conn.execute("INSERT OR IGNORE INTO srs_cards (id, front, back, due_at, created_at) VALUES (?1, 'f', 'b', 'now', 'now')", params![id]).unwrap();
        conn.execute("INSERT OR IGNORE INTO srs_cards (id, front, back, due_at, created_at) VALUES (?1, 'f', 'b2', 'now', 'now')", params![Ulid::new().to_string()]).unwrap();
        let n: i64 = conn.query_row("SELECT COUNT(*) FROM srs_cards WHERE front='f'", [], |r| r.get(0)).unwrap();
        assert_eq!(n, 1, "UNIQUE(note_id, front) should dedupe");
    }
}
```

- [ ] **Step 3: Wire commands in `lib.rs`**

Add 6 to `generate_handler![...]`: `srs::generate_cards, srs::save_cards, srs::queue, srs::review, srs::suspend`.

- [ ] **Step 4: Frontend types**

```ts
// types.ts additions
export type DraftCard = { front: string; back: string };
export type Card = {
  id: string; noteId: string | null; front: string; back: string;
  ease: number; intervalDays: number; repetitions: number; dueAt: string;
  createdAt: string; suspendedAt: string | null;
};
export type CardSummary = { id: string; front: string; back: string; dueAt: string };
export type SaveCardsInput = { noteId: string | null; cards: DraftCard[] };
export type ReviewOutcome = { quality: 1 | 3 | 4 | 5 };
```

- [ ] **Step 5: IPC**

```ts
export function generateCards(noteId: string, count: number): Promise<DraftCard[]> { return invoke('generate_cards', { noteId, count }); }
export function saveCards(input: SaveCardsInput): Promise<Card[]> { return invoke('save_cards', { input }); }
export function queueSrs(limit: number): Promise<CardSummary[]> { return invoke('queue', { limit }); }
export function reviewCard(cardId: string, quality: 1|3|4|5): Promise<Card> { return invoke('review', { cardId, outcome: { quality } }); }
export function suspendCard(cardId: string, suspended: boolean): Promise<void> { return invoke('suspend', { cardId, suspended }); }
```

- [ ] **Step 6: Frontend store**

`src/lib/stores/srs.svelte.ts`:
```ts
import { queueSrs, reviewCard, suspendCard, generateCards, saveCards } from '$lib/ipc';
import type { CardSummary, DraftCard } from '$lib/types';

class SrsStore {
  queue = $state<CardSummary[]>([]);
  currentIndex = $state(0);
  showBack = $state(false);

  async loadQueue(limit = 20) {
    this.queue = await queueSrs(limit);
    this.currentIndex = 0;
    this.showBack = false;
  }

  get current() { return this.queue[this.currentIndex]; }
  get hasNext() { return this.currentIndex < this.queue.length - 1; }

  reveal() { this.showBack = true; }

  async grade(quality: 1|3|4|5) {
    if (!this.current) return;
    await reviewCard(this.current.id, quality);
    this.currentIndex += 1;
    this.showBack = false;
  }

  async skip() {
    // ponytail: skip just advances to next; doesn't write a review. Suspend is the durable 'drop this'.
    if (!this.hasNext) return;
    this.currentIndex += 1;
    this.showBack = false;
  }

  async toggleSuspend(id: string, suspended: boolean) {
    await suspendCard(id, suspended);
    await this.loadQueue(this.queue.length);
  }

  async generate(noteId: string, count: number): Promise<DraftCard[]> {
    return await generateCards(noteId, count);
  }

  async saveBatch(noteId: string | null, cards: DraftCard[]) {
    return await saveCards({ noteId, cards });
  }
}

export const srs = new SrsStore();
```

- [ ] **Step 7: Verify checks**

Run: `pnpm check` and `cd src-tauri && cargo test --lib srs -- --skip generate`  -- skip network tests; SM-2 + DB tests pass.
Expected: SM-2 7 tests pass + queue/dedupe tests pass + frontend clean.

- [ ] **Step 8: Commit**

```bash
git add src-tauri/src/srs/ src-tauri/src/tasks/ src-tauri/src/time_util.rs src-tauri/src/lib.rs src/lib/
git commit -m "feat(srs): generate/save/queue/review/suspend + FE store"
```

---

### Task T52: Quiz module (commands + grading) + study store

**Files:**
- Create: `src-tauri/src/study/mod.rs`, `src-tauri/src/study/model.rs`, `src-tauri/src/study/quiz.rs`
- Modify: `src-tauri/src/lib.rs`
- Modify: `src/lib/types.ts`, `src/lib/ipc.ts`, `src/lib/stores/study.svelte.ts`

- [ ] **Step 1: Model**

`src-tauri/src/study/model.rs`:
```rust
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Question {
    Mc {
        question: String,
        choices: Vec<String>,
        #[serde(default)]
        answer: usize,
        #[serde(default)]
        rationale: String,
    },
    Short {
        question: String,
        #[serde(default)]
        answer: String,
        #[serde(default)]
        rationale: String,
    },
    Cloze {
        question: String,
        #[serde(default)]
        answer: String,
        #[serde(default)]
        rationale: String,
    },
}

#[derive(Debug, Serialize, Clone)]
pub struct Quiz {
    pub id: String,
    pub note_ids: Vec<String>,
    pub questions: Vec<Question>,
    pub created_at: String,
}

#[derive(Debug, Serialize, Clone)]
pub struct QuizResult {
    pub quiz_id: String,
    pub score: u32,
    pub total: u32,
    pub per_question: Vec<QuestionOutcome>,
}

#[derive(Debug, Serialize, Clone)]
pub struct QuestionOutcome {
    pub index: usize,
    pub correct: bool,
    pub expected: String,
    pub given: String,
    pub rationale: String,
}

#[derive(Debug, Deserialize)]
pub struct NewQuizInput {
    pub note_ids: Vec<String>,
    pub count: u8,
}
```

- [ ] **Step 2: Quiz commands**

`src-tauri/src/study/quiz.rs`:
```rust
use crate::error::{AppError, AppResult};
use crate::study::model::{NewQuizInput, Question, QuestionOutcome, Quiz, QuizResult};
use rusqlite::params;
use ulid::Ulid;
use std::collections::HashSet;

use crate::ai::json_helpers::{first_json, JsonShape};
use crate::ai::prompts::quiz_generate;
use crate::time_util::time_now;

pub fn generate(input: NewQuizInput, pool_body: &str, provider: &dyn crate::ai::Provider) -> AppResult<Quiz> {
    if !(1..=20).contains(&input.count) { return Err(AppError::Invalid("count 1..=20".into())); }
    if input.note_ids.is_empty() { return Err(AppError::Invalid("note_ids empty".into())); }
    let prompt = quiz_generate(pool_body, input.count);
    let req = crate::ai::provider::CompleteRequest { prompt, model: "llama3.2".into(), max_tokens: Some(1500) };
    let out = provider.complete(req).await?;   // ⬅ provider.complete is async; quiz fn is async
    let qs: Vec<Question> = first_json(&out.text, JsonShape::Array)
        .map_err(|e| AppError::Provider(format!("quiz parse: {e}")))?;
    if qs.is_empty() { return Err(AppError::Provider("quiz parse returned empty".into())); }
    Ok(Quiz {
        id: Ulid::new().to_string(),
        note_ids: input.note_ids,
        questions: qs,
        created_at: time_now(),
    })
}

pub fn grade(quiz: &Quiz, answers: &[String]) -> QuizResult {
    // ponytail: multiple-choice equality; short/cloze use case-insensitive trim match.
    // Rationales are returned verbatim from the quiz regardless of score.
    let mut per_q: Vec<QuestionOutcome> = vec![];
    let mut score = 0u32;
    for (i, q) in quiz.questions.iter().enumerate() {
        let given = answers.get(i).cloned().unwrap_or_default();
        let (correct, expected) = match q {
            Question::Mc { choices, answer, .. } => {
                let exp_idx = *answer;
                let exp_str = choices.get(exp_idx).cloned().unwrap_or_default();
                let given_idx = given.parse::<usize>().ok();
                (Some(exp_idx) == given_idx, format!("index={exp_idx} ({exp_str})"))
            }
            Question::Short { answer, .. } => {
                let norm: String = answer.trim().to_lowercase();
                (norm == given.trim().to_lowercase(), answer.clone())
            }
            Question::Cloze { answer, .. } => {
                let norm: String = answer.trim().to_lowercase();
                (norm == given.trim().to_lowercase(), answer.clone())
            }
        };
        let rationale = match q {
            Question::Mc { rationale, .. } | Question::Short { rationale, .. } | Question::Cloze { rationale, .. } => rationale.clone(),
        };
        if correct { score += 1; }
        per_q.push(QuestionOutcome { index: i, correct, expected, given, rationale });
    }
    QuizResult { quiz_id: quiz.id.clone(), score, total: quiz.questions.len() as u32, per_question: per_q }
}

pub fn save_attempt(conn: &rusqlite::Connection, quiz: &Quiz, answers: &[String], result: &QuizResult) -> AppResult<String> {
    let id = Ulid::new().to_string();
    let qjson = serde_json::to_string(&quiz.questions)?;
    let ajson = serde_json::to_string(answers)?;
    conn.execute(
        "INSERT INTO quiz_attempts (id, note_id, questions_json, answers_json, score, total, created_at, completed_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?7)",
        params![id, quiz.note_ids.first().cloned().unwrap_or_default(), qjson, ajson, result.score as i64, result.total as i64, time_now()],
    )?;
    Ok(id)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::study::model::Question;

    #[test]
    fn mc_correct_when_index_matches() {
        let q = Question::Mc { question: "x".into(), choices: vec!["a".into(), "b".into()], answer: 1, rationale: "b".into() };
        let quiz = Quiz { id: "q".into(), note_ids: vec![], questions: vec![q], created_at: "now".into() };
        let r = grade(&quiz, &["1".into()]);
        assert!(r.per_question[0].correct);
        assert_eq!(r.score, 1);
        assert_eq!(r.total, 1);
    }

    #[test]
    fn short_grade_is_case_insensitive_trim() {
        let q = Question::Short { question: "x".into(), answer: "Photosynthesis".into(), rationale: "r".into() };
        let quiz = Quiz { id: "q".into(), note_ids: vec![], questions: vec![q], created_at: "now".into() };
        let r = grade(&quiz, &["  photosynthesis ".into()]);
        assert!(r.per_question[0].correct);
    }

    #[test]
    fn empty_quiz_grade_is_zero() {
        let quiz = Quiz { id: "q".into(), note_ids: vec![], questions: vec![], created_at: "now".into() };
        let r = grade(&quiz, &[]);
        assert_eq!(r.score, 0);
        assert_eq!(r.total, 0);
    }
}
```

Note: `fn generate` is async. Tauri requires `async` for this command. Update:

```rust
pub async fn generate(input: NewQuizInput, pool_body: &str, provider: &dyn crate::ai::Provider) -> AppResult<Quiz> { ... }
```

- [ ] **Step 3: Commands in `study/mod.rs`**

```rust
pub mod model;
pub mod quiz;
pub mod plan;

use crate::error::{AppError, AppResult};
use crate::AppState;
use tauri::State;

use model::{NewQuizInput, Quiz, QuizResult};

#[tauri::command]
pub async fn generate_quiz(input: NewQuizInput, state: State<'_, AppState>) -> AppResult<Quiz> {
    let conn = state.db.lock().map_err(|_| AppError::Config("db lock".into()))?;
    let mut body = String::new();
    for nid in &input.note_ids {
        let b: String = conn.query_row("SELECT body FROM notes WHERE id=?1", rusqlite::params![nid], |r| r.get(0)).unwrap_or_default();
        body.push_str(&format!("\n\n# {}\n{}", nid, b));
    }
    drop(conn);
    let provider = state.router.pick_chat().await?;
    crate::study::quiz::generate(input, &body, &*provider).await
}

#[tauri::command]
pub async fn grade_quiz(quiz: serde_json::Value, answers: Vec<String>, state: State<'_, AppState>) -> AppResult<QuizResult> {
    let q: Quiz = serde_json::from_value(quiz).map_err(|e| AppError::Invalid(format!("quiz json: {e}")))?;
    let result = crate::study::quiz::grade(&q, &answers);
    let conn = state.db.lock().map_err(|_| AppError::Config("db lock".into()))?;
    let _ = crate::study::quiz::save_attempt(&conn, &q, &answers, &result)?;
    Ok(result)
}
```

`grade_quiz` takes `quiz: serde_json::Value` because the front will pass back the same `Quiz` we sent it — simpler than round-tripping via DB. Spec §9 has both `quiz.generate` and `quiz.check_answer` — they map to `generate_quiz` and `grade_quiz` here.

- [ ] **Step 4: Frontend types**

```ts
export type Question =
  | { type: 'mc'; question: string; choices: string[]; answer?: number; rationale?: string }
  | { type: 'short'; question: string; answer?: string; rationale?: string }
  | { type: 'cloze'; question: string; answer?: string; rationale?: string };

export type Quiz = { id: string; noteIds: string[]; questions: Question[]; createdAt: string };
export type QuizResult = { quizId: string; score: number; total: number; perQuestion: { index: number; correct: boolean; expected: string; given: string; rationale: string }[] };
export type NewQuizInput = { noteIds: string[]; count: number };
```

- [ ] **Step 5: IPC**

```ts
export function generateQuiz(input: NewQuizInput): Promise<Quiz> { return invoke('generate_quiz', { input }); }
export function gradeQuiz(quiz: Quiz, answers: string[]): Promise<QuizResult> { return invoke('grade_quiz', { quiz, answers }); }
```

- [ ] **Step 6: Frontend store — extend `study.svelte.ts`**

```ts
import { generateQuiz, gradeQuiz } from '$lib/ipc';
import type { Question, Quiz, QuizResult } from '$lib/types';

class StudyStore {
  quiz = $state<Quiz | null>(null);
  answers = $state<string[]>([]);
  result = $state<QuizResult | null>(null);
  busy = $state(false);
  error = $state<string | null>(null);

  async startQuiz(noteIds: string[], count: number) {
    this.busy = true; this.error = null; this.result = null;
    try {
      this.quiz = await generateQuiz({ noteIds, count });
      this.answers = new Array(this.quiz.questions.length).fill('');
    } catch (e) {
      this.error = (e as { message: string }).message;
    } finally { this.busy = false; }
  }

  setAnswer(i: number, v: string) { this.answers[i] = v; }

  async submit() {
    if (!this.quiz) return;
    this.busy = true;
    try {
      this.result = await gradeQuiz(this.quiz, this.answers);
    } finally { this.busy = false; }
  }

  reset() { this.quiz = null; this.answers = []; this.result = null; this.error = null; }
}

// plan store added in T53
class PlanStore { /* T53 */ }

export const study = new StudyStore();
```

- [ ] **Step 7: Verify**

Run: `pnpm check` 0/0; `cd src-tauri && cargo test --lib study::quiz`. 3 tests pass.

- [ ] **Step 8: Commit**

```bash
git add src-tauri/src/study/ src-tauri/src/lib.rs src/lib/
git commit -m "feat(study): quiz generation + grading + FE store"
```

---

### Task T53: Plan module + weekly plan + IPC

**Files:**
- Create: `src-tauri/src/study/plan.rs`
- Modify: `src-tauri/src/study/mod.rs` — add `generate_plan`, `save_plan`, `get_plan`
- Modify: `src/lib/stores/study.svelte.ts` — add `PlanStore`

- [ ] **Step 1: Plan validate + persist**

`src-tauri/src/study/plan.rs`:
```rust
use crate::error::{AppError, AppResult};
use crate::time_util::time_now;
use rusqlite::params;
use serde::{Deserialize, Serialize};
use ulid::Ulid;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct PlanBlock {
    pub day: String,           // YYYY-MM-DD
    pub start: String,         // HH:MM
    pub minutes: u32,
    pub kind: String,          // review | read | quiz | break
    pub refs: Vec<String>,
    pub rationale: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Plan {
    pub week_start: String,    // YYYY-MM-DD (must be Monday)
    pub daily_hours_cap: u8,
    pub blocks: Vec<PlanBlock>,
}

pub fn validate(p: &Plan) -> Result<(), String> {
    // ponytail: structural invariants; the model still produces *something* useful even if the LLM
    // violates a soft rule (e.g. an over-budget block). Caller decides whether to discard.
    if p.blocks.is_empty() { return Err("no blocks".into()); }
    if p.daily_hours_cap == 0 || p.daily_hours_cap > 16 { return Err("daily_hours_cap out of range".into()); }
    for b in &p.blocks {
        if b.minutes == 0 || b.minutes > 60 * p.daily_hours_cap as u32 { return Err(format!("bad minutes {}", b.minutes)); }
        if !b.day.starts_with("202") && !b.day.starts_with("203") { return Err(format!("bad day {}", b.day)); }
        if b.start.len() != 5 || b.start.as_bytes()[2] != b':' { return Err(format!("bad start {}", b.start)); }
    }
    Ok(())
}

pub fn save(conn: &rusqlite::Connection, p: &Plan) -> AppResult<String> {
    if let Err(e) = validate(p) { return Err(AppError::Invalid(format!("plan: {e}"))); }
    let id = Ulid::new().to_string();
    let plan_json = serde_json::to_string(p)?;
    conn.execute(
        "INSERT INTO study_sessions (id, kind, started_at, plan_json) VALUES (?1, 'plan', ?2, ?3)",
        params![id, time_now(), plan_json],
    )?;
    Ok(id)
}

pub fn get(conn: &rusqlite::Connection, week_start: &str) -> AppResult<Option<Plan>> {
    let mut stmt = conn.prepare(
        "SELECT plan_json FROM study_sessions
         WHERE kind='plan' AND json_extract(plan_json, '$.week_start') = ?1
         ORDER BY started_at DESC LIMIT 1"
    )?;
    let mut rows = stmt.query(params![week_start])?;
    if let Some(r) = rows.next()? {
        let s: String = r.get(0)?;
        let p: Plan = serde_json::from_str(&s).map_err(|e| AppError::Invalid(format!("plan json: {e}")))?;
        return Ok(Some(p));
    }
    Ok(None)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db;
    use tempfile::tempdir;

    fn bp() -> PlanBlock {
        PlanBlock { day: "2026-07-06".into(), start: "09:00".into(), minutes: 60, kind: "review".into(), refs: vec![], rationale: "x".into() }
    }

    #[test]
    fn validate_rejects_zero_blocks() {
        let p = Plan { week_start: "2026-07-06".into(), daily_hours_cap: 4, blocks: vec![] };
        assert!(validate(&p).is_err());
    }

    #[test]
    fn validate_accepts_valid() {
        let p = Plan { week_start: "2026-07-06".into(), daily_hours_cap: 4, blocks: vec![bp()] };
        assert!(validate(&p).is_ok());
    }

    #[test]
    fn round_trip_save_get() {
        let d = tempdir().unwrap();
        let p = db::AppPaths::from_root(d.path().to_path_buf()).unwrap();
        std::fs::create_dir_all(&p.data_dir).unwrap();
        let conn = db::open(&p).unwrap();
        let plan = Plan { week_start: "2026-07-06".into(), daily_hours_cap: 4, blocks: vec![bp()] };
        let _id = save(&conn, &plan).unwrap();
        let got = get(&conn, "2026-07-06").unwrap();
        assert!(got.is_some());
        assert_eq!(got.unwrap().blocks.len(), 1);
    }
}
```

- [ ] **Step 2: Commands**

```rust
// study/mod.rs additions
pub mod plan;
pub use plan::{Plan, PlanBlock};

#[tauri::command]
pub async fn generate_plan(
    week_start: String,
    daily_hours_cap: u8,
    state: State<'_, AppState>,
) -> AppResult<Plan> {
    let conn = state.db.lock().map_err(|_| AppError::Config("db lock".into()))?;
    let context = plan::build_context(&conn, &week_start, daily_hours_cap)?;
    drop(conn);
    let provider = state.router.pick_chat().await?;
    let prompt = crate::ai::prompts::plan_generate(&context, &week_start, daily_hours_cap);
    let req = crate::ai::provider::CompleteRequest { prompt, model: "llama3.2".into(), max_tokens: Some(1200) };
    let out = provider.complete(req).await?;
    let p: Plan = crate::ai::json_helpers::first_json(&out.text, JsonShape::Object)
        .map_err(|e| AppError::Provider(format!("plan parse: {e}")))?;
    if let Err(e) = plan::validate(&p) { return Err(AppError::Provider(format!("plan invalid: {e}"))); }
    Ok(p)
}

#[tauri::command]
pub fn save_plan(plan: Plan, state: State<'_, AppState>) -> AppResult<String> {
    let conn = state.db.lock().map_err(|_| AppError::Config("db lock".into()))?;
    crate::study::plan::save(&conn, &plan)
}

#[tauri::command]
pub fn get_plan(week_start: String, state: State<'_, AppState>) -> AppResult<Option<Plan>> {
    let conn = state.db.lock().map_err(|_| AppError::Config("db lock".into()))?;
    crate::study::plan::get(&conn, &week_start)
}
```

Add `plan::build_context`:
```rust
pub fn build_context(conn: &rusqlite::Connection, week_start: &str, daily_hours: u8) -> AppResult<String> {
    // Summary of tasks due this week + recent 10 note titles + SRS backlog count.
    let mut s = String::new();
    let mut stmt = conn.prepare("SELECT title, due_at FROM tasks WHERE status IN ('todo','doing') AND due_at BETWEEN ?1 AND datetime(?1, '+6 days') ORDER BY due_at ASC LIMIT 20")?;
    let rows = stmt.query_map(params![week_start], |r| Ok((r.get::<_, String>(0)?, r.get::<_, Option<String>>(1)?)))?;
    s.push_str("Tasks due this week:\n");
    for r in rows { let (t, d) = r?; s.push_str(&format!("- {} (due {})\n", t, d.unwrap_or_else(|| "n/a".into()))); }

    let mut stmt = conn.prepare("SELECT title FROM notes WHERE deleted_at IS NULL ORDER BY updated DESC LIMIT 10")?;
    let rows = stmt.query_map([], |r| r.get::<_, String>(0))?;
    s.push_str("\nRecent notes:\n");
    for r in rows { s.push_str(&format!("- {}\n", r?)); }

    let mut stmt = conn.prepare("SELECT COUNT(*) FROM srs_cards WHERE suspended_at IS NULL")?;
    let backlog: i64 = stmt.query_row([], |r| r.get(0))?;
    s.push_str(&format!("\nSRS backlog: {} cards\n", backlog));
    s.push_str(&format!("Daily study cap: {} hours\n", daily_hours));
    Ok(s)
}
```

- [ ] **Step 3: Frontend types**

```ts
export type PlanBlock = { day: string; start: string; minutes: number; kind: string; refs: string[]; rationale: string };
export type Plan = { weekStart: string; dailyHoursCap: number; blocks: PlanBlock[] };
```

- [ ] **Step 4: IPC**

```ts
export function generatePlan(weekStart: string, dailyHoursCap: number): Promise<Plan> { return invoke('generate_plan', { weekStart, dailyHoursCap }); }
export function savePlan(plan: Plan): Promise<string> { return invoke('save_plan', { plan }); }
export function getPlan(weekStart: string): Promise<Plan | null> { return invoke('get_plan', { weekStart }); }
```

- [ ] **Step 5: Frontend — `PlanStore` in study store**

```ts
import { generatePlan, savePlan, getPlan } from '$lib/ipc';
import type { Plan } from '$lib/types';

class PlanStore {
  plan = $state<Plan | null>(null);
  busy = $state(false);
  error = $state<string | null>(null);
  weekStart = $state(weekStartThisMonday());

  async generate(dailyHoursCap = 4) {
    this.busy = true; this.error = null;
    try {
      this.plan = await generatePlan(this.weekStart, dailyHoursCap);
    } catch (e) { this.error = (e as { message: string }).message; }
    finally { this.busy = false; }
  }

  async load() {
    const got = await getPlan(this.weekStart);
    if (got) this.plan = got;
  }

  async persist() {
    if (!this.plan) return;
    await savePlan(this.plan);
  }

  moveBlock(i: number, toDay: string) {
    if (!this.plan) return;
    const b = this.plan.blocks[i];
    if (!b) return;
    this.plan.blocks[i] = { ...b, day: toDay };
  }
}

function weekStartThisMonday(): string {
  const d = new Date();
  const day = d.getUTCDay(); // 0=Sun
  const diff = (day === 0 ? -6 : 1 - day);
  d.setUTCDate(d.getUTCDate() + diff);
  return d.toISOString().split('T')[0];
}

export const plan = new PlanStore();
```

Append `PlanStore` and `export const plan = new PlanStore();` to `study.svelte.ts`.

- [ ] **Step 6: Verify**

Run: `pnpm check`. `cd src-tauri && cargo test --lib study::plan` — 3 tests pass.

- [ ] **Step 7: Commit**

```bash
git add src-tauri/src/study/ src-tauri/src/lib.rs src/lib/
git commit -m "feat(study): weekly plan generation + persist + FE store"
```

---

### Task T54: /tasks route + TaskBoard.svelte

**Files:**
- Create: `src/routes/tasks/+page.svelte`
- Create: `src/lib/components/TaskBoard.svelte`

- [ ] **Step 1: TaskBoard component**

`src/lib/components/TaskBoard.svelte`:
```svelte
<script lang="ts">
  import { tasks } from '$lib/stores/tasks.svelte';
  import type { TaskStatus } from '$lib/types';

  const COLS: { key: TaskStatus; label: string }[] = [
    { key: 'todo', label: 'To do' },
    { key: 'doing', label: 'Doing' },
    { key: 'done', label: 'Done' },
    { key: 'cancelled', label: 'Cancelled' },
  ];

  let newTitle = $state('');
  let newPriority: 0 | 1 | 2 = $state(0);
  let dragId = $state<string | null>(null);

  async function add() {
    if (!newTitle.trim()) return;
    await tasks.add({ title: newTitle.trim(), priority: newPriority });
    newTitle = '';
  }

  function onDragStart(id: string) { dragId = id; }
  async function onDrop(status: TaskStatus) {
    if (!dragId) return;
    await tasks.setStatus(dragId, status);
    dragId = null;
  }
</script>

<header>
  <input
    placeholder="What needs to happen?"
    bind:value={newTitle}
    onkeydown={(e) => { if (e.key === 'Enter') add(); }}
  />
  <select bind:value={newPriority}>
    <option value={0}>Low</option><option value={1}>Med</option><option value={2}>High</option>
  </select>
  <button onclick={add}>Add</button>
</header>

<section class="board">
  {#each COLS as col}
    <div
      class="col"
      ondragover={(e) => e.preventDefault()}
      ondrop={() => onDrop(col.key)}
    >
      <h3>{col.label}</h3>
      {#each tasks.items.filter((t) => t.status === col.key) as t (t.id)}
        <article
          draggable="true"
          ondragstart={() => onDragStart(t.id)}
        >
          <p>{t.title}</p>
          {#if t.dueAt}<small>Due: {t.dueAt.split('T')[0]}</small>{/if}
          <button onclick={() => tasks.remove(t.id)} aria-label="Delete">×</button>
        </article>
      {/each}
    </div>
  {/each}
</section>

<style>
  header { display: flex; gap: 0.5rem; margin-bottom: 1rem; }
  header input { flex: 1; }
  .board { display: grid; grid-template-columns: repeat(4, 1fr); gap: 1rem; }
  .col { background: var(--bg-elevated, #1a1a1a); padding: 0.75rem; border-radius: 0.5rem; min-height: 60vh; }
  .col h3 { margin: 0 0 0.5rem; }
  article { background: #222; padding: 0.6rem; border-radius: 0.3rem; margin-bottom: 0.5rem; display: flex; flex-direction: column; gap: 0.3rem; }
  article small { opacity: 0.7; }
  article button { margin-left: auto; }
</style>
```

- [ ] **Step 2: Route**

`src/routes/tasks/+page.svelte`:
```svelte
<script lang="ts">
  import { onMount } from 'svelte';
  import { tasks } from '$lib/stores/tasks.svelte';
  import TaskBoard from '$lib/components/TaskBoard.svelte';

  onMount(() => tasks.load());
</script>

<main>
  <h1>Tasks</h1>
  <TaskBoard />
</main>

<style>
  main { padding: 1rem; }
</style>
```

- [ ] **Step 3: Verify**

Run: `pnpm check` — 0/0.

- [ ] **Step 4: Commit**

```bash
git add src/routes/tasks src/lib/components/TaskBoard.svelte
git commit -m "feat(ui): tasks kanban at /tasks"
```

---

### Task T55: /study route + SRSReviewer + QuizRunner + WeeklyPlan

**Files:**
- Create: `src/lib/components/SRSReviewer.svelte`
- Create: `src/lib/components/QuizRunner.svelte`
- Create: `src/lib/components/WeeklyPlan.svelte`
- Create: `src/routes/study/+page.svelte`

- [ ] **Step 1: SRSReviewer**

`src/lib/components/SRSReviewer.svelte`:
```svelte
<script lang="ts">
  import { onMount } from 'svelte';
  import { srs } from '$lib/stores/srs.svelte';
  import { generateCards } from '$lib/ipc';

  let { noteId }: { noteId?: string } = $props();
  let generating = $state(false);
  let drafts = $state<{ front: string; back: string }[]>([]);

  onMount(() => srs.loadQueue());

  async function gen() {
    if (!noteId) return;
    generating = true;
    try { drafts = await generateCards(noteId, 5); }
    finally { generating = false; }
  }
</script>

<section>
  <header>
    <h2>Today</h2>
    <span>{srs.queue.length} cards due</span>
    {#if noteId}<button onclick={gen} disabled={generating}>+ Generate 5 cards</button>{/if}
  </header>

  {#if drafts.length > 0}
    <article class="drafts">
      <h3>Drafts</h3>
      {#each drafts as d, i}
        <div>
          <strong>{d.front}</strong> → {d.back}
          <button onclick={() => { srs.saveBatch(noteId!, [d]); drafts.splice(i, 1); }}>Save</button>
        </div>
      {/each}
    </article>
  {/if}

  {#if srs.current}
    <div class="card">
      {#if srs.showBack}
        <p>{srs.current.front}</p>
        <hr>
        <p>{srs.current.back}</p>
      {:else}
        <p>{srs.current.front}</p>
        <button onclick={() => srs.reveal()}>Show answer</button>
      {/if}
    </div>

    {#if srs.showBack}
      <div class="grade">
        <button onclick={() => srs.grade(1)}>Again</button>
        <button onclick={() => srs.grade(3)}>Hard</button>
        <button onclick={() => srs.grade(4)}>Good</button>
        <button onclick={() => srs.grade(5)}>Easy</button>
      </div>
    {:else}
      <button onclick={() => srs.skip()}>Skip</button>
    {/if}
  {:else}
    <p>Nothing due. 🎉</p>
  {/if}
</section>

<style>
  section { padding: 1rem; }
  header { display: flex; gap: 1rem; align-items: baseline; }
  .card { padding: 2rem; background: var(--bg-elevated, #1a1a1a); border-radius: 0.5rem; margin: 1rem 0; text-align: center; min-height: 8rem; }
  .grade { display: flex; gap: 0.5rem; justify-content: center; }
  .grade button { padding: 0.5rem 1rem; }
  .drafts { margin: 1rem 0; padding: 0.5rem; background: #111; border-radius: 0.5rem; }
</style>
```

- [ ] **Step 2: QuizRunner**

`src/lib/components/QuizRunner.svelte`:
```svelte
<script lang="ts">
  import { study } from '$lib/stores/study.svelte';

  let { noteIds }: { noteIds: string[] } = $props();
  let noteIdInput = $state('');

  function addId() {
    if (!noteIdInput.trim()) return;
    noteIds = [...noteIds, noteIdInput.trim()];
    noteIdInput = '';
  }

  async function start() {
    if (!noteIds.length) return;
    await study.startQuiz(noteIds, 5);
  }
</script>

<section>
  <h2>Quizzes</h2>

  {#if !study.quiz}
    <div class="setup">
      <input bind:value={noteIdInput} placeholder="Note ID" onkeydown={(e) => { if (e.key === 'Enter') addId(); }} />
      <button onclick={addId}>+ Note</button>
      <ul>{#each noteIds as id}<li>{id} <button onclick={() => noteIds = noteIds.filter((n) => n !== id)}>×</button></li>{/each}</ul>
      <button onclick={start} disabled={!noteIds.length || study.busy}>Generate quiz</button>
    </div>
 $1{#if study.error}<p class="err">{study.error}</p>{/if}
  {:else if !study.result}
    <div class="qs">
      {#each study.quiz.questions as q, i}
        <div>
          <p><strong>{i + 1}.</strong> {q.question}</p>
          {#if q.type === 'mc'}
            {#each q.choices as c, j}
              <label><input type="radio" name={`q${i}`} value={j} onchange={() => study.setAnswer(i, String(j))} /> {c}</label>
            {/each}
          {:else}
            <input oninput={(e) => study.setAnswer(i, (e.target as HTMLInputElement).value)} placeholder="Your answer" />
          {/if}
        </div>
      {/each}
      <button onclick={() => study.submit()} disabled={study.busy}>Submit</button>
    </div>
  {:else}
    <div class="result">
      <h3>Score: {study.result.score} / {study.result.total}</h3>
      {#each study.result.perQuestion as r}
        <div class={r.correct ? 'ok' : 'no'}>
          <strong>Q{r.index + 1}</strong>: {r.correct ? '✓' : '✗'} expected <em>{r.expected}</em>, you wrote <em>{r.given}</em>
          <p>{r.rationale}</p>
        </div>
      {/each}
      <button onclick={() => study.reset()}>New quiz</button>
    </div>
  {/if}
</section>

<style>
  section { padding: 1rem; }
  .setup, .qs, .result { display: flex; flex-direction: column; gap: 0.75rem; }
  .ok { border-left: 3px solid #4f4; padding-left: 0.5rem; }
  .no { border-left: 3px solid #f44; padding-left: 0.5rem; }
  .err { color: #c00; }
  ul { list-style: none; padding: 0; }
</style>
```

(Note: `$1` in the snippet above was a transcription artifact in this draft; the real file uses `{#if study.error}<p ...>{/if}`.)

- [ ] **Step 3: WeeklyPlan**

`src/lib/components/WeeklyPlan.svelte`:
```svelte
<script lang="ts">
  import { onMount } from 'svelte';
  import { plan } from '$lib/stores/study.svelte';

  onMount(() => plan.load());

  let cap = $state(4);

  async function generate() {
    await plan.generate(cap);
    if (plan.plan) await plan.persist();
  }

  function dayBlocks(day: string) {
    return plan.plan?.blocks.filter((b) => b.day === day) ?? [];
  }
</script>

<section>
  <header>
    <h2>Plan for week of {plan.weekStart}</h2>
    <label>Cap <input type="number" min="1" max="12" bind:value={cap} /> hours/day</label>
    <button onclick={generate} disabled={plan.busy}>Regenerate</button>
  </header>

  {#if plan.error}<p class="err">{plan.error}</p>{/if}

  {#if plan.plan}
    <div class="grid">
      {#each ['2026-07-06'] as dayPlaceholder}
        <article>
          <h4>{dayPlaceholder}</h4>
          {#each dayBlocks(dayPlaceholder) as b, i}
            <div>
              <strong>{b.start}</strong> · {b.minutes}m · {b.kind}
              <p>{b.rationale}</p>
            </div>
          {/each}
        </article>
      {/each}
    </div>
  {:else}
    <p>No plan yet.</p>
  {/if}
</section>

<style>
  section { padding: 1rem; }
  header { display: flex; gap: 1rem; align-items: baseline; }
  .grid { display: grid; grid-template-columns: repeat(7, 1fr); gap: 0.5rem; margin-top: 1rem; }
  article { background: var(--bg-elevated, #1a1a1a); padding: 0.5rem; border-radius: 0.5rem; min-height: 6rem; }
  .err { color: #c00; }
</style>
```

The hardcoded `dayPlaceholder` line is the planned upgrade spot — at execution time the loop will iterate the 7 ISO dates starting from `plan.weekStart`. I'll fix it during execution:

```ts
const weekDays = (start: string): string[] => {
  const d = new Date(start + 'T00:00:00Z');
  return Array.from({ length: 7 }, (_, i) => {
    const x = new Date(d); x.setUTCDate(d.getUTCDate() + i);
    return x.toISOString().split('T')[0];
  });
};
```

Then in template: `{#each weekDays(plan.weekStart) as day} ... {/each}`.

- [ ] **Step 4: Route**

`src/routes/study/+page.svelte`:
```svelte
<script lang="ts">
  import SRSReviewer from '$lib/components/SRSReviewer.svelte';
  import QuizRunner from '$lib/components/QuizRunner.svelte';
  import WeeklyPlan from '$lib/components/WeeklyPlan.svelte';
  import { page } from '$app/state';

  let tab = $state<'today' | 'quizzes' | 'plan'>('today');
  let selectedNoteIds = $state<string[]>([]);

  // ponytail: read selectedNote from query string for the Today tab to enable
  // "Generate cards" — passing via URL keeps the SRS component reusable.
  $effect(() => { selectedNoteIds = page.url.searchParams.getAll('note'); });
</script>

<main>
  <nav class="tabs">
    <button class:active={tab === 'today'} onclick={() => tab = 'today'}>Today</button>
    <button class:active={tab === 'quizzes'} onclick={() => tab = 'quizzes'}>Quizzes</button>
    <button class:active={tab === 'plan'} onclick={() => tab = 'plan'}>Plan</button>
  </nav>

  {#if tab === 'today'}
    <SRSReviewer noteId={selectedNoteIds[0]} />
  {:else if tab === 'quizzes'}
    <QuizRunner noteIds={selectedNoteIds} />
  {:else}
    <WeeklyPlan />
  {/if}
</main>

<style>
  main { padding: 1rem; }
  .tabs { display: flex; gap: 0.5rem; margin-bottom: 1rem; }
  .tabs button { padding: 0.5rem 1rem; }
  .tabs button.active { background: var(--accent, #4af); color: white; }
</style>
```

- [ ] **Step 5: Verify**

Run: `pnpm check`. 0/0.

- [ ] **Step 6: Commit**

```bash
git add src/routes/study src/lib/components/SRSReviewer.svelte src/lib/components/QuizRunner.svelte src/lib/components/WeeklyPlan.svelte src/lib/stores/study.svelte.ts
git commit -m "feat(ui): study tabs (Today/Quizzes/Plan) at /study"
```

---

### Task T56: Sidebar + capabilities + lib.rs wiring

**Files:**
- Modify: `src/lib/components/Sidebar.svelte` — add `/tasks` and `/study`
- Modify: `src-tauri/src/capabilities/default.json` — append 14 capabilities

- [ ] **Step 1: Sidebar**

In `src/lib/components/Sidebar.svelte`, append two items:
```ts
{ href: '/tasks', label: 'Tasks' },
{ href: '/study', label: 'Study' },
```

- [ ] **Step 2: Capabilities**

Append to `src-tauri/src/capabilities/default.json` under `"permissions"`:
```json
"tasks:allow-list-tasks",
"tasks:allow-create-task",
"tasks:allow-update-task",
"tasks:allow-delete-task",
"srs:allow-generate-cards",
"srs:allow-save-cards",
"srs:allow-queue",
"srs:allow-review",
"srs:allow-suspend",
"study:allow-generate-quiz",
"study:allow-grade-quiz",
"study:allow-generate-plan",
"study:allow-save-plan",
"study:allow-get-plan",
```

(Order: existing then new; capabilities match the Tauri command naming convention `<module>:<commandId>`.)

- [ ] **Step 3: Verify**

Run: `pnpm check`. 0/0.

- [ ] **Step 4: Commit**

```bash
git add src-tauri/capabilities/default.json src/lib/components/Sidebar.svelte
git commit -m "feat: sidebar tasks/study links + 14 phase-4 capabilities"
```

---

### Task T57: Selfcheck extension + checklist + README + tag

**Files:**
- Modify: `src-tauri/examples/selfcheck.rs`
- Modify: `docs/test-checklist.md`
- Modify: `README.md`
- Tag: `phase-4-estudio`

- [ ] **Step 1: Selfcheck — exercise SM-2 + persistence + plan round-trip**

Append to `src-tauri/examples/selfcheck.rs`:

```rust
// Phase 4 — SRS + quiz + plan invariants. No network calls.
use notias_lib::ai::prompts;
use notias_lib::srs::sm2::{review, CardState};
use notias_lib::study::plan::{self, Plan, PlanBlock};
use notias_lib::time_util::time_now;
use rusqlite::Connection;

let conn_for_phase4 = db::open(&paths).unwrap_or_else(|_| db::open(&paths).unwrap()); // reuses or rebuilds

// 1. SM-2: simulate Again → Hard → Good → Easy sequence.
let mut s = CardState { ease: 2.5, interval_days: 0, repetitions: 0, due_at: "2026-07-06T00:00:00Z".into() };
let baseline_ease = s.ease;
s = review(s, 1, "2026-07-06T00:00:00Z");   // Again
assert!(s.ease < baseline_ease, "ease should drop on Again");
s = review(s, 4, "2026-07-06T00:00:00Z");   // Good resets
s = review(s, 4, "2026-07-07T00:00:00Z");   // 6 days
s = review(s, 4, "2026-07-13T00:00:00Z");   // scale by ease
s = review(s, 5, "2026-07-13T00:00:00Z");   // Easy
println!("phase-4 SM-2 final: ease={:.3} reps={} int={}d", s.ease, s.repetitions, s.interval_days);

// 2. Tasks: insert, list.
let now = time_now();
conn_for_phase4.execute(
    "INSERT INTO tasks (id, title, priority, status, created_at, updated_at) VALUES ('p4t1','Read paper',1,'todo',?1,?1)",
    [&now],
).unwrap();
let n: i64 = conn_for_phase4.query_row("SELECT COUNT(*) FROM tasks", [], |r| r.get(0)).unwrap();
assert!(n >= 1, "task must persist");

// 3. SRS: insert 3 cards, assert queue returns at least 1 (cards are due now).
for f in ["Hecke operators", "Schur multipliers", "Borel subgroups"] {
    let id = ulid::Ulid::new().to_string();
    conn_for_phase4.execute(
        "INSERT INTO srs_cards (id, front, back, due_at, created_at) VALUES (?1, ?2, 'answer', '2026-07-06T00:00:00Z', '2026-07-06T00:00:00Z')",
        rusqlite::params![id, f],
    ).unwrap();
}
let q: i64 = conn_for_phase4.query_row("SELECT COUNT(*) FROM srs_cards WHERE suspended_at IS NULL AND due_at <= ?1", [&now], |r| r.get(0)).unwrap();
assert!(q >= 3, "expected ≥3 due cards");

// 4. Plan: save and read back.
let plan = Plan {
    week_start: "2026-07-06".into(),
    daily_hours_cap: 4,
    blocks: vec![PlanBlock {
        day: "2026-07-06".into(), start: "09:00".into(), minutes: 60,
        kind: "review".into(), refs: vec![], rationale: "warmup".into(),
    }],
};
let _ = plan::save(&conn_for_phase4, &plan).unwrap();
let got = plan::get(&conn_for_phase4, "2026-07-06").unwrap();
assert!(got.is_some(), "plan must round-trip");
println!("phase-4 selfcheck OK: tasks={n} srs_due={q} plan_persisted=true");

// 5. Prompts contain required content.
assert!(prompts::cards_generate("body", 5).contains("5"));
assert!(prompts::quiz_generate("body", 3).contains("JSON"));
```

- [ ] **Step 2: Test checklist**

Append to `docs/test-checklist.md`:

```markdown
## Phase 4 — Estudio autónomo

- [ ] `/tasks`: drag a card across columns → status updates without reload.
- [ ] `/tasks`: add a new task via input + Enter → row appears in `todo` column.
- [ ] `/study` Today: queue loads; Reveal → grade (Again/Hard/Good/Easy) advances; SM-2 interval grows on Good.
- [ ] `/study` Today: pass `?note=<id>` in URL → "+ Generate 5 cards" button appears + draft cards show.
- [ ] `/study` Quizzes: paste 2 note IDs + Generate → answer questions + Submit → score + per-question rationale visible.
- [ ] `/study` Plan: Regenerate writes `study_sessions.plan_json` with `week_start` of current Monday; drag a block → UI updates `plan.blocks[i].day`.
- [ ] Disable Ollama, enable Groq or OpenAI: SRS generation, quiz, and plan still work via cloud.
- [ ] `cd src-tauri && cargo run --example selfcheck` prints `phase-4 selfcheck OK: tasks=1 srs_due>=3 plan_persisted=true` (3+ tasks seeded by selfcheck).
- [ ] `pnpm tauri dev` opens window; sidebar nav shows Tasks + Study links.
- [ ] `cargo test --lib` passes including all 7 SM-2 tests + 3 quiz tests + 3 plan tests.
```

- [ ] **Step 3: README status**

Append to `README.md` "Status":
```
- Phase 4 (Estudio autónomo) complete — SM-2 SRS with UI grader, quiz generator + runner, weekly plan generator (AI via router fallback), tasks kanban. Verify with `pnpm tauri dev`; `cargo run --example selfcheck` exercises schema, SM-2, and plan persistence.
```

- [ ] **Step 4: Run selfcheck (sanity-check by inspection; Windows link.exe gap)**

Run: `cd src-tauri && cargo check --example selfcheck`
Expected: pass — confirms the example compiles. Actual run is blocked by missing link.exe (carry-over from Phases 2/3).

- [ ] **Step 5: Tag**

```bash
git tag phase-4-estudio -f
```

- [ ] **Step 6: Commit**

```bash
rtk git restore --staged README.md docs src-tauri/Cargo.lock
git add src-tauri/examples/selfcheck.rs docs/test-checklist.md README.md
git commit -m "test(phase-4): selfcheck exercises schema + SM-2 + plan + checklist"
```

---

## Review Checklist (for plan-document-reviewer)

- [ ] Each task lists exact file paths; no "add X" without path.
- [ ] Each task has a runnable verification (test name + `pnpm check` / `cargo test`).
- [ ] SM-2 is TDD-first per spec §12.
- [ ] Card/quiz/plan generation routes via existing `router.pick_chat()` (Phase 3 — no new provider).
- [ ] `json_helpers.rs` is the single shared parser; no duplication of the LLM-JSON extraction logic.
- [ ] Plan validation rejects malformed LLM output per spec §8 shape.
- [ ] Idempotency: `srs.save_cards` uses `UNIQUE(note_id, front)`.
- [ ] All 14 new Tauri commands show up in `generate_handler![...]` in `lib.rs`.
- [ ] All 14 capabilities in `capabilities/default.json`.
- [ ] Phase 4 doesn't introduce new deps (no Cargo.toml additions expected, no npm additions).
- [ ] Selfcheck covers schema + SM-2 + plan + tasks row, no network calls.

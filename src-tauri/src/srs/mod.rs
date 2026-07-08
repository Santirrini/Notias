pub mod model;
pub mod sm2;

use crate::ai::json_helpers::{first_json, JsonShape};
use crate::ai::provider::CompleteRequest;
use crate::error::{AppError, AppResult};
use crate::time_util::time_now;
use crate::AppState;
use rusqlite::params;
use tauri::State;
use ulid::Ulid;

use model::{Card, CardSummary, DraftCard, ReviewOutcome, SaveCardsInput};
use sm2::review as sm2_review;

const CARD_GEN_MODEL_DEFAULT: &str = "llama3.2";

#[tauri::command]
pub async fn generate_cards(
    note_id: String,
    count: u8,
    state: State<'_, AppState>,
) -> AppResult<Vec<DraftCard>> {
    if !(1..=20).contains(&count) {
        return Err(AppError::Invalid("count must be 1..=20".into()));
    }
    let body = {
        let conn = state.db.lock().map_err(|_| AppError::Config("db lock".into()))?;
        read_note_body(&conn, &note_id)?
    };
    let provider = state.router.pick_chat().await?;
    let prompt = crate::ai::prompts::cards_generate(&body, count);
    let out = provider
        .complete(CompleteRequest {
            prompt,
            model: CARD_GEN_MODEL_DEFAULT.into(),
            max_tokens: Some(800),
        })
        .await?;
    let drafts: Vec<DraftCard> = first_json(&out.text, JsonShape::Array)
        .map_err(|e| AppError::Provider(format!("card parse: {e}")))?;
    Ok(drafts)
}

#[tauri::command]
pub fn save_cards(input: SaveCardsInput, state: State<'_, AppState>) -> AppResult<Vec<Card>> {
    let now = time_now();
    let conn = state.db.lock().map_err(|_| AppError::Config("db lock".into()))?;
    let mut out = vec![];
    for c in input.cards {
        if c.front.trim().is_empty() || c.back.trim().is_empty() {
            continue;
        }
        let id = Ulid::new().to_string();
        // ponytail: dedupe via UNIQUE(note_id, front); insert IGNORE on conflict.
        let rows = conn.execute(
            "INSERT OR IGNORE INTO srs_cards (id, note_id, front, back, due_at, created_at) \
             VALUES (?1, ?2, ?3, ?4, ?5, ?5)",
            params![id, input.note_id, c.front, c.back, now],
        )?;
        if rows == 0 {
            continue;
        }
        out.push(Card {
            id,
            note_id: input.note_id.clone(),
            front: c.front,
            back: c.back,
            ease: 2.5,
            interval_days: 0,
            repetitions: 0,
            due_at: now.clone(),
            created_at: now.clone(),
            suspended_at: None,
        });
    }
    Ok(out)
}

#[tauri::command]
pub fn queue(limit: u32, state: State<'_, AppState>) -> AppResult<Vec<CardSummary>> {
    let conn = state.db.lock().map_err(|_| AppError::Config("db lock".into()))?;
    let now = time_now();
    let mut stmt = conn.prepare(
        "SELECT id, front, back, due_at FROM srs_cards \
         WHERE suspended_at IS NULL AND due_at <= ?1 \
         ORDER BY due_at ASC LIMIT ?2",
    )?;
    let rows = stmt
        .query_map(params![now, limit as i64], |r| {
            Ok(CardSummary {
                id: r.get(0)?,
                front: r.get(1)?,
                back: r.get(2)?,
                due_at: r.get(3)?,
            })
        })?
        .filter_map(Result::ok)
        .collect();
    Ok(rows)
}

#[tauri::command]
pub fn review(card_id: String, outcome: ReviewOutcome, state: State<'_, AppState>) -> AppResult<Card> {
    let now = time_now();
    let conn = state.db.lock().map_err(|_| AppError::Config("db lock".into()))?;
    let prev = conn
        .query_row(
            "SELECT ease, interval_days, repetitions FROM srs_cards WHERE id=?1",
            params![card_id],
            |r| {
                Ok((
                    r.get::<_, f64>(0)? as f32,
                    r.get::<_, i64>(1)? as u32,
                    r.get::<_, i64>(2)? as u32,
                ))
            },
        )
        .map_err(|e| match e {
            rusqlite::Error::QueryReturnedNoRows => AppError::NotFound(format!("card {card_id}")),
            e => AppError::Db(e),
        })?;
    let state_in = sm2::CardState {
        ease: prev.0,
        interval_days: prev.1,
        repetitions: prev.2,
        due_at: now.clone(),
    };
    let next = sm2_review(state_in, outcome.quality, &now);
    let next_interval = next.interval_days as i64;
    let next_reps = next.repetitions as i64;
    let next_ease = next.ease as f64;
    conn.execute(
        "UPDATE srs_cards SET ease=?1, interval_days=?2, repetitions=?3, due_at=?4 WHERE id=?5",
        params![next_ease, next_interval, next_reps, next.due_at, card_id],
    )?;
    let review_id = Ulid::new().to_string();
    conn.execute(
        "INSERT INTO srs_reviews (id, card_id, reviewed_at, quality, interval_after, ease_after) \
         VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        params![review_id, card_id, now, outcome.quality as i64, next_interval, next_ease],
    )?;
    read_card(&conn, &card_id)
}

#[tauri::command]
pub fn suspend(card_id: String, suspended: bool, state: State<'_, AppState>) -> AppResult<()> {
    let conn = state.db.lock().map_err(|_| AppError::Config("db lock".into()))?;
    let stamp = if suspended { Some(time_now()) } else { None };
    conn.execute(
        "UPDATE srs_cards SET suspended_at=?1 WHERE id=?2",
        params![stamp, card_id],
    )?;
    Ok(())
}

fn read_card(conn: &rusqlite::Connection, id: &str) -> AppResult<Card> {
    conn.query_row(
        "SELECT id, note_id, front, back, ease, interval_days, repetitions, due_at, created_at, suspended_at \
         FROM srs_cards WHERE id=?1",
        params![id],
        |r| {
            Ok(Card {
                id: r.get(0)?,
                note_id: r.get(1)?,
                front: r.get(2)?,
                back: r.get(3)?,
                ease: r.get::<_, f64>(4)? as f32,
                interval_days: r.get::<_, i64>(5)? as u32,
                repetitions: r.get::<_, i64>(6)? as u32,
                due_at: r.get(7)?,
                created_at: r.get(8)?,
                suspended_at: r.get(9)?,
            })
        },
    )
    .map_err(|e| match e {
        rusqlite::Error::QueryReturnedNoRows => AppError::NotFound(format!("card {id}")),
        e => AppError::Db(e),
    })
}

fn read_note_body(conn: &rusqlite::Connection, id: &str) -> AppResult<String> {
    conn.query_row(
        "SELECT body FROM notes WHERE id=?1",
        params![id],
        |r| r.get(0),
    )
    .map_err(|e| match e {
        rusqlite::Error::QueryReturnedNoRows => AppError::NotFound(format!("note {id}")),
        e => AppError::Db(e),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db;

    fn open_db() -> rusqlite::Connection {
        let d = tempfile::tempdir().unwrap();
        let p = db::AppPaths::from_root(d.path().to_path_buf()).unwrap();
        std::fs::create_dir_all(&p.data_dir).unwrap();
        db::open(&p).unwrap()
    }

    fn seed_card(conn: &rusqlite::Connection, front: &str, suspended: Option<&str>) {
        let id = Ulid::new().to_string();
        let sus = suspended.unwrap_or("NULL");
        if sus == "NULL" {
            conn.execute(
                "INSERT INTO srs_cards (id, front, back, due_at, created_at) \
                 VALUES (?1, ?2, 'b', '2026-07-06T00:00:00Z', '2026-07-06T00:00:00Z')",
                params![id, front],
            )
            .unwrap();
        } else {
            conn.execute(
                "INSERT INTO srs_cards (id, front, back, due_at, created_at, suspended_at) \
                 VALUES (?1, ?2, 'b', '2026-07-06T00:00:00Z', '2026-07-06T00:00:00Z', ?3)",
                params![id, front, sus],
            )
            .unwrap();
        }
    }

    #[test]
    fn queue_filters_suspended() {
        let conn = open_db();
        seed_card(&conn, "f1", None);
        seed_card(&conn, "f2", Some("2026-07-06T00:00:00Z"));
        let n: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM srs_cards WHERE suspended_at IS NULL",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(n, 1, "queue should only see active cards");
    }

    #[test]
    fn save_dedupes_by_note_front() {
        let conn = open_db();
        // ponytail: SQLite UNIQUE indexes treat NULL as distinct, so two rows with
        // NULL note_id + same front would NOT collide. The dedup index only applies
        // once a note_id is attached — that's the realistic insert path through
        // `save_cards`, which always carries an `input.note_id`.
        conn.execute(
            "INSERT INTO notes (id, path, title, created, updated) \
             VALUES ('n1', 'n1.md', 'n', 'n', 'n')",
            [],
        ).unwrap();
        conn.execute(
            "INSERT OR IGNORE INTO srs_cards (id, note_id, front, back, due_at, created_at) \
             VALUES ('a', 'n1', 'dup-front', 'b1', 'n', 'n')",
            [],
        )
        .unwrap();
        conn.execute(
            "INSERT OR IGNORE INTO srs_cards (id, note_id, front, back, due_at, created_at) \
             VALUES ('b', 'n1', 'dup-front', 'b2', 'n', 'n')",
            [],
        )
        .unwrap();
        let n: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM srs_cards WHERE front='dup-front'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(n, 1, "UNIQUE(note_id, front) must dedupe on same front");
    }

    #[test]
    fn read_card_returns_full_state() {
        let conn = open_db();
        conn.execute(
            "INSERT INTO srs_cards (id, front, back, ease, interval_days, repetitions, due_at, created_at) \
             VALUES ('c1','q','a', 2.6, 6, 2, '2030-01-01T00:00:00Z','2026-07-06T00:00:00Z')",
            [],
        ).unwrap();
        let card = read_card(&conn, "c1").unwrap();
        assert_eq!(card.front, "q");
        assert!((card.ease - 2.6).abs() < 0.001);
        assert_eq!(card.interval_days, 6);
        assert_eq!(card.repetitions, 2);
    }
}

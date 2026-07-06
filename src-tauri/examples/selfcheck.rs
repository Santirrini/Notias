// Notias self-check. Validates the foundation works on a fresh checkout.
// Run from src-tauri/: cargo run --example selfcheck
// Optional: OLLAMA_TEST_URL=http://127.0.0.1:11434 to verify Ollama HTTP path.

use notias_lib::ai::ollama::OllamaProvider;
use notias_lib::ai::provider::Provider;
use notias_lib::db::{self, AppPaths};
use tempfile::tempdir;

#[tokio::main]
async fn main() {
    let tmp = tempdir().expect("tempdir");
    let paths = AppPaths::from_root(tmp.path().to_path_buf()).expect("paths");
    std::fs::create_dir_all(&paths.data_dir).expect("mkdir");

    let conn = db::open(&paths).expect("db open");

    let v: i64 = conn
        .query_row("SELECT COALESCE(MAX(version), 0) FROM schema_version", [], |r| r.get(0))
        .expect("version");
    assert!(v >= 1, "no migration applied");

    db::write_meta(&paths.meta_file, &db::migrations::db_hash(&conn).unwrap(), v)
        .expect("meta write");
    let report = db::verify(&paths.meta_file, &conn).expect("verify");
    assert!(matches!(report, db::IntegrityReport::Ok), "expected Ok, got {:?}", report);

    println!("selfcheck OK (schema v{}, data_dir={:?})", v, paths.data_dir);

    if let Ok(url) = std::env::var("OLLAMA_TEST_URL") {
        let client = reqwest::Client::new();
        let p = OllamaProvider::new(url, client);
        match p.health().await {
            Ok(s) => println!("ollama health: healthy={} detail={:?}", s.healthy, s.detail),
            Err(e) => println!("ollama health error: {e}"),
        }
    }

    // ponytail: cheap structural assertions so CI catches drift. Real network tests
    // live in the manual smoke checklist — they need user credentials.
    use notias_lib::ai::{OpenAiProvider, GroqProvider, provider::Provider};
    let client = reqwest::Client::new();
    let _openai = OpenAiProvider::new("sk-fake".into(), client.clone());
    let _groq = GroqProvider::new("gsk-fake".into(), client);
    println!("phase-3 invariants: OpenAIProvider + GroqProvider construct");

    // Verify migration 0004 set the expected defaults.
    let count: i64 = conn
        .query_row("SELECT COUNT(*) FROM provider_settings", [], |r| r.get(0))
        .unwrap();
    assert!(count >= 3, "expected ollama/openai/groq rows, found {count}");

    let mut stmt = conn.prepare("SELECT name, chat_priority FROM provider_settings ORDER BY name").unwrap();
    let rows: Vec<(String, i64)> = stmt.query_map([], |r| Ok((r.get(0)?, r.get(1)?))).unwrap()
        .filter_map(Result::ok).collect();
    for (name, _) in &rows { println!("provider: {} priority", name); }

    // Phase 4 — SRS + quiz + plan invariants. No network calls.
    use notias_lib::ai::prompts;
    use notias_lib::srs::sm2::{review, CardState};
    use notias_lib::study::plan::{self as plan_mod, Plan, PlanBlock};

    // 1. SM-2: simulate Again → Good → Good → Good → Easy sequence.
    let mut s = CardState { ease: 2.5, interval_days: 0, repetitions: 0, due_at: "2026-07-06T00:00:00Z".into() };
    let baseline = s.ease;
    s = review(s, 1, "2026-07-06T00:00:00Z");
    assert!(s.ease < baseline, "ease should drop on Again");
    s = review(s, 4, "2026-07-06T00:00:00Z");
    s = review(s, 4, "2026-07-07T00:00:00Z");
    s = review(s, 4, "2026-07-13T00:00:00Z");
    s = review(s, 5, "2026-07-13T00:00:00Z");
    println!("phase-4 SM-2 final: ease={:.3} reps={} interval={}d", s.ease, s.repetitions, s.interval_days);

    // 2. Tasks: insert row, count >= 1.
    let now = notias_lib::time_util::time_now();
    conn.execute(
        "INSERT INTO tasks (id, title, priority, status, created_at, updated_at) VALUES ('p4t1','Read paper',1,'todo',?1,?1)",
        [&now],
    ).unwrap();
    let n: i64 = conn.query_row("SELECT COUNT(*) FROM tasks", [], |r| r.get(0)).unwrap();
    assert!(n >= 1, "task must persist");

    // 3. SRS: insert 3 cards, queue should return ≥3 (all due now).
    for f in ["Hecke operators", "Schur multipliers", "Borel subgroups"] {
        let id = ulid::Ulid::new().to_string();
        conn.execute(
            "INSERT INTO srs_cards (id, front, back, due_at, created_at) VALUES (?1, ?2, 'answer', '2026-07-06T00:00:00Z', '2026-07-06T00:00:00Z')",
            rusqlite::params![id, f],
        ).unwrap();
    }
    let q: i64 = conn.query_row(
        "SELECT COUNT(*) FROM srs_cards WHERE suspended_at IS NULL AND due_at <= ?1",
        [&now], |r| r.get(0),
    ).unwrap();
    assert!(q >= 3, "expected ≥3 due cards, got {q}");

    // 4. Plan: save and read back.
    let plan = Plan {
        week_start: "2026-07-06".into(),
        daily_hours_cap: 4,
        blocks: vec![PlanBlock {
            day: "2026-07-06".into(), start: "09:00".into(), minutes: 60,
            kind: "review".into(), refs: vec![], rationale: "warmup".into(),
        }],
    };
    let _ = plan_mod::save(&conn, &plan).unwrap();
    let got = plan_mod::get(&conn, "2026-07-06").unwrap();
    assert!(got.is_some(), "plan must round-trip");
    println!("phase-4 selfcheck OK: tasks={n} srs_due={q} plan_persisted=true");

    // 5. Prompts contain required content.
    assert!(prompts::cards_generate("body", 5).contains("5"));
    assert!(prompts::quiz_generate("body", 3).contains("JSON"));
    assert!(prompts::plan_generate("ctx", "2026-07-06", 4).contains("2026-07-06"));
}
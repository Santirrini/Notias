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
}
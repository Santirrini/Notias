// Notias self-check. Validates the foundation works on a fresh checkout.
// Run from src-tauri/: cargo run --example selfcheck

use notias_lib::db::{self, AppPaths};
use rusqlite::Connection;
use tempfile::tempdir;

fn main() {
    // ponytail: hermetic — uses a tempdir so the dev machine's real data dir is never touched.
    let tmp = tempdir().expect("tempdir");
    let paths = AppPaths::from_root(tmp.path().to_path_buf()).expect("paths");
    std::fs::create_dir_all(&paths.data_dir).expect("mkdir");

    let conn = Connection::open(&paths.db_file).expect("open db");
    conn.pragma_update(None, "journal_mode", "WAL").unwrap();
    db::migrations::run(&conn).expect("migrations");

    let v: i64 = conn
        .query_row("SELECT COALESCE(MAX(version), 0) FROM schema_version", [], |r| r.get(0))
        .expect("version");
    assert!(v >= 1, "no migration applied");

    db::write_meta(&paths.meta_file, &db::migrations::db_hash(&conn).unwrap(), v)
        .expect("meta write");
    let report = db::verify(&paths.db_file, &paths.meta_file, &conn).expect("verify");
    assert!(matches!(report, db::IntegrityReport::Ok), "expected Ok, got {:?}", report);

    println!("selfcheck OK (schema v{}, data_dir={:?})", v, paths.data_dir);
}
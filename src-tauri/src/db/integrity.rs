use crate::error::AppResult;
use rusqlite::Connection;
use std::path::Path;

pub fn write_meta(meta_path: &Path, db_hash: &str, schema_version: i64) -> AppResult<()> {
    let body = serde_json::json!({
        "db_hash": db_hash,
        "schema_version": schema_version,
    });
    std::fs::write(meta_path, serde_json::to_vec_pretty(&body)?)?;
    Ok(())
}

pub fn verify(_db_path: &Path, meta_path: &Path, conn: &Connection) -> AppResult<IntegrityReport> {
    let actual = super::migrations::db_hash(conn)?;
    let report = if !meta_path.exists() {
        IntegrityReport::Fresh
    } else {
        let body: serde_json::Value =
            serde_json::from_slice(&std::fs::read(meta_path)?)?;
        let stored = body["db_hash"].as_str().unwrap_or("");
        if stored == actual { IntegrityReport::Ok } else { IntegrityReport::Mismatch }
    };
    Ok(report)
}

pub enum IntegrityReport {
    Ok,
    Fresh,
    Mismatch,
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn fresh_when_no_meta() {
        let dir = tempdir().unwrap();
        let conn = Connection::open_in_memory().unwrap();
        let r = verify(&dir.path().join("x.db"), &dir.path().join("notias.meta"), &conn).unwrap();
        assert!(matches!(r, IntegrityReport::Fresh));
    }
}
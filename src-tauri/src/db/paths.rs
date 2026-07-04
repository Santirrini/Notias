use directories::ProjectDirs;
use std::path::{Path, PathBuf};

pub struct AppPaths {
    pub data_dir: PathBuf,
    pub notes_dir: PathBuf,
    pub db_file: PathBuf,
    pub meta_file: PathBuf,
    pub config_file: PathBuf,
}

impl AppPaths {
    /// Production constructor — resolves the OS data dir. Used in app startup.
    pub fn new() -> Result<Self, crate::error::AppError> {
        Self::from_root(default_root()?)
    }

    /// Test-friendly constructor — caller supplies the root so tests stay hermetic.
    pub fn from_root(data_dir: PathBuf) -> Result<Self, crate::error::AppError> {
        if data_dir.as_os_str().is_empty() {
            return Err(crate::error::AppError::Config("empty data dir".into()));
        }
        Ok(Self {
            notes_dir: data_dir.join("notes"),
            db_file: data_dir.join("notias.db"),
            meta_file: data_dir.join("notias.meta"),
            config_file: data_dir.join("config.json"),
            data_dir,
        })
    }
}

fn default_root() -> Result<PathBuf, crate::error::AppError> {
    let dirs = ProjectDirs::from("app", "Notias", "Notias")
        .ok_or_else(|| crate::error::AppError::Config("no project dirs".into()))?;
    Ok(dirs.data_dir().to_path_buf())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn from_root_has_expected_suffixes() {
        let dir = tempdir().unwrap();
        let p = AppPaths::from_root(dir.path().to_path_buf()).unwrap();
        assert!(p.notes_dir.ends_with("notes"));
        assert!(p.db_file.ends_with("notias.db"));
        assert!(p.meta_file.ends_with("notias.meta"));
        assert!(p.config_file.ends_with("config.json"));
        assert_eq!(p.notes_dir.parent().unwrap(), dir.path());
    }

    #[test]
    fn from_root_rejects_empty() {
        let r = AppPaths::from_root(PathBuf::new());
        assert!(r.is_err());
    }
}

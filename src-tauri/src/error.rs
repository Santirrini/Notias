use serde::Serialize;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum AppError {
    #[error("io: {0}")]
    Io(#[from] std::io::Error),
    #[error("db: {0}")]
    Db(#[from] rusqlite::Error),
    #[error("json: {0}")]
    Json(#[from] serde_json::Error),
    #[error("yaml: {0}")]
    Yaml(#[from] serde_yaml::Error),
    #[error("keyring: {0}")]
    Keyring(#[from] keyring::Error),
    #[error("zip: {0}")]
    Zip(#[from] zip::result::ZipError),
    #[error("config: {0}")]
    Config(String),
    #[error("not found: {0}")]
    NotFound(String),
    #[error("invalid: {0}")]
    Invalid(String),
    #[error("provider: {0}")]
    Provider(String),
    #[error("auth: {0}")]
    Auth(String),
}

#[derive(Serialize)]
struct WireError<'a> {
    code: &'a str,
    message: String,
}

impl Serialize for AppError {
    fn serialize<S: serde::Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        let code = match self {
            AppError::Io(_) => "io",
            AppError::Db(_) => "db",
            AppError::Json(_) | AppError::Yaml(_) => "serialization", // ponytail: prefix-free wire codes; the message string carries the variant name.
            AppError::Keyring(_) => "auth",
            AppError::Zip(_) => "io",
            AppError::Config(_) => "config",
            AppError::NotFound(_) => "not_found",
            AppError::Invalid(_) => "invalid",
            AppError::Provider(_) => "provider",
            AppError::Auth(_) => "auth",
        };
        WireError { code, message: self.to_string() }.serialize(s)
    }
}

pub type AppResult<T> = std::result::Result<T, AppError>;
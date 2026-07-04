use crate::error::{AppError, AppResult};
use keyring::Entry;

const SERVICE: &str = "com.notias.app";

pub fn set(provider: &str, key: &str) -> AppResult<()> {
    Entry::new(SERVICE, provider)?.set_password(key)?;
    Ok(())
}

pub fn get(provider: &str) -> AppResult<Option<String>> {
    match Entry::new(SERVICE, provider)?.get_password() {
        Ok(s) => Ok(Some(s)),
        Err(keyring::Error::NoEntry) => Ok(None),
        Err(e) => Err(AppError::Auth(e.to_string())),
    }
}

pub fn delete(provider: &str) -> AppResult<()> {
    match Entry::new(SERVICE, provider)?.delete_password() {
        Ok(()) | Err(keyring::Error::NoEntry) => Ok(()),
        Err(e) => Err(AppError::Auth(e.to_string())),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    // ponytail: keyring backend is per-OS and flaky in CI; tests are smoke-only on dev machines.
    #[test]
    #[ignore]
    fn round_trip() {
        let k = "sk-test-notias";
        set("openai", k).unwrap();
        let got = get("openai").unwrap();
        assert_eq!(got.as_deref(), Some(k));
        delete("openai").unwrap();
    }
}
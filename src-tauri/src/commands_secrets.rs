use crate::error::AppResult;
use crate::secrets;
use serde::Serialize;

#[tauri::command]
pub fn set_provider_key(name: String, key: String) -> AppResult<()> {
    if key.is_empty() {
        return Err(crate::error::AppError::Auth("empty key".into()));
    }
    secrets::set(&name, &key)
}

#[tauri::command]
pub fn delete_provider_key(name: String) -> AppResult<()> {
    secrets::delete(&name)
}

#[tauri::command]
pub fn has_provider_key(name: String) -> AppResult<bool> {
    Ok(secrets::get(&name)?.is_some())
}

#[derive(Serialize)]
pub struct ProviderKeyStatus {
    pub name: String,
    pub has_key: bool,
}

#[tauri::command]
pub fn provider_key_status() -> AppResult<Vec<ProviderKeyStatus>> {
    let providers = ["openai", "groq"];
    Ok(providers
        .iter()
        .map(|n| ProviderKeyStatus {
            name: n.to_string(),
            has_key: secrets::get(n).ok().flatten().is_some(),
        })
        .collect())
}

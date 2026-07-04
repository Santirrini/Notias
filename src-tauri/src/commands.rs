use crate::error::AppResult;

#[tauri::command]
pub fn ping() -> AppResult<String> {
    Ok("pong".into())
}

use crate::error::AppResult;
use crate::AppState;

#[tauri::command]
pub fn ping() -> AppResult<String> {
    Ok("pong".into())
}

#[tauri::command]
pub fn recovery_required(state: tauri::State<'_, AppState>) -> bool {
    state.recovery_required
}

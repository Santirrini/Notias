pub mod model;
pub mod quiz;
pub mod plan;

use crate::ai::json_helpers::{first_json, JsonShape};
use crate::ai::provider::CompleteRequest;
use crate::ai::prompts::plan_generate;
use crate::error::{AppError, AppResult};
use crate::AppState;
use tauri::State;

use model::{NewQuizInput, Quiz, QuizResult};
use plan::Plan;

#[tauri::command]
pub async fn generate_quiz(input: NewQuizInput, state: State<'_, AppState>) -> AppResult<Quiz> {
    let mut body = String::new();
    {
        let conn = state
            .db
            .lock()
            .map_err(|_| AppError::Config("db lock".into()))?;
        for nid in &input.note_ids {
            let b: String = conn
                .query_row(
                    "SELECT body FROM notes WHERE id=?1",
                    rusqlite::params![nid],
                    |r| r.get(0),
                )
                .unwrap_or_default();
            if !b.is_empty() {
                body.push_str(&format!("\n\n# {}\n{}", nid, b));
            }
        }
    }
    let provider = state.router.pick_chat().await?;
    quiz::generate(input, &body, &*provider).await
}

#[tauri::command]
pub async fn grade_quiz(
    quiz: serde_json::Value,
    answers: Vec<String>,
    state: State<'_, AppState>,
) -> AppResult<QuizResult> {
    let q: Quiz = serde_json::from_value(quiz)
        .map_err(|e| AppError::Invalid(format!("quiz json: {e}")))?;
    let result = crate::study::quiz::grade(&q, &answers);
    {
        let conn = state
            .db
            .lock()
            .map_err(|_| AppError::Config("db lock".into()))?;
        let _ = crate::study::quiz::save_attempt(&conn, &q, &answers, &result)?;
    }
    Ok(result)
}

#[tauri::command]
pub async fn generate_plan(
    week_start: String,
    daily_hours_cap: u8,
    state: State<'_, AppState>,
) -> AppResult<Plan> {
    let context = {
        let conn = state
            .db
            .lock()
            .map_err(|_| AppError::Config("db lock".into()))?;
        plan::build_context(&conn, &week_start, daily_hours_cap)?
    };
    let provider = state.router.pick_chat().await?;
    let prompt = plan_generate(&context, &week_start, daily_hours_cap);
    let out = provider
        .complete(CompleteRequest {
            prompt,
            model: "llama3.2".into(),
            max_tokens: Some(1200),
        })
        .await?;
    let p: Plan = first_json(&out.text, JsonShape::Object)
        .map_err(|e| AppError::Provider(format!("plan parse: {e}")))?;
    if let Err(e) = plan::validate(&p) {
        return Err(AppError::Provider(format!("plan invalid: {e}")));
    }
    Ok(p)
}

#[tauri::command]
pub fn save_plan(plan: Plan, state: State<'_, AppState>) -> AppResult<String> {
    let conn = state
        .db
        .lock()
        .map_err(|_| AppError::Config("db lock".into()))?;
    plan::save(&conn, &plan)
}

#[tauri::command]
pub fn get_plan(week_start: String, state: State<'_, AppState>) -> AppResult<Option<Plan>> {
    let conn = state
        .db
        .lock()
        .map_err(|_| AppError::Config("db lock".into()))?;
    plan::get(&conn, &week_start)
}

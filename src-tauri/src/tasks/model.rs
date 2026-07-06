use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Clone)]
pub struct Task {
    pub id: String,
    pub note_id: Option<String>,
    pub title: String,
    pub priority: u8,
    pub status: String,
    pub due_at: Option<String>,
    pub created_at: String,
    pub updated_at: String,
    pub done_at: Option<String>,
}

#[derive(Debug, Serialize, Clone)]
pub struct TaskSummary {
    pub id: String,
    pub title: String,
    pub priority: u8,
    pub status: String,
    pub due_at: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct NewTask {
    pub title: String,
    #[serde(default)]
    pub priority: u8,
    #[serde(default)]
    pub note_id: Option<String>,
    #[serde(default)]
    pub due_at: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct TaskPatch {
    pub title: Option<String>,
    pub priority: Option<u8>,
    pub status: Option<String>,
    pub due_at: Option<Option<String>>,
}

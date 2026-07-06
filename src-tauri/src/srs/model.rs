use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Clone)]
pub struct Card {
    pub id: String,
    pub note_id: Option<String>,
    pub front: String,
    pub back: String,
    pub ease: f32,
    pub interval_days: u32,
    pub repetitions: u32,
    pub due_at: String,
    pub created_at: String,
    pub suspended_at: Option<String>,
}

#[derive(Debug, Serialize, Clone)]
pub struct CardSummary {
    pub id: String,
    pub front: String,
    pub back: String,
    pub due_at: String,
}

#[derive(Debug, Deserialize)]
pub struct DraftCard {
    pub front: String,
    pub back: String,
}

#[derive(Debug, Deserialize)]
pub struct ReviewOutcome {
    pub quality: u8,
}

#[derive(Debug, Deserialize)]
pub struct SaveCardsInput {
    pub note_id: Option<String>,
    pub cards: Vec<DraftCard>,
}

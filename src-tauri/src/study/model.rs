use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Question {
    Mc {
        question: String,
        choices: Vec<String>,
        #[serde(default)]
        answer: usize,
        #[serde(default)]
        rationale: String,
    },
    Short {
        question: String,
        #[serde(default)]
        answer: String,
        #[serde(default)]
        rationale: String,
    },
    Cloze {
        question: String,
        #[serde(default)]
        answer: String,
        #[serde(default)]
        rationale: String,
    },
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Quiz {
    pub id: String,
    pub note_ids: Vec<String>,
    pub questions: Vec<Question>,
    pub created_at: String,
}

#[derive(Debug, Serialize, Clone)]
pub struct QuizResult {
    pub quiz_id: String,
    pub score: u32,
    pub total: u32,
    pub per_question: Vec<QuestionOutcome>,
}

#[derive(Debug, Serialize, Clone)]
pub struct QuestionOutcome {
    pub index: usize,
    pub correct: bool,
    pub expected: String,
    pub given: String,
    pub rationale: String,
}

#[derive(Debug, Deserialize)]
pub struct NewQuizInput {
    pub note_ids: Vec<String>,
    pub count: u8,
}

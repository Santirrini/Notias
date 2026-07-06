use crate::ai::json_helpers::{first_json, JsonShape};
use crate::ai::prompts::quiz_generate;
use crate::ai::provider::CompleteRequest;
use crate::error::{AppError, AppResult};
use crate::study::model::{NewQuizInput, Question, QuestionOutcome, Quiz, QuizResult};
use crate::time_util::time_now;
use rusqlite::params;
use ulid::Ulid;

pub async fn generate(
    input: NewQuizInput,
    pool_body: &str,
    provider: &dyn crate::ai::Provider,
) -> AppResult<Quiz> {
    if !(1..=20).contains(&input.count) {
        return Err(AppError::Invalid("count must be 1..=20".into()));
    }
    if input.note_ids.is_empty() {
        return Err(AppError::Invalid("note_ids empty".into()));
    }
    let prompt = quiz_generate(pool_body, input.count);
    let out = provider
        .complete(CompleteRequest {
            prompt,
            model: "llama3.2".into(),
            max_tokens: Some(1500),
        })
        .await?;
    let qs: Vec<Question> = first_json(&out.text, JsonShape::Array)
        .map_err(|e| AppError::Provider(format!("quiz parse: {e}")))?;
    if qs.is_empty() {
        return Err(AppError::Provider("quiz parse returned empty".into()));
    }
    Ok(Quiz {
        id: Ulid::new().to_string(),
        note_ids: input.note_ids,
        questions: qs,
        created_at: time_now(),
    })
}

pub fn grade(quiz: &Quiz, answers: &[String]) -> QuizResult {
    // ponytail: multiple-choice equality; short/cloze use case-insensitive trim match.
    // Rationales are returned verbatim from the quiz regardless of score.
    let mut per_q: Vec<QuestionOutcome> = vec![];
    let mut score = 0u32;
    for (i, q) in quiz.questions.iter().enumerate() {
        let given = answers.get(i).cloned().unwrap_or_default();
        let (correct, expected, rationale) = match q {
            Question::Mc { choices, answer, rationale } => {
                let exp = *answer;
                let exp_str = choices.get(exp).cloned().unwrap_or_default();
                let given_idx = given.trim().parse::<usize>().ok();
                let ok = Some(exp) == given_idx;
                (ok, format!("index={exp} ({exp_str})"), rationale.clone())
            }
            Question::Short { answer, rationale } => {
                let norm = answer.trim().to_lowercase();
                let ok = norm == given.trim().to_lowercase();
                (ok, answer.clone(), rationale.clone())
            }
            Question::Cloze { answer, rationale } => {
                let norm = answer.trim().to_lowercase();
                let ok = norm == given.trim().to_lowercase();
                (ok, answer.clone(), rationale.clone())
            }
        };
        if correct {
            score += 1;
        }
        per_q.push(QuestionOutcome {
            index: i,
            correct,
            expected,
            given,
            rationale,
        });
    }
    QuizResult {
        quiz_id: quiz.id.clone(),
        score,
        total: quiz.questions.len() as u32,
        per_question: per_q,
    }
}

pub fn save_attempt(
    conn: &rusqlite::Connection,
    quiz: &Quiz,
    answers: &[String],
    result: &QuizResult,
) -> AppResult<String> {
    let id = Ulid::new().to_string();
    let qjson = serde_json::to_string(&quiz.questions)?;
    let ajson = serde_json::to_string(answers)?;
    let now = time_now();
    conn.execute(
        "INSERT INTO quiz_attempts (id, note_id, questions_json, answers_json, score, total, created_at, completed_at) \
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?7)",
        params![
            id,
            quiz.note_ids.first().cloned().unwrap_or_default(),
            qjson,
            ajson,
            result.score as i64,
            result.total as i64,
            now
        ],
    )?;
    Ok(id)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::study::model::Question;

    #[test]
    fn mc_correct_when_index_matches() {
        let q = Question::Mc {
            question: "x".into(),
            choices: vec!["a".into(), "b".into()],
            answer: 1,
            rationale: "because b".into(),
        };
        let quiz = Quiz {
            id: "q".into(),
            note_ids: vec![],
            questions: vec![q],
            created_at: "now".into(),
        };
        let r = grade(&quiz, &["1".into()]);
        assert!(r.per_question[0].correct);
        assert_eq!(r.score, 1);
        assert_eq!(r.total, 1);
    }

    #[test]
    fn short_grade_is_case_insensitive_trim() {
        let q = Question::Short {
            question: "x".into(),
            answer: "Photosynthesis".into(),
            rationale: "r".into(),
        };
        let quiz = Quiz {
            id: "q".into(),
            note_ids: vec![],
            questions: vec![q],
            created_at: "now".into(),
        };
        let r = grade(&quiz, &["  photosynthesis ".into()]);
        assert!(r.per_question[0].correct);
    }

    #[test]
    fn cloze_works_like_short() {
        let q = Question::Cloze {
            question: "The capital of {{France}} is Paris".into(),
            answer: "Paris".into(),
            rationale: "well-known".into(),
        };
        let quiz = Quiz {
            id: "q".into(),
            note_ids: vec![],
            questions: vec![q],
            created_at: "now".into(),
        };
        let r = grade(&quiz, &["paris".into()]);
        assert!(r.per_question[0].correct);
    }

    #[test]
    fn empty_quiz_grade_is_zero() {
        let quiz = Quiz {
            id: "q".into(),
            note_ids: vec![],
            questions: vec![],
            created_at: "now".into(),
        };
        let r = grade(&quiz, &[]);
        assert_eq!(r.score, 0);
        assert_eq!(r.total, 0);
    }

    #[test]
    fn wrong_mc_does_not_score() {
        let q = Question::Mc {
            question: "x".into(),
            choices: vec!["a".into(), "b".into()],
            answer: 0,
            rationale: "a".into(),
        };
        let quiz = Quiz {
            id: "q".into(),
            note_ids: vec![],
            questions: vec![q],
            created_at: "now".into(),
        };
        let r = grade(&quiz, &["1".into()]);
        assert!(!r.per_question[0].correct);
        assert_eq!(r.score, 0);
    }
}

pub mod model;

use crate::error::{AppError, AppResult};
use crate::time_util::time_now;
use crate::AppState;
use rusqlite::params;
use tauri::State;
use ulid::Ulid;

use model::{NewTask, Task, TaskPatch, TaskSummary};

const ALLOWED_STATUSES: &[&str] = &["todo", "doing", "done", "cancelled"];

#[tauri::command]
pub fn list_tasks(state: State<'_, AppState>) -> AppResult<Vec<TaskSummary>> {
    let conn = state.db.lock().map_err(|_| AppError::Config("db lock".into()))?;
    let mut stmt = conn.prepare(
        "SELECT id, title, priority, status, due_at FROM tasks \
         ORDER BY (status='done') ASC, priority DESC, due_at ASC NULLS LAST LIMIT 500",
    )?;
    let rows = stmt.query_map([], |r| {
        Ok(TaskSummary {
            id: r.get(0)?,
            title: r.get(1)?,
            priority: r.get(2)?,
            status: r.get(3)?,
            due_at: r.get(4)?,
        })
    })?.filter_map(Result::ok).collect();
    Ok(rows)
}

#[tauri::command]
pub fn create_task(input: NewTask, state: State<'_, AppState>) -> AppResult<Task> {
    if input.title.trim().is_empty() {
        return Err(AppError::Invalid("title required".into()));
    }
    let id = Ulid::new().to_string();
    let now = time_now();
    let conn = state.db.lock().map_err(|_| AppError::Config("db lock".into()))?;
    conn.execute(
        "INSERT INTO tasks (id, note_id, title, priority, status, due_at, created_at, updated_at)
         VALUES (?1, ?2, ?3, ?4, 'todo', ?5, ?6, ?6)",
        params![id, input.note_id, input.title, input.priority as i64, input.due_at, now],
    )?;
    Ok(Task {
        id,
        note_id: input.note_id,
        title: input.title,
        priority: input.priority,
        status: "todo".into(),
        due_at: input.due_at,
        created_at: now.clone(),
        updated_at: now,
        done_at: None,
    })
}

#[tauri::command]
pub fn update_task(id: String, patch: TaskPatch, state: State<'_, AppState>) -> AppResult<Task> {
    let now = time_now();
    {
        let conn = state.db.lock().map_err(|_| AppError::Config("db lock".into()))?;
        let mut tx = conn.unchecked_transaction()?;
        if let Some(t) = &patch.title {
            tx.execute(
                "UPDATE tasks SET title=?1, updated_at=?2 WHERE id=?3",
                params![t, now, id],
            )?;
        }
        if let Some(p) = patch.priority {
            tx.execute(
                "UPDATE tasks SET priority=?1, updated_at=?2 WHERE id=?3",
                params![p as i64, now, id],
            )?;
        }
        if let Some(s) = &patch.status {
            if !ALLOWED_STATUSES.contains(&s.as_str()) {
                return Err(AppError::Invalid(format!("bad status: {s}")));
            }
            tx.execute(
                "UPDATE tasks SET status=?1, updated_at=?2, done_at=CASE WHEN ?1='done' THEN ?2 WHEN done_at IS NOT NULL THEN NULL ELSE done_at END WHERE id=?3",
                params![s, now, id],
            )?;
        }
        if let Some(due) = &patch.due_at {
            tx.execute(
                "UPDATE tasks SET due_at=?1, updated_at=?2 WHERE id=?3",
                params![due, now, id],
            )?;
        }
        tx.commit()?;
    }
    read_one(&id, state)
}

#[tauri::command]
pub fn delete_task(id: String, state: State<'_, AppState>) -> AppResult<()> {
    let conn = state.db.lock().map_err(|_| AppError::Config("db lock".into()))?;
    conn.execute("DELETE FROM tasks WHERE id=?1", params![id])?;
    Ok(())
}

fn read_one(id: &str, state: State<'_, AppState>) -> AppResult<Task> {
    let conn = state.db.lock().map_err(|_| AppError::Config("db lock".into()))?;
    conn.query_row(
        "SELECT id, note_id, title, priority, status, due_at, created_at, updated_at, done_at \
         FROM tasks WHERE id=?1",
        params![id],
        |r| {
            Ok(Task {
                id: r.get(0)?,
                note_id: r.get(1)?,
                title: r.get(2)?,
                priority: r.get(3)?,
                status: r.get(4)?,
                due_at: r.get(5)?,
                created_at: r.get(6)?,
                updated_at: r.get(7)?,
                done_at: r.get(8)?,
            })
        },
    )
    .map_err(|e| match e {
        rusqlite::Error::QueryReturnedNoRows => AppError::NotFound(format!("task {id}")),
        e => AppError::Db(e),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db;

    fn open_db() -> rusqlite::Connection {
        let d = tempfile::tempdir().unwrap();
        let p = db::AppPaths::from_root(d.path().to_path_buf()).unwrap();
        std::fs::create_dir_all(&p.data_dir).unwrap();
        db::open(&p).unwrap()
    }

    #[test]
    fn allowed_statuses_includes_done() {
        assert!(ALLOWED_STATUSES.contains(&"done"));
        assert!(!ALLOWED_STATUSES.contains(&"random"));
    }

    #[test]
    fn insert_and_select() {
        let conn = open_db();
        conn.execute(
            "INSERT INTO tasks (id, title, priority, status, created_at, updated_at) \
             VALUES ('t1','Test',1,'todo','2026-07-06T00:00:00Z','2026-07-06T00:00:00Z')",
            [],
        ).unwrap();
        let (title, status): (String, String) = conn
            .query_row("SELECT title, status FROM tasks WHERE id='t1'", [], |r| {
                Ok((r.get(0)?, r.get(1)?))
            })
            .unwrap();
        assert_eq!(title, "Test");
        assert_eq!(status, "todo");
    }
}

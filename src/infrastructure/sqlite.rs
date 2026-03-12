use crate::domain::repository::TaskRepository;
use crate::domain::task::{Status, Task};
use anyhow::Result;
use rusqlite::{Connection, Row, params};
use std::path::Path;
use uuid::Uuid;

pub struct SqliteRepository {
    conn: Connection,
}

impl SqliteRepository {
    pub fn new<P: AsRef<Path>>(path: P) -> Result<Self> {
        let conn = Connection::open(path)?;

        // WAL モードとビジータイムアウトの設定
        conn.pragma_update(None, "journal_mode", "WAL")?;
        conn.pragma_update(None, "busy_timeout", 5000)?;

        let repo = Self { conn };
        repo.init_db()?;
        Ok(repo)
    }

    fn init_db(&self) -> Result<()> {
        self.conn.execute(
            "CREATE TABLE IF NOT EXISTS tasks (
                local_id INTEGER PRIMARY KEY AUTOINCREMENT,
                global_id TEXT NOT NULL UNIQUE,
                title TEXT NOT NULL,
                description TEXT,
                status TEXT NOT NULL,
                parent_global_id TEXT,
                linked_files TEXT,
                created_at DATETIME NOT NULL,
                updated_at DATETIME NOT NULL
            )",
            [],
        )?;
        Ok(())
    }

    fn map_row(row: &Row) -> rusqlite::Result<Task> {
        let global_id_str: String = row.get("global_id")?;
        let parent_global_id_str: Option<String> = row.get("parent_global_id")?;
        let linked_files_json: Option<String> = row.get("linked_files")?;

        let linked_files = match linked_files_json {
            Some(json) => serde_json::from_str(&json).unwrap_or_default(),
            None => Vec::new(),
        };

        Ok(Task {
            local_id: Some(row.get("local_id")?),
            global_id: Uuid::parse_str(&global_id_str).unwrap_or_default(),
            title: row.get("title")?,
            description: row.get("description")?,
            status: Status::from(row.get::<_, String>("status")?),
            parent_global_id: parent_global_id_str.and_then(|s| Uuid::parse_str(&s).ok()),
            linked_files,
            created_at: row.get("created_at")?,
            updated_at: row.get("updated_at")?,
        })
    }
}

impl TaskRepository for SqliteRepository {
    fn save(&self, task: &Task) -> Result<()> {
        let linked_files_json = serde_json::to_string(&task.linked_files)?;
        let parent_global_id = task.parent_global_id.map(|u| u.to_string());

        self.conn.execute(
            "INSERT INTO tasks (
                global_id, title, description, status, parent_global_id, linked_files, created_at, updated_at
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)
            ON CONFLICT(global_id) DO UPDATE SET
                title = excluded.title,
                description = excluded.description,
                status = excluded.status,
                parent_global_id = excluded.parent_global_id,
                linked_files = excluded.linked_files,
                updated_at = excluded.updated_at",
            params![
                task.global_id.to_string(),
                task.title,
                task.description,
                task.status.to_string(),
                parent_global_id,
                linked_files_json,
                task.created_at,
                task.updated_at,
            ],
        )?;
        Ok(())
    }

    fn find_by_local_id(&self, local_id: i32) -> Result<Option<Task>> {
        let mut stmt = self
            .conn
            .prepare("SELECT * FROM tasks WHERE local_id = ?1")?;
        let mut rows = stmt.query(params![local_id])?;

        if let Some(row) = rows.next()? {
            Ok(Some(Self::map_row(row)?))
        } else {
            Ok(None)
        }
    }

    fn find_by_global_id(&self, global_id: Uuid) -> Result<Option<Task>> {
        let mut stmt = self
            .conn
            .prepare("SELECT * FROM tasks WHERE global_id = ?1")?;
        let mut rows = stmt.query(params![global_id.to_string()])?;

        if let Some(row) = rows.next()? {
            Ok(Some(Self::map_row(row)?))
        } else {
            Ok(None)
        }
    }

    fn find_all(&self) -> Result<Vec<Task>> {
        let mut stmt = self.conn.prepare("SELECT * FROM tasks ORDER BY local_id")?;
        let task_iter = stmt.query_map([], Self::map_row)?;

        let mut tasks = Vec::new();
        for task in task_iter {
            tasks.push(task?);
        }
        Ok(tasks)
    }

    fn delete(&self, local_id: i32) -> Result<()> {
        self.conn
            .execute("DELETE FROM tasks WHERE local_id = ?1", params![local_id])?;
        Ok(())
    }
}

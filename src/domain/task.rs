use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Serialize, Deserialize, Clone, Copy, PartialEq, Eq)]
pub enum Status {
    Open,
    InProgress,
    Pending,
    Close,
}

impl std::fmt::Display for Status {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Status::Open => write!(f, "Open"),
            Status::InProgress => write!(f, "In Progress"),
            Status::Pending => write!(f, "Pending"),
            Status::Close => write!(f, "Close"),
        }
    }
}

impl From<String> for Status {
    fn from(s: String) -> Self {
        match s.to_lowercase().as_str() {
            "open" => Status::Open,
            "inprogress" | "in progress" | "in_progress" => Status::InProgress,
            "pending" => Status::Pending,
            "close" | "closed" => Status::Close,
            _ => Status::Open,
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Task {
    pub local_id: Option<i32>,
    pub global_id: Uuid,
    pub title: String,
    pub description: Option<String>,
    pub status: Status,
    pub parent_global_id: Option<Uuid>,
    pub linked_files: Vec<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl Task {
    pub fn new(title: String, description: Option<String>, parent_global_id: Option<Uuid>) -> Self {
        let now = Utc::now();
        Self {
            local_id: None,
            global_id: Uuid::new_v4(),
            title,
            description,
            status: Status::Open,
            parent_global_id,
            linked_files: Vec::new(),
            created_at: now,
            updated_at: now,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_status_from_string() {
        assert_eq!(Status::from("open".to_string()), Status::Open);
        assert_eq!(Status::from("inprogress".to_string()), Status::InProgress);
        assert_eq!(Status::from("pending".to_string()), Status::Pending);
        assert_eq!(Status::from("close".to_string()), Status::Close);
        assert_eq!(Status::from("unknown".to_string()), Status::Open);
    }

    #[test]
    fn test_task_creation() {
        let title = "Test Task".to_string();
        let task = Task::new(title.clone(), None, None);
        assert_eq!(task.title, title);
        assert_eq!(task.status, Status::Open);
        assert!(task.local_id.is_none());
        assert!(task.linked_files.is_empty());
    }
}

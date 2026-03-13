use crate::domain::task::Task;
use anyhow::{Context, Result};
use std::fs::{self, File};
use std::path::{Path, PathBuf};

pub struct JsonRepository {
    base_path: PathBuf,
}

impl JsonRepository {
    pub fn new<P: AsRef<Path>>(path: P) -> Self {
        Self {
            base_path: path.as_ref().to_path_buf(),
        }
    }

    fn get_task_path(&self, global_id: &uuid::Uuid) -> PathBuf {
        let id_str = global_id.to_string();
        let prefix = &id_str[0..2];
        self.base_path.join(prefix).join(format!("{}.json", id_str))
    }

    pub fn save_task(&self, task: &Task) -> Result<()> {
        let path = self.get_task_path(&task.global_id);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        let file = File::create(&path)
            .with_context(|| format!("Failed to create JSON file: {:?}", path))?;
        serde_json::to_writer_pretty(file, task)
            .with_context(|| "Failed to write task to JSON")?;
        Ok(())
    }

    pub fn save_all(&self, tasks: &[Task]) -> Result<()> {
        for task in tasks {
            self.save_task(task)?;
        }
        Ok(())
    }

    pub fn load_all(&self) -> Result<Vec<Task>> {
        let mut tasks = Vec::new();
        if !self.base_path.exists() {
            return Ok(tasks);
        }

        for entry in walkdir::WalkDir::new(&self.base_path) {
            let entry = entry?;
            if entry.file_type().is_file() && entry.path().extension().is_some_and(|ext| ext == "json") {
                let file = File::open(entry.path())?;
                let task: Task = serde_json::from_reader(file)
                    .with_context(|| format!("Failed to parse task from {:?}", entry.path()))?;
                tasks.push(task);
            }
        }
        Ok(tasks)
    }

    #[allow(dead_code)]
    pub fn delete_task(&self, global_id: &uuid::Uuid) -> Result<()> {
        let path = self.get_task_path(global_id);
        if path.exists() {
            fs::remove_file(path)?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::task::Task;
    use tempfile::tempdir;

    #[test]
    fn test_save_and_load_all() -> Result<()> {
        let dir = tempdir()?;
        let repo = JsonRepository::new(dir.path());

        let task1 = Task::new("Task 1".to_string(), None, None);
        let task2 = Task::new("Task 2".to_string(), None, None);

        repo.save_task(&task1)?;
        repo.save_task(&task2)?;

        let loaded = repo.load_all()?;
        assert_eq!(loaded.len(), 2);
        
        let titles: Vec<String> = loaded.iter().map(|t| t.title.clone()).collect();
        assert!(titles.contains(&"Task 1".to_string()));
        assert!(titles.contains(&"Task 2".to_string()));

        // パスの階層確認 (先頭2文字)
        let id_str = task1.global_id.to_string();
        let expected_path = dir.path().join(&id_str[0..2]).join(format!("{}.json", id_str));
        assert!(expected_path.exists());

        Ok(())
    }
}

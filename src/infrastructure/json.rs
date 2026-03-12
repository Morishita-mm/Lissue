use crate::domain::task::Task;
use anyhow::{Context, Result};
use std::fs::File;
use std::path::Path;

pub struct JsonRepository {
    path: std::path::PathBuf,
}

impl JsonRepository {
    pub fn new<P: AsRef<Path>>(path: P) -> Self {
        Self {
            path: path.as_ref().to_path_buf(),
        }
    }

    pub fn save_all(&self, tasks: &[Task]) -> Result<()> {
        let file = File::create(&self.path)
            .with_context(|| format!("Failed to create JSON file: {:?}", self.path))?;
        serde_json::to_writer_pretty(file, tasks)
            .with_context(|| "Failed to write tasks to JSON")?;
        Ok(())
    }

    pub fn load_all(&self) -> Result<Vec<Task>> {
        if !self.path.exists() {
            return Ok(Vec::new());
        }
        let file = File::open(&self.path)
            .with_context(|| format!("Failed to open JSON file: {:?}", self.path))?;
        let tasks =
            serde_json::from_reader(file).with_context(|| "Failed to parse tasks from JSON")?;
        Ok(tasks)
    }
}

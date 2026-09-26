use crate::domain::task::Task;
use anyhow::{Context, Result, anyhow};
use std::fs::{self, File};
use std::path::{Path, PathBuf};
use tempfile::NamedTempFile;

pub struct JsonRepository {
    base_path: PathBuf,
    root_path: PathBuf,
}

impl JsonRepository {
    /// Construct a repository whose storage is confined to `path`.
    ///
    /// This constructor is kept for callers that own the directory directly (for
    /// example, the repository unit tests). Project-backed callers should use
    /// [`Self::new_with_root`] so that `.lissue/tasks` is checked against the
    /// project root rather than treated as the root itself.
    pub fn new<P: AsRef<Path>>(path: P) -> Self {
        let path = path.as_ref().to_path_buf();
        Self {
            base_path: path.clone(),
            root_path: path,
        }
    }

    pub fn new_with_root<P: AsRef<Path>, R: AsRef<Path>>(path: P, root: R) -> Result<Self> {
        let root_path = fs::canonicalize(root.as_ref()).with_context(|| {
            format!(
                "Failed to canonicalize JSON repository root: {:?}",
                root.as_ref()
            )
        })?;
        let repository = Self {
            base_path: path.as_ref().to_path_buf(),
            root_path,
        };
        repository.validate_path(&repository.base_path)?;
        Ok(repository)
    }

    fn get_task_path(&self, global_id: &uuid::Uuid) -> PathBuf {
        let id_str = global_id.to_string();
        let prefix = &id_str[0..2];
        self.base_path.join(prefix).join(format!("{}.json", id_str))
    }

    /// Ensure that a path and all existing symlink targets resolve below the
    /// configured root. The nearest existing ancestor is checked so this also
    /// protects paths that will be created later.
    fn validate_path(&self, path: &Path) -> Result<()> {
        let root = fs::canonicalize(&self.root_path).with_context(|| {
            format!(
                "Failed to canonicalize JSON repository root: {:?}",
                self.root_path
            )
        })?;

        let mut existing = path.to_path_buf();
        loop {
            match fs::symlink_metadata(&existing) {
                Ok(_) => break,
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                    if !existing.pop() {
                        return Err(anyhow!(
                            "Path does not have an existing ancestor: {:?}",
                            path
                        ));
                    }
                }
                Err(error) => {
                    return Err(error).with_context(|| {
                        format!("Failed to inspect JSON repository path: {:?}", existing)
                    });
                }
            }
        }

        let resolved = fs::canonicalize(&existing)
            .with_context(|| format!("Failed to resolve JSON repository path: {:?}", existing))?;
        if !resolved.starts_with(&root) {
            return Err(anyhow!(
                "JSON repository path resolves outside project root: {:?}",
                path
            ));
        }
        Ok(())
    }

    pub fn validate_task_path(&self, task: &Task) -> Result<()> {
        self.validate_path(&self.base_path)?;
        self.validate_path(&self.get_task_path(&task.global_id))
    }

    pub fn save_task(&self, task: &Task) -> Result<()> {
        self.validate_task_path(task)?;
        if let Some(parent) = self.get_task_path(&task.global_id).parent() {
            fs::create_dir_all(parent)?;
        }

        let path = self.get_task_path(&task.global_id);
        self.validate_task_path(task)?;
        let target = if path.exists() {
            fs::canonicalize(&path)
                .with_context(|| format!("Failed to resolve JSON file: {:?}", path))?
        } else {
            path.clone()
        };
        self.validate_path(&target)?;
        let parent = target
            .parent()
            .ok_or_else(|| anyhow!("JSON file has no parent directory: {:?}", target))?;
        let mut temporary = NamedTempFile::new_in(parent)
            .with_context(|| format!("Failed to create temporary JSON file in {:?}", parent))?;
        self.validate_path(temporary.path())?;
        serde_json::to_writer_pretty(temporary.as_file_mut(), task)
            .with_context(|| "Failed to write task to JSON")?;
        temporary
            .as_file()
            .sync_all()
            .with_context(|| "Failed to flush task JSON")?;
        temporary
            .persist(&target)
            .map_err(|error| error.error)
            .with_context(|| format!("Failed to replace JSON file: {:?}", target))?;
        Ok(())
    }

    pub fn save_all(&self, tasks: &[Task]) -> Result<()> {
        for task in tasks {
            self.save_task(task)?;
        }
        Ok(())
    }

    pub fn load_all(&self) -> Result<Vec<Task>> {
        self.validate_path(&self.base_path)?;
        let mut tasks = Vec::new();
        if !self.base_path.exists() {
            return Ok(tasks);
        }

        let walk_root = fs::canonicalize(&self.base_path).with_context(|| {
            format!(
                "Failed to resolve JSON tasks directory: {:?}",
                self.base_path
            )
        })?;
        for entry in walkdir::WalkDir::new(walk_root) {
            let entry = entry?;
            self.validate_path(entry.path())?;
            if entry.file_type().is_file()
                && entry.path().extension().is_some_and(|ext| ext == "json")
            {
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
        self.validate_path(&self.base_path)?;
        let path = self.get_task_path(global_id);
        self.validate_path(&path)?;
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
        let expected_path = dir
            .path()
            .join(&id_str[0..2])
            .join(format!("{}.json", id_str));
        assert!(expected_path.exists());

        Ok(())
    }

    #[cfg(unix)]
    #[test]
    fn test_load_all_follows_in_root_tasks_symlink() -> Result<()> {
        use std::os::unix::fs::symlink;

        let root = tempdir()?;
        let real_tasks = root.path().join("real-tasks");
        let linked_tasks = root.path().join("tasks");
        fs::create_dir(&real_tasks)?;
        symlink(&real_tasks, &linked_tasks)?;

        let repo = JsonRepository::new_with_root(&linked_tasks, root.path())?;
        let task = Task::new("Through symlink".to_string(), None, None);
        repo.save_task(&task)?;

        let loaded = repo.load_all()?;
        assert_eq!(loaded.len(), 1);
        assert_eq!(loaded[0].title, "Through symlink");
        Ok(())
    }

    #[cfg(unix)]
    #[test]
    fn test_save_rejects_external_task_file_symlink() -> Result<()> {
        use std::os::unix::fs::symlink;

        let root = tempdir()?;
        let outside = tempdir()?;
        let tasks_dir = root.path().join(".lissue/tasks");
        let task = Task::new("Protected".to_string(), None, None);
        let id = task.global_id.to_string();
        let task_path = tasks_dir.join(&id[0..2]).join(format!("{id}.json"));
        fs::create_dir_all(task_path.parent().unwrap())?;
        let external_path = outside.path().join("external.json");
        fs::write(&external_path, "sentinel")?;
        symlink(&external_path, &task_path)?;

        let repo = JsonRepository::new_with_root(&tasks_dir, root.path())?;
        assert!(repo.save_task(&task).is_err());
        assert_eq!(fs::read_to_string(external_path)?, "sentinel");

        Ok(())
    }
}

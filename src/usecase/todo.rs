use crate::domain::config::Config;
use crate::domain::repository::TaskRepository;
use crate::domain::task::{Status, Task};
use crate::infrastructure::config::YamlConfigRepository;
use crate::infrastructure::json::JsonRepository;
use crate::infrastructure::sqlite::SqliteRepository;
use anyhow::{Context, Result, anyhow};
use chrono::Utc;
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Default)]
pub struct TaskFilter {
    pub status: Option<Status>,
    pub unassigned: bool,
}

pub struct ProjectPaths {
    pub root: PathBuf,
}

impl ProjectPaths {
    pub fn new(root: PathBuf) -> Self {
        Self { root }
    }

    pub fn dot_lissue(&self) -> PathBuf {
        self.root.join(".lissue")
    }

    pub fn db(&self) -> PathBuf {
        self.dot_lissue().join("data.db")
    }

    pub fn tasks_dir(&self) -> PathBuf {
        self.dot_lissue().join("tasks")
    }

    pub fn config(&self) -> PathBuf {
        self.dot_lissue().join("config.yaml")
    }

    /// Check a path using filesystem resolution, not just lexical components.
    /// The nearest existing ancestor is resolved so a path that will be created
    /// cannot escape through a symlinked parent directory.
    fn validate_path(&self, path: &Path) -> Result<()> {
        let root = fs::canonicalize(&self.root)
            .with_context(|| format!("Failed to canonicalize project root: {:?}", self.root))?;

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
                        format!("Failed to inspect project path: {:?}", existing)
                    });
                }
            }
        }

        let resolved = fs::canonicalize(&existing)
            .with_context(|| format!("Failed to resolve project path: {:?}", existing))?;
        if !resolved.starts_with(&root) {
            return Err(anyhow!("Path resolves outside of project root: {:?}", path));
        }
        Ok(())
    }

    pub fn validate_within_root(&self, path: &str) -> Result<PathBuf> {
        let path = Path::new(path);
        let full_path = if path.is_absolute() {
            path.to_path_buf()
        } else {
            self.root.join(path)
        };
        self.validate_path(&full_path)?;
        Ok(full_path)
    }

    fn validate_storage_paths(&self) -> Result<()> {
        self.validate_path(&self.dot_lissue())?;
        self.validate_path(&self.tasks_dir())?;
        self.validate_path(&self.db())?;
        self.validate_path(&self.config())?;

        if self.dot_lissue().exists() && !self.dot_lissue().is_dir() {
            return Err(anyhow!(".lissue is not a directory"));
        }
        if self.tasks_dir().exists() && !self.tasks_dir().is_dir() {
            return Err(anyhow!(".lissue/tasks is not a directory"));
        }
        Ok(())
    }
}

pub struct TodoUsecase {
    repo: SqliteRepository,
    json_repo: JsonRepository,
    config_repo: YamlConfigRepository,
    paths: ProjectPaths,
}

impl TodoUsecase {
    pub fn new(start_dir: PathBuf) -> Result<Self> {
        let root_dir = Self::find_root(start_dir).ok_or_else(|| {
            anyhow!("Not initialized. Run 'lissue init' first in a project root.")
        })?;

        let canonical_root = root_dir
            .canonicalize()
            .with_context(|| format!("Failed to canonicalize project root: {:?}", root_dir))?;

        let paths = ProjectPaths::new(canonical_root);
        paths.validate_storage_paths()?;

        let repo = SqliteRepository::new(paths.db())?;
        let json_repo = JsonRepository::new_with_root(paths.tasks_dir(), &paths.root)?;
        let config_repo = YamlConfigRepository::new(paths.config());

        Ok(Self {
            repo,
            json_repo,
            config_repo,
            paths,
        })
    }

    pub fn find_root(start_dir: PathBuf) -> Option<PathBuf> {
        let mut current = start_dir;
        loop {
            if current.join(".lissue").is_dir() {
                return Some(current);
            }
            if !current.pop() {
                break;
            }
        }
        None
    }

    pub fn init(root_dir: PathBuf) -> Result<()> {
        let canonical_root = root_dir
            .canonicalize()
            .with_context(|| format!("Failed to canonicalize project root: {:?}", root_dir))?;
        let paths = ProjectPaths::new(canonical_root.clone());
        paths.validate_storage_paths()?;
        let dot_lissue = paths.dot_lissue();

        if !dot_lissue.exists() {
            fs::create_dir(&dot_lissue)?;
        } else if !dot_lissue.is_dir() {
            return Err(anyhow!(".lissue is not a directory"));
        }

        let tasks_dir = paths.tasks_dir();
        if !tasks_dir.exists() {
            fs::create_dir(&tasks_dir)?;
        } else if !tasks_dir.is_dir() {
            return Err(anyhow!(".lissue/tasks is not a directory"));
        }

        paths.validate_storage_paths()?;
        let gitattributes_path = tasks_dir.join(".gitattributes");
        paths.validate_path(&gitattributes_path)?;
        if !gitattributes_path.exists() {
            fs::write(&gitattributes_path, "**/*.json linguist-generated=true\n")?;
        }

        let gitignore_path = canonical_root.join(".gitignore");
        paths.validate_path(&gitignore_path)?;
        let mut content = if gitignore_path.exists() {
            fs::read_to_string(&gitignore_path)?
        } else {
            String::new()
        };

        if !content.contains(".lissue/data.db") {
            if !content.is_empty() && !content.ends_with('\n') {
                content.push('\n');
            }
            content.push_str(".lissue/data.db\n");
            fs::write(gitignore_path, content)?;
        }

        let config_repo = YamlConfigRepository::new(paths.config());
        if !paths.config().exists() {
            config_repo.save(&Config::default())?;
        }

        let _ = SqliteRepository::new(paths.db())?;

        Ok(())
    }

    pub fn add_task(
        &self,
        title: String,
        description: Option<String>,
        parent_local_id: Option<i32>,
    ) -> Result<Task> {
        let parent_global_id = if let Some(pid) = parent_local_id {
            let parent = self
                .repo
                .find_by_local_id(pid)?
                .ok_or_else(|| anyhow!("Parent task with local_id {} not found", pid))?;
            Some(parent.global_id)
        } else {
            None
        };

        let task = Task::new(title, description, parent_global_id);
        self.repo.save(&task)?;
        self.json_repo.save_task(&task)?;

        let saved_task = self
            .repo
            .find_by_global_id(task.global_id)?
            .ok_or_else(|| anyhow!("Failed to retrieve saved task"))?;
        Ok(saved_task)
    }

    /// Validate every attachment before any task persistence occurs.
    pub fn validate_file_paths(&self, file_paths: &[String]) -> Result<()> {
        for path in file_paths {
            let full_path = self.paths.validate_within_root(path)?;
            if !full_path.is_file() {
                return Err(anyhow!("File does not exist: {}", path));
            }
        }
        Ok(())
    }

    fn validate_linked_paths(&self, task: &Task) -> Result<()> {
        for path in &task.linked_files {
            self.paths.validate_within_root(path)?;
        }
        Ok(())
    }

    pub fn list_tasks(&self, filter: TaskFilter) -> Result<Vec<Task>> {
        let tasks = self.repo.find_all()?;
        let filtered = tasks
            .into_iter()
            .filter(|t| {
                if filter.status.is_some_and(|s| t.status != s) {
                    return false;
                }
                if filter.unassigned && t.assignee.is_some() {
                    return false;
                }
                true
            })
            .collect();
        Ok(filtered)
    }

    pub fn update_status(&self, local_id: i32, status: Status) -> Result<()> {
        let mut task = self
            .repo
            .find_by_local_id(local_id)?
            .ok_or_else(|| anyhow!("Task not found: {}", local_id))?;

        task.status = status;
        task.updated_at = Utc::now();
        self.repo.save(&task)?;
        self.json_repo.save_task(&task)
    }

    pub fn sync(&self) -> Result<()> {
        let json_tasks = self.json_repo.load_all()?;
        for mut json_task in json_tasks {
            let local_task = self.repo.find_by_global_id(json_task.global_id)?;

            match local_task {
                Some(lt) => {
                    if json_task.updated_at > lt.updated_at {
                        self.validate_linked_paths(&json_task)?;
                        json_task.local_id = lt.local_id;
                        self.repo.save(&json_task)?;
                    }
                }
                None => {
                    self.validate_linked_paths(&json_task)?;
                    json_task.local_id = None;
                    self.repo.save(&json_task)?;
                }
            }
        }
        self.sync_to_json()
    }

    fn sync_to_json(&self) -> Result<()> {
        let all_tasks = self.repo.find_all()?;
        for task in &all_tasks {
            self.validate_linked_paths(task)?;
        }
        self.json_repo.save_all(&all_tasks)
    }

    pub fn link_tasks(&self, child_id: i32, parent_id: i32) -> Result<()> {
        let parent = self
            .repo
            .find_by_local_id(parent_id)?
            .ok_or_else(|| anyhow!("Parent task not found: {}", parent_id))?;
        let mut child = self
            .repo
            .find_by_local_id(child_id)?
            .ok_or_else(|| anyhow!("Child task not found: {}", child_id))?;

        child.parent_global_id = Some(parent.global_id);
        child.updated_at = Utc::now();
        self.repo.save(&child)?;
        self.json_repo.save_task(&child)
    }

    pub fn unlink_task(&self, id: i32) -> Result<()> {
        let mut task = self
            .repo
            .find_by_local_id(id)?
            .ok_or_else(|| anyhow!("Task not found: {}", id))?;

        task.parent_global_id = None;
        task.updated_at = Utc::now();
        self.repo.save(&task)?;
        self.json_repo.save_task(&task)
    }

    pub fn claim_task(&self, id: i32, assignee: Option<String>) -> Result<()> {
        let mut task = self
            .repo
            .find_by_local_id(id)?
            .ok_or_else(|| anyhow!("Task not found: {}", id))?;

        task.status = Status::InProgress;
        task.assignee = assignee;
        task.updated_at = Utc::now();
        self.repo.save(&task)?;
        self.json_repo.save_task(&task)
    }

    pub fn get_task_context(&self, id: i32) -> Result<(Task, String)> {
        let task = self
            .repo
            .find_by_local_id(id)?
            .ok_or_else(|| anyhow!("Task not found: {}", id))?;

        let config = self.get_config()?;
        let mut context = String::new();

        context.push_str(&format!("Title: {}\n", task.title));
        if let Some(desc) = &task.description {
            context.push_str(&format!("Description: {}\n", desc));
        }
        context.push_str(&format!("Status: {}\n", task.status));
        if let Some(assignee) = &task.assignee {
            context.push_str(&format!("Assignee: {}\n", assignee));
        }
        context.push_str("\nLinked Files:\n");

        for file_path in &task.linked_files {
            context.push_str(&format!("- {}\n", file_path));
            if let Some(full_path) = Some(file_path)
                .filter(|_| config.context.strategy == "raw_content")
                .and_then(|p| self.paths.validate_within_root(p).ok())
                .filter(|p| p.is_file())
            {
                let content = fs::read_to_string(full_path)?;
                context.push_str("```\n");
                context.push_str(&content);
                context.push_str("\n```\n");
            }
        }

        Ok((task, context))
    }

    pub fn get_config(&self) -> Result<Config> {
        self.config_repo.load()
    }

    pub fn get_next_task(&self) -> Result<Option<Task>> {
        let config = self.get_config()?;
        if config.output.auto_sync {
            let _ = self.sync();
        }

        let tasks = self.list_tasks(TaskFilter {
            status: Some(Status::Open),
            unassigned: true,
        })?;

        Ok(tasks.into_iter().next())
    }

    pub fn save_task(&self, task: &Task) -> Result<()> {
        self.validate_linked_paths(task)?;
        self.repo.save(task)?;
        self.json_repo.save_task(task)
    }

    pub fn delete_task(&self, id: i32) -> Result<()> {
        if let Some(task) = self.repo.find_by_local_id(id)? {
            self.repo.delete(id)?;
            self.json_repo.delete_task(&task.global_id)?;
        }
        Ok(())
    }

    pub fn clear_closed_tasks(&self) -> Result<usize> {
        let tasks = self.repo.find_all()?;
        let mut count = 0;
        for task in tasks {
            if let (Status::Close, Some(local_id)) = (task.status, task.local_id) {
                self.delete_task(local_id)?;
                count += 1;
            }
        }
        Ok(count)
    }

    pub fn attach_files(&self, local_id: i32, file_paths: Vec<String>) -> Result<()> {
        let mut task = self
            .repo
            .find_by_local_id(local_id)?
            .ok_or_else(|| anyhow!("Task not found: {}", local_id))?;

        self.validate_file_paths(&file_paths)?;
        for path in file_paths {
            if !task.linked_files.contains(&path) {
                task.linked_files.push(path);
            }
        }

        task.updated_at = Utc::now();
        self.repo.save(&task)?;
        self.json_repo.save_task(&task)
    }

    pub fn detach_file(&self, local_id: i32, file_path: &str) -> Result<()> {
        let mut task = self
            .repo
            .find_by_local_id(local_id)?
            .ok_or_else(|| anyhow!("Task not found: {}", local_id))?;

        task.linked_files.retain(|f| f != file_path);
        task.updated_at = Utc::now();
        self.repo.save(&task)?;
        self.json_repo.save_task(&task)
    }

    pub fn list_project_files(&self) -> Result<Vec<String>> {
        let mut files = Vec::new();
        for entry in walkdir::WalkDir::new(&self.paths.root)
            .into_iter()
            .filter_entry(|e| {
                let name = e.file_name().to_string_lossy();
                name != ".git" && name != ".lissue" && name != "target"
            })
        {
            let entry = entry?;
            if entry.file_type().is_file()
                && let Ok(path) = entry.path().strip_prefix(&self.paths.root)
            {
                let path_str = path.to_string_lossy().to_string();
                if !path_str.is_empty() {
                    files.push(path_str);
                }
            }
        }
        files.sort();
        Ok(files)
    }

    fn persist_move_metadata(&self, changes: &[(Task, Task)]) -> Result<()> {
        for (_, updated) in changes {
            self.repo.save(updated)?;
        }
        for (_, updated) in changes {
            self.json_repo.save_task(updated)?;
        }
        Ok(())
    }

    fn restore_move_metadata(&self, changes: &[(Task, Task)]) -> Result<()> {
        let mut errors = Vec::new();
        for (original, _) in changes {
            if let Err(error) = self.repo.save(original) {
                errors.push(format!("database restore failed: {error}"));
            }
        }
        for (original, _) in changes {
            if let Err(error) = self.json_repo.save_task(original) {
                errors.push(format!("JSON restore failed: {error}"));
            }
        }
        if errors.is_empty() {
            Ok(())
        } else {
            Err(anyhow!("{}", errors.join("; ")))
        }
    }

    fn rollback_file_move(
        &self,
        old_full: &Path,
        new_full: &Path,
        old_path: &str,
        new_path: &str,
        moved_by_git: bool,
    ) -> Result<()> {
        let mut git_rollback_error = None;
        if moved_by_git {
            match std::process::Command::new("git")
                .arg("mv")
                .arg(new_path)
                .arg(old_path)
                .current_dir(&self.paths.root)
                .status()
            {
                Ok(status) if status.success() => {
                    if old_full.is_file() && !new_full.exists() {
                        return Ok(());
                    }
                    git_rollback_error = Some(
                        "git mv rollback completed without restoring the worktree".to_string(),
                    );
                }
                Ok(status) => {
                    git_rollback_error =
                        Some(format!("git mv rollback exited unsuccessfully: {}", status));
                }
                Err(error) => {
                    git_rollback_error = Some(format!("could not run git mv rollback: {error}"));
                }
            }

            // A failed reverse git operation may still have restored the
            // worktree while leaving the index uncertain. Never call that a
            // clean rollback; report the Git failure to the caller.
            if old_full.is_file() && !new_full.exists() {
                return Err(anyhow!(
                    "{}; worktree restored but Git index state is uncertain",
                    git_rollback_error
                        .as_deref()
                        .unwrap_or("git mv rollback failed")
                ));
            }
        }

        if old_full.exists() {
            return Err(anyhow!(
                "Cannot roll back file move because source exists: {}{}",
                old_path,
                git_rollback_error
                    .as_deref()
                    .map(|error| format!("; {error}"))
                    .unwrap_or_default()
            ));
        }
        if !new_full.is_file() {
            return Err(anyhow!(
                "Cannot roll back file move because destination is missing: {}{}",
                new_path,
                git_rollback_error
                    .as_deref()
                    .map(|error| format!("; {error}"))
                    .unwrap_or_default()
            ));
        }
        fs::rename(new_full, old_full).with_context(|| {
            format!(
                "Failed to roll back file move from {} to {}",
                new_path, old_path
            )
        })?;
        if let Some(error) = git_rollback_error {
            return Err(anyhow!(
                "{error}; worktree restored with filesystem rename but Git index state is uncertain"
            ));
        }
        Ok(())
    }

    pub fn move_file(&self, old_path: &str, new_path: &str) -> Result<()> {
        let old_full = self.paths.validate_within_root(old_path)?;
        let new_full = self.paths.validate_within_root(new_path)?;

        if !old_full.is_file() {
            return Err(anyhow!("Source file does not exist: {}", old_path));
        }
        if fs::symlink_metadata(&new_full).is_ok() {
            return Err(anyhow!("Destination already exists: {}", new_path));
        }
        let new_parent = new_full
            .parent()
            .ok_or_else(|| anyhow!("Destination has no parent directory: {}", new_path))?;
        if !new_parent.is_dir() {
            return Err(anyhow!("Destination parent does not exist: {}", new_path));
        }

        let config = self.get_config()?;
        // Read and prepare every metadata change before touching the filesystem.
        // This makes database/JSON failures happen before a move whenever possible.
        let all_tasks = self.repo.find_all()?;
        let mut changes = Vec::new();
        for task in all_tasks {
            let mut updated = task.clone();
            let mut file_updated = false;
            for file in &mut updated.linked_files {
                if file == old_path {
                    *file = new_path.to_string();
                    file_updated = true;
                }
            }
            if file_updated {
                updated.updated_at = Utc::now();
                self.validate_linked_paths(&updated)?;
                self.json_repo.validate_task_path(&updated)?;
                changes.push((task, updated));
            }
        }

        let moved_by_git = if config.integration.git_mv_hook {
            std::process::Command::new("git")
                .arg("mv")
                .arg(old_path)
                .arg(new_path)
                .current_dir(&self.paths.root)
                .status()
                .is_ok_and(|status| status.success())
        } else {
            false
        };

        if !moved_by_git {
            // A failed hook must not be allowed to leave us guessing about the
            // filesystem state before falling back to the normal rename.
            if !old_full.exists() && new_full.exists() {
                let rollback =
                    self.rollback_file_move(&old_full, &new_full, old_path, new_path, true);
                return match rollback {
                    Ok(()) => Err(anyhow!(
                        "git mv failed after changing the filesystem: {} -> {}; rollback succeeded",
                        old_path,
                        new_path
                    )),
                    Err(error) => Err(anyhow!(
                        "git mv failed after changing the filesystem: {} -> {}; rollback failed: {error}",
                        old_path,
                        new_path
                    )),
                };
            }
            fs::rename(&old_full, &new_full).with_context(|| {
                format!("Failed to move file from {} to {}", old_path, new_path)
            })?;
        }

        if old_full.exists() || !new_full.is_file() {
            let rollback =
                self.rollback_file_move(&old_full, &new_full, old_path, new_path, moved_by_git);
            return match rollback {
                Ok(()) => Err(anyhow!(
                    "File move did not complete: {} -> {}",
                    old_path,
                    new_path
                )),
                Err(error) => Err(anyhow!(
                    "File move did not complete: {} -> {}; rollback failed: {error}",
                    old_path,
                    new_path
                )),
            };
        }

        if let Err(error) = self.persist_move_metadata(&changes) {
            let mut rollback_errors = Vec::new();
            if let Err(rollback_error) = self.restore_move_metadata(&changes) {
                rollback_errors.push(rollback_error.to_string());
            }
            if let Err(rollback_error) =
                self.rollback_file_move(&old_full, &new_full, old_path, new_path, moved_by_git)
            {
                rollback_errors.push(rollback_error.to_string());
            }
            if rollback_errors.is_empty() {
                return Err(anyhow!("Failed to persist moved task metadata: {error}"));
            }
            return Err(anyhow!(
                "Failed to persist moved task metadata: {error}; rollback failed: {}",
                rollback_errors.join("; ")
            ));
        }

        Ok(())
    }

    pub fn parse_editor_content(content: &str) -> (String, Option<String>) {
        let lines: Vec<&str> = content.lines().collect();
        if lines.is_empty() {
            return (String::new(), None);
        }
        let title = lines[0].trim().to_string();
        let description = if lines.len() > 1 {
            let desc = lines[1..].join("\n").trim().to_string();
            if desc.is_empty() { None } else { Some(desc) }
        } else {
            None
        };
        (title, description)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{Duration, Utc};
    use tempfile::tempdir;

    #[test]
    fn test_find_root() -> Result<()> {
        let dir = tempdir()?;
        let root = dir.path().to_path_buf();
        let sub = root.join("a/b/c");
        fs::create_dir_all(&sub)?;

        TodoUsecase::init(root.clone())?;

        let found = TodoUsecase::find_root(sub).expect("Should find root");
        assert_eq!(found.canonicalize()?, root.canonicalize()?);

        Ok(())
    }

    #[test]
    fn test_operation_from_subdir() -> Result<()> {
        let dir = tempdir()?;
        let root = dir.path().to_path_buf();
        let sub = root.join("subdir");
        fs::create_dir_all(&sub)?;

        TodoUsecase::init(root.clone())?;

        // Root で追加
        let usecase_root = TodoUsecase::new(root)?;
        usecase_root.add_task("Root Task".to_string(), None, None)?;

        // Subdir で一覧取得
        let usecase_sub = TodoUsecase::new(sub)?;
        let tasks = usecase_sub.list_tasks(TaskFilter::default())?;

        assert_eq!(tasks.len(), 1);
        assert_eq!(tasks[0].title, "Root Task");
        assert_eq!(
            usecase_sub.paths.root.canonicalize()?,
            usecase_root.paths.root.canonicalize()?
        );

        Ok(())
    }

    #[test]
    fn test_init() -> Result<()> {
        let dir = tempdir()?;
        let root = dir.path().to_path_buf();

        TodoUsecase::init(root.clone())?;

        assert!(root.join(".lissue").exists());
        assert!(root.join(".lissue/data.db").exists());
        assert!(root.join(".lissue/config.yaml").exists());
        assert!(root.join(".lissue/tasks/.gitattributes").exists());
        assert!(root.join(".gitignore").exists());

        let gitignore = fs::read_to_string(root.join(".gitignore"))?;
        assert!(gitignore.contains(".lissue/data.db"));

        Ok(())
    }

    #[test]
    fn test_add_and_list() -> Result<()> {
        let dir = tempdir()?;
        let root = dir.path().to_path_buf();
        TodoUsecase::init(root.clone())?;

        let usecase = TodoUsecase::new(root)?;
        usecase.add_task("Test 1".to_string(), None, None)?;
        usecase.add_task("Test 2".to_string(), Some("Desc".to_string()), None)?;

        let tasks = usecase.list_tasks(TaskFilter::default())?;
        assert_eq!(tasks.len(), 2);
        assert_eq!(tasks[0].title, "Test 1");
        assert_eq!(tasks[1].description, Some("Desc".to_string()));

        Ok(())
    }

    #[test]
    fn test_sync_lww() -> Result<()> {
        let dir = tempdir()?;
        let root = dir.path().to_path_buf();
        TodoUsecase::init(root.clone())?;
        let usecase = TodoUsecase::new(root.clone())?;

        let _task = usecase.add_task("Base Task".to_string(), None, None)?;

        let mut tasks_in_json = usecase.json_repo.load_all()?;
        let original_updated_at = tasks_in_json[0].updated_at;
        tasks_in_json[0].title = "Old JSON".to_string();
        tasks_in_json[0].updated_at = original_updated_at - Duration::minutes(10);
        usecase.json_repo.save_all(&tasks_in_json)?;

        usecase.sync()?;
        let tasks = usecase.list_tasks(TaskFilter::default())?;
        assert_eq!(tasks[0].title, "Base Task");

        let mut tasks_in_json = usecase.json_repo.load_all()?;
        tasks_in_json[0].title = "New JSON".to_string();
        tasks_in_json[0].updated_at = Utc::now() + Duration::minutes(10);
        usecase.json_repo.save_all(&tasks_in_json)?;

        usecase.sync()?;
        let tasks = usecase.list_tasks(TaskFilter::default())?;
        assert_eq!(tasks[0].title, "New JSON");

        Ok(())
    }

    #[test]
    fn test_assignee_preservation() -> Result<()> {
        let dir = tempdir()?;
        let root = dir.path().to_path_buf();
        TodoUsecase::init(root.clone())?;
        let usecase = TodoUsecase::new(root)?;

        let mut task = usecase.add_task("Assignee Test".to_string(), None, None)?;
        task.assignee = Some("AI-Agent".to_string());
        usecase.save_task(&task)?;

        let retrieved = usecase.repo.find_by_local_id(1)?.unwrap();
        assert_eq!(retrieved.assignee, Some("AI-Agent".to_string()));

        Ok(())
    }

    #[test]
    fn test_claim_task() -> Result<()> {
        let dir = tempdir()?;
        let root = dir.path().to_path_buf();
        TodoUsecase::init(root.clone())?;
        let usecase = TodoUsecase::new(root)?;

        usecase.add_task("Claim Test".to_string(), None, None)?;
        usecase.claim_task(1, Some("Agent-1".to_string()))?;

        let task = usecase.repo.find_by_local_id(1)?.unwrap();
        assert_eq!(task.status, Status::InProgress);
        assert_eq!(task.assignee, Some("Agent-1".to_string()));

        Ok(())
    }

    #[test]
    fn test_get_task_context() -> Result<()> {
        let dir = tempdir()?;
        let root = dir.path().to_path_buf();
        TodoUsecase::init(root.clone())?;
        let usecase = TodoUsecase::new(root.clone())?;

        let file_path = "test_file.txt";
        fs::write(root.join(file_path), "File Content")?;

        let mut task =
            usecase.add_task("Context Test".to_string(), Some("Desc".to_string()), None)?;
        task.linked_files.push(file_path.to_string());
        usecase.save_task(&task)?;

        let (_, context) = usecase.get_task_context(1)?;
        assert!(context.contains("Context Test"));
        assert!(context.contains("Desc"));
        assert!(context.contains("test_file.txt"));

        Ok(())
    }

    #[test]
    fn test_attach_files() -> Result<()> {
        let dir = tempdir()?;
        let root = dir.path().to_path_buf();
        TodoUsecase::init(root.clone())?;
        let usecase = TodoUsecase::new(root.clone())?;

        let file1 = "file1.txt";
        let file2 = "file2.txt";
        fs::write(root.join(file1), "content")?;
        fs::write(root.join(file2), "content")?;

        usecase.add_task("Attach Test".to_string(), None, None)?;

        // Success
        usecase.attach_files(1, vec![file1.to_string(), file2.to_string()])?;
        let task = usecase.repo.find_by_local_id(1)?.unwrap();
        assert_eq!(task.linked_files.len(), 2);
        assert!(task.linked_files.contains(&file1.to_string()));

        // Duplicate prevention
        usecase.attach_files(1, vec![file1.to_string()])?;
        let task = usecase.repo.find_by_local_id(1)?.unwrap();
        assert_eq!(task.linked_files.len(), 2);

        // File not exist
        assert!(
            usecase
                .attach_files(1, vec!["not_exist.txt".to_string()])
                .is_err()
        );

        // Path traversal
        assert!(
            usecase
                .attach_files(1, vec!["../outside.txt".to_string()])
                .is_err()
        );

        Ok(())
    }

    #[test]
    fn test_list_project_files() -> Result<()> {
        let dir = tempdir()?;
        let root = dir.path().to_path_buf();
        TodoUsecase::init(root.clone())?;
        let usecase = TodoUsecase::new(root.clone())?;

        fs::write(root.join("file1.txt"), "content")?;
        fs::create_dir(root.join("subdir"))?;
        fs::write(root.join("subdir/file2.txt"), "content")?;
        fs::create_dir(root.join(".git"))?;
        fs::write(root.join(".git/config"), "content")?;

        let files = usecase.list_project_files()?;
        // file1.txt, subdir/file2.txt, AND .gitignore (created by init)
        assert_eq!(files.len(), 3);
        assert!(files.contains(&"file1.txt".to_string()));
        assert!(files.contains(&"subdir/file2.txt".to_string()));
        assert!(files.contains(&".gitignore".to_string()));
        assert!(!files.contains(&".git/config".to_string()));

        Ok(())
    }

    #[test]
    fn test_detach_file() -> Result<()> {
        let dir = tempdir()?;
        let root = dir.path().to_path_buf();
        TodoUsecase::init(root.clone())?;
        let usecase = TodoUsecase::new(root.clone())?;

        let file = "file.txt";
        fs::write(root.join(file), "content")?;

        usecase.add_task("Detach Test".to_string(), None, None)?;
        usecase.attach_files(1, vec![file.to_string()])?;

        let task = usecase.repo.find_by_local_id(1)?.unwrap();
        assert_eq!(task.linked_files.len(), 1);

        usecase.detach_file(1, file)?;
        let task = usecase.repo.find_by_local_id(1)?.unwrap();
        assert_eq!(task.linked_files.len(), 0);

        Ok(())
    }

    #[test]
    fn test_parse_editor_content() {
        let content = "Task Title\nTask Description line 1\nline 2";
        let (title, desc) = TodoUsecase::parse_editor_content(content);
        assert_eq!(title, "Task Title");
        assert_eq!(desc, Some("Task Description line 1\nline 2".to_string()));

        let content_only_title = "Only Title";
        let (title, desc) = TodoUsecase::parse_editor_content(content_only_title);
        assert_eq!(title, "Only Title");
        assert!(desc.is_none());
    }

    #[test]
    fn test_move_file_updates_db() -> Result<()> {
        let dir = tempdir()?;
        let root = dir.path().to_path_buf();
        TodoUsecase::init(root.clone())?;
        let usecase = TodoUsecase::new(root.clone())?;

        let old_path = "old_file.txt";
        let new_path = "new_file.txt";
        fs::write(root.join(old_path), "content")?;

        let mut task = usecase.add_task("Move Test".to_string(), None, None)?;
        task.linked_files.push(old_path.to_string());
        usecase.save_task(&task)?;

        usecase.move_file(old_path, new_path)?;

        let tasks = usecase.list_tasks(TaskFilter::default())?;
        assert_eq!(tasks[0].linked_files[0], new_path);
        assert!(root.join(new_path).exists());
        assert!(!root.join(old_path).exists());

        Ok(())
    }

    #[cfg(unix)]
    #[test]
    fn test_sync_follows_in_root_symlinked_tasks_directory() -> Result<()> {
        use std::os::unix::fs::symlink;

        let dir = tempdir()?;
        let root = dir.path().to_path_buf();
        TodoUsecase::init(root.clone())?;
        let real_tasks = root.join(".lissue/real-tasks");
        let linked_tasks = root.join(".lissue/tasks");
        fs::create_dir(&real_tasks)?;
        fs::remove_dir_all(&linked_tasks)?;
        symlink(&real_tasks, &linked_tasks)?;

        let usecase = TodoUsecase::new(root)?;
        let task = Task::new("Imported through symlink".to_string(), None, None);
        usecase.json_repo.save_task(&task)?;

        usecase.sync()?;
        let tasks = usecase.list_tasks(TaskFilter::default())?;
        assert_eq!(tasks.len(), 1);
        assert_eq!(tasks[0].title, "Imported through symlink");
        Ok(())
    }

    #[cfg(unix)]
    #[test]
    fn test_move_rolls_back_after_json_persistence_failure() -> Result<()> {
        use std::os::unix::fs::PermissionsExt;

        let dir = tempdir()?;
        let root = dir.path().to_path_buf();
        TodoUsecase::init(root.clone())?;
        let usecase = TodoUsecase::new(root.clone())?;
        let old_path = "old_file.txt";
        let new_path = "new_file.txt";
        fs::write(root.join(old_path), "content")?;

        let mut task = usecase.add_task("JSON rollback".to_string(), None, None)?;
        task.linked_files.push(old_path.to_string());
        usecase.save_task(&task)?;

        let id = task.global_id.to_string();
        let json_path = root
            .join(".lissue/tasks")
            .join(&id[0..2])
            .join(format!("{id}.json"));
        let json_parent = json_path.parent().unwrap();
        let original_mode = fs::metadata(json_parent)?.permissions().mode();
        fs::set_permissions(
            json_parent,
            fs::Permissions::from_mode(original_mode & !0o222),
        )?;

        let result = usecase.move_file(old_path, new_path);
        fs::set_permissions(json_parent, fs::Permissions::from_mode(original_mode))?;

        assert!(result.is_err());
        assert_eq!(fs::read_to_string(root.join(old_path))?, "content");
        assert!(!root.join(new_path).exists());
        let restored_task = usecase.repo.find_by_local_id(1)?.unwrap();
        assert_eq!(restored_task.linked_files, vec![old_path.to_string()]);
        let json = fs::read_to_string(json_path)?;
        assert!(json.contains("old_file.txt"));
        assert!(!json.contains("new_file.txt"));
        Ok(())
    }

    #[test]
    fn test_validate_path() -> Result<()> {
        let dir = tempdir()?;
        let root = dir.path().to_path_buf();
        TodoUsecase::init(root.clone())?;
        let usecase = TodoUsecase::new(root)?;

        // Project root
        assert!(usecase.paths.validate_within_root("valid.txt").is_ok());
        assert!(
            usecase
                .paths
                .validate_within_root("subdir/valid.txt")
                .is_ok()
        );

        // Path traversal
        assert!(
            usecase
                .paths
                .validate_within_root("../outside.txt")
                .is_err()
        );
        assert!(usecase.paths.validate_within_root("/etc/passwd").is_err());

        Ok(())
    }

    #[test]
    fn test_list_tasks_filtering() -> Result<()> {
        let dir = tempdir()?;
        let root = dir.path().to_path_buf();
        TodoUsecase::init(root.clone())?;
        let usecase = TodoUsecase::new(root)?;

        usecase.add_task("Task 1".to_string(), None, None)?; // Open, Unassigned
        let mut t2 = usecase.add_task("Task 2".to_string(), None, None)?;
        t2.status = Status::Close;
        usecase.save_task(&t2)?; // Close, Unassigned

        let mut t3 = usecase.add_task("Task 3".to_string(), None, None)?;
        t3.assignee = Some("Agent".to_string());
        usecase.save_task(&t3)?; // Open, Assigned

        // 1. Status Filter
        let open_tasks = usecase.list_tasks(TaskFilter {
            status: Some(Status::Open),
            ..Default::default()
        })?;
        assert_eq!(open_tasks.len(), 2);

        // 2. Unassigned Filter
        let unassigned_tasks = usecase.list_tasks(TaskFilter {
            unassigned: true,
            ..Default::default()
        })?;
        assert_eq!(unassigned_tasks.len(), 2);
        assert!(unassigned_tasks.iter().all(|t| t.assignee.is_none()));

        // 3. Combined Filter
        let next_tasks = usecase.list_tasks(TaskFilter {
            status: Some(Status::Open),
            unassigned: true,
        })?;
        assert_eq!(next_tasks.len(), 1);
        assert_eq!(next_tasks[0].title, "Task 1");

        Ok(())
    }

    #[test]
    fn test_get_next_task() -> Result<()> {
        let dir = tempdir()?;
        let root = dir.path().to_path_buf();
        TodoUsecase::init(root.clone())?;
        let usecase = TodoUsecase::new(root)?;

        // 最初はNone
        assert!(usecase.get_next_task()?.is_none());

        // タスク追加
        usecase.add_task("Next 1".to_string(), None, None)?;
        usecase.add_task("Next 2".to_string(), None, None)?;

        let next = usecase.get_next_task()?.unwrap();
        assert_eq!(next.title, "Next 1");

        // クレームすると次が返る
        usecase.claim_task(1, Some("Agent".to_string()))?;
        let next = usecase.get_next_task()?.unwrap();
        assert_eq!(next.title, "Next 2");

        Ok(())
    }

    #[test]
    fn test_delete_task() -> Result<()> {
        let dir = tempdir()?;
        let root = dir.path().to_path_buf();
        TodoUsecase::init(root.clone())?;
        let usecase = TodoUsecase::new(root)?;

        let task = usecase.add_task("To be deleted".to_string(), None, None)?;
        let global_id = task.global_id;

        usecase.delete_task(1)?;

        assert!(usecase.repo.find_by_local_id(1)?.is_none());
        let all_json = usecase.json_repo.load_all()?;
        assert!(!all_json.iter().any(|t| t.global_id == global_id));

        Ok(())
    }

    #[test]
    fn test_clear_closed_tasks() -> Result<()> {
        let dir = tempdir()?;
        let root = dir.path().to_path_buf();
        TodoUsecase::init(root.clone())?;
        let usecase = TodoUsecase::new(root)?;

        usecase.add_task("Task 1".to_string(), None, None)?; // Open
        usecase.add_task("Task 2".to_string(), None, None)?; // Open
        usecase.update_status(2, Status::Close)?; // Close Task 2

        let cleared_count = usecase.clear_closed_tasks()?;
        assert_eq!(cleared_count, 1);

        let tasks = usecase.list_tasks(TaskFilter::default())?;
        assert_eq!(tasks.len(), 1);
        assert_eq!(tasks[0].title, "Task 1");

        Ok(())
    }

    #[cfg(unix)]
    #[test]
    fn test_init_rejects_external_lissue_symlink() -> Result<()> {
        use std::os::unix::fs::symlink;

        let root = tempdir()?;
        let outside = tempdir()?;
        let external_lissue = outside.path().join("external-lissue");
        fs::create_dir(&external_lissue)?;
        symlink(&external_lissue, root.path().join(".lissue"))?;

        assert!(TodoUsecase::init(root.path().to_path_buf()).is_err());
        assert!(!external_lissue.join("data.db").exists());
        Ok(())
    }

    #[cfg(unix)]
    #[test]
    fn test_new_rejects_external_tasks_symlink() -> Result<()> {
        use std::os::unix::fs::symlink;

        let root = tempdir()?;
        let outside = tempdir()?;
        TodoUsecase::init(root.path().to_path_buf())?;
        fs::remove_dir_all(root.path().join(".lissue/tasks"))?;
        symlink(outside.path(), root.path().join(".lissue/tasks"))?;

        assert!(TodoUsecase::new(root.path().to_path_buf()).is_err());
        assert!(fs::read_dir(outside.path())?.next().is_none());
        Ok(())
    }

    #[cfg(unix)]
    #[test]
    fn test_context_does_not_read_external_symlink() -> Result<()> {
        use std::os::unix::fs::symlink;

        let root = tempdir()?;
        let outside = tempdir()?;
        let secret = outside.path().join("secret.txt");
        fs::write(&secret, "secret-value")?;
        TodoUsecase::init(root.path().to_path_buf())?;
        let usecase = TodoUsecase::new(root.path().to_path_buf())?;
        symlink(&secret, root.path().join("linked.txt"))?;

        let mut config = usecase.get_config()?;
        config.context.strategy = "raw_content".to_string();
        usecase.config_repo.save(&config)?;
        let mut task = usecase.add_task("Safe context".to_string(), None, None)?;
        task.linked_files.push("linked.txt".to_string());
        // Bypass the public path validation to model pre-existing untrusted
        // metadata loaded from an older JSON/database file.
        usecase.repo.save(&task)?;

        let (_, context) = usecase.get_task_context(1)?;
        assert!(context.contains("- linked.txt"));
        assert!(!context.contains("secret-value"));
        Ok(())
    }

    #[cfg(unix)]
    #[test]
    fn test_save_task_rejects_external_linked_path() -> Result<()> {
        use std::os::unix::fs::symlink;

        let root = tempdir()?;
        let outside = tempdir()?;
        let secret = outside.path().join("secret.txt");
        fs::write(&secret, "secret")?;
        TodoUsecase::init(root.path().to_path_buf())?;
        let usecase = TodoUsecase::new(root.path().to_path_buf())?;
        symlink(&secret, root.path().join("linked.txt"))?;

        let mut task = usecase.add_task("Reject unsafe metadata".to_string(), None, None)?;
        task.linked_files.push("linked.txt".to_string());
        assert!(usecase.save_task(&task).is_err());
        assert!(
            usecase
                .repo
                .find_by_local_id(1)?
                .unwrap()
                .linked_files
                .is_empty()
        );
        Ok(())
    }
}

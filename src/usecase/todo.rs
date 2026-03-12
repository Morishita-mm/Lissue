use crate::domain::config::Config;
use crate::domain::repository::TaskRepository;
use crate::domain::task::{Status, Task};
use crate::infrastructure::config::YamlConfigRepository;
use crate::infrastructure::json::JsonRepository;
use crate::infrastructure::sqlite::SqliteRepository;
use anyhow::{Result, anyhow};
use chrono::Utc;
use std::fs;
use std::path::PathBuf;

pub struct TodoUsecase {
    repo: SqliteRepository,
    json_repo: JsonRepository,
    config_repo: YamlConfigRepository,
    root_dir: PathBuf,
}

impl TodoUsecase {
    pub fn new(root_dir: PathBuf) -> Result<Self> {
        let dot_mytodo = root_dir.join(".mytodo");
        let db_path = dot_mytodo.join("data.db");
        let json_path = dot_mytodo.join("tasks.json");
        let config_path = dot_mytodo.join("config.yaml");

        if !dot_mytodo.exists() {
            return Err(anyhow!("Not initialized. Run 'todo init' first."));
        }

        let repo = SqliteRepository::new(db_path)?;
        let json_repo = JsonRepository::new(json_path);
        let config_repo = YamlConfigRepository::new(config_path);

        Ok(Self {
            repo,
            json_repo,
            config_repo,
            root_dir,
        })
    }

    pub fn init(root_dir: PathBuf) -> Result<()> {
        let dot_mytodo = root_dir.join(".mytodo");
        if !dot_mytodo.exists() {
            fs::create_dir(&dot_mytodo)?;
        }

        // gitignore への追記
        let gitignore_path = root_dir.join(".gitignore");
        let mut content = if gitignore_path.exists() {
            fs::read_to_string(&gitignore_path)?
        } else {
            String::new()
        };

        if !content.contains(".mytodo/data.db") {
            if !content.is_empty() && !content.ends_with('\n') {
                content.push('\n');
            }
            content.push_str(".mytodo/data.db\n");
            fs::write(gitignore_path, content)?;
        }

        // 初期設定ファイルの作成
        let config_repo = YamlConfigRepository::new(dot_mytodo.join("config.yaml"));
        if !dot_mytodo.join("config.yaml").exists() {
            config_repo.save(&Config::default())?;
        }

        // DBの初期化（SqliteRepository::new 内で行われる）
        SqliteRepository::new(dot_mytodo.join("data.db"))?;

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
        self.sync_to_json()?;

        // local_id を取得するために再度検索（あるいは save で ID を返すようにリポジトリを修正するのもあり）
        let saved_task = self
            .repo
            .find_by_global_id(task.global_id)?
            .ok_or_else(|| anyhow!("Failed to retrieve saved task"))?;
        Ok(saved_task)
    }

    pub fn list_tasks(&self) -> Result<Vec<Task>> {
        self.repo.find_all()
    }

    pub fn update_status(&self, local_id: i32, status: Status) -> Result<()> {
        let mut task = self
            .repo
            .find_by_local_id(local_id)?
            .ok_or_else(|| anyhow!("Task not found: {}", local_id))?;

        task.status = status;
        task.updated_at = Utc::now();
        self.repo.save(&task)?;
        self.sync_to_json()
    }

    pub fn sync(&self) -> Result<()> {
        let json_tasks = self.json_repo.load_all()?;
        for mut json_task in json_tasks {
            let local_task = self.repo.find_by_global_id(json_task.global_id)?;

            match local_task {
                Some(lt) => {
                    // Last-Write-Wins
                    if json_task.updated_at > lt.updated_at {
                        // JSON側が新しいのでDBを更新
                        // local_id を維持する必要がある
                        json_task.local_id = lt.local_id;
                        self.repo.save(&json_task)?;
                    }
                }
                None => {
                    // DBに存在しないので追加
                    // 追加時は local_id を None にして AUTOINCREMENT に任せる
                    json_task.local_id = None;
                    self.repo.save(&json_task)?;
                }
            }
        }
        self.sync_to_json()
    }

    fn sync_to_json(&self) -> Result<()> {
        let all_tasks = self.repo.find_all()?;
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
        self.sync_to_json()
    }

    pub fn unlink_task(&self, id: i32) -> Result<()> {
        let mut task = self
            .repo
            .find_by_local_id(id)?
            .ok_or_else(|| anyhow!("Task not found: {}", id))?;

        task.parent_global_id = None;
        task.updated_at = Utc::now();
        self.repo.save(&task)?;
        self.sync_to_json()
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
        self.sync_to_json()
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
            if config.context.strategy == "raw_content" {
                let full_path = self.root_dir.join(file_path);
                if full_path.exists() {
                    let content = fs::read_to_string(full_path)?;
                    context.push_str("```\n");
                    context.push_str(&content);
                    context.push_str("\n```\n");
                }
            }
        }

        Ok((task, context))
    }

    pub fn get_config(&self) -> Result<Config> {
        self.config_repo.load()
    }

    pub fn save_task(&self, task: &Task) -> Result<()> {
        self.repo.save(task)?;
        self.sync_to_json()
    }

    pub fn move_file(&self, old_path: &str, new_path: &str) -> Result<()> {
        let config = self.config_repo.load()?;
        let all_tasks = self.repo.find_all()?;
        let mut updated = false;

        for mut task in all_tasks {
            let mut file_updated = false;
            for file in task.linked_files.iter_mut() {
                if file == old_path {
                    *file = new_path.to_string();
                    file_updated = true;
                    updated = true;
                }
            }
            if file_updated {
                task.updated_at = Utc::now();
                self.repo.save(&task)?;
            }
        }

        if updated {
            self.sync_to_json()?;
        }

        // 物理ファイルの移動
        let old_full = self.root_dir.join(old_path);
        let new_full = self.root_dir.join(new_path);

        if config.integration.git_mv_hook {
            let status = std::process::Command::new("git")
                .arg("mv")
                .arg(old_path)
                .arg(new_path)
                .current_dir(&self.root_dir)
                .status();

            if let Ok(s) = status {
                if !s.success() {
                    let _ = fs::rename(old_full, new_full);
                }
            } else {
                let _ = fs::rename(old_full, new_full);
            }
        } else {
            let _ = fs::rename(old_full, new_full);
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
            if desc.is_empty() {
                None
            } else {
                Some(desc)
            }
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
    fn test_init() -> Result<()> {
        let dir = tempdir()?;
        let root = dir.path().to_path_buf();

        TodoUsecase::init(root.clone())?;

        assert!(root.join(".mytodo").exists());
        assert!(root.join(".mytodo/data.db").exists());
        assert!(root.join(".mytodo/config.yaml").exists());
        assert!(root.join(".gitignore").exists());

        let gitignore = fs::read_to_string(root.join(".gitignore"))?;
        assert!(gitignore.contains(".mytodo/data.db"));

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

        let tasks = usecase.list_tasks()?;
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

        // 1. タスク追加
        let _task = usecase.add_task("Base Task".to_string(), None, None)?;

        // 2. JSONを直接書き換えて古いデータにする
        let mut tasks_in_json = usecase.json_repo.load_all()?;
        let original_updated_at = tasks_in_json[0].updated_at;
        tasks_in_json[0].title = "Old JSON".to_string();
        tasks_in_json[0].updated_at = original_updated_at - Duration::minutes(10);
        usecase.json_repo.save_all(&tasks_in_json)?;

        // 3. 同期実行（DBの方が新しいのでDB優先）
        usecase.sync()?;
        let tasks = usecase.list_tasks()?;
        assert_eq!(tasks[0].title, "Base Task");

        // 4. JSONを新しくする
        let mut tasks_in_json = usecase.json_repo.load_all()?;
        tasks_in_json[0].title = "New JSON".to_string();
        tasks_in_json[0].updated_at = Utc::now() + Duration::minutes(10);
        usecase.json_repo.save_all(&tasks_in_json)?;

        // 5. 同期実行（JSONの方が新しいのでJSON優先）
        usecase.sync()?;
        let tasks = usecase.list_tasks()?;
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
        usecase.repo.save(&task)?;

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

        // 関連ファイルの作成
        let file_path = "test_file.txt";
        fs::write(root.join(file_path), "File Content")?;

        let mut task = usecase.add_task("Context Test".to_string(), Some("Desc".to_string()), None)?;
        task.linked_files.push(file_path.to_string());
        usecase.repo.save(&task)?;

        let (_, context) = usecase.get_task_context(1)?;
        assert!(context.contains("Context Test"));
        assert!(context.contains("Desc"));
        assert!(context.contains("test_file.txt"));

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

        // 1. タスク作成とファイル紐付け
        let old_path = "old_file.txt";
        let new_path = "new_file.txt";
        fs::write(root.join(old_path), "content")?;

        let mut task = usecase.add_task("Move Test".to_string(), None, None)?;
        task.linked_files.push(old_path.to_string());
        usecase.repo.save(&task)?;

        // 2. move_file を実行
        usecase.move_file(old_path, new_path)?;

        // 3. 全タスクをチェックし、古いパスが新しいパスに置換されているか確認
        let tasks = usecase.list_tasks()?;
        assert_eq!(tasks[0].linked_files[0], new_path);
        assert!(root.join(new_path).exists());
        assert!(!root.join(old_path).exists());

        Ok(())
    }
}


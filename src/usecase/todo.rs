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
    _config_repo: YamlConfigRepository,
    _root_dir: PathBuf,
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
            _config_repo: config_repo,
            _root_dir: root_dir,
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
        let task = usecase.add_task("Base Task".to_string(), None, None)?;

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
}

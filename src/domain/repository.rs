use crate::domain::task::Task;
use anyhow::Result;
use uuid::Uuid;

pub trait TaskRepository {
    fn save(&self, task: &Task) -> Result<()>;
    fn find_by_local_id(&self, local_id: i32) -> Result<Option<Task>>;
    fn find_by_global_id(&self, global_id: Uuid) -> Result<Option<Task>>;
    fn find_all(&self) -> Result<Vec<Task>>;
    fn delete(&self, local_id: i32) -> Result<()>;
}

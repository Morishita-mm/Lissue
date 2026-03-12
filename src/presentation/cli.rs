use crate::domain::task::{Status, Task};
use clap::{Parser, Subcommand};
use std::collections::HashMap;
use uuid::Uuid;

#[derive(Parser)]
#[command(name = "lissue")]
#[command(about = "A local TODO CLI for project management", long_about = None)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Initialize the .lissue directory and database
    Init,
    /// Add a new task
    Add {
        /// Title of the task (opens editor if omitted)
        title: Option<String>,
        /// Optional detailed description
        #[arg(short, long)]
        message: Option<String>,
        /// Parent task ID to create a subtask
        #[arg(short, long)]
        parent: Option<i32>,
        /// Paths to files related to this task
        #[arg(short, long)]
        files: Vec<String>,
    },
    /// List tasks with filtering and formatting
    List {
        /// Output format (human or json)
        #[arg(short, long)]
        format: Option<String>,
        /// Display tasks in a tree structure
        #[arg(short, long)]
        tree: bool,
        /// Filter by status (Open or Close)
        #[arg(short, long)]
        status: Option<Status>,
        /// Filter to show only unassigned tasks
        #[arg(short, long)]
        unassigned: bool,
    },
    /// Get the next recommended task to work on (Open and Unassigned)
    Next,
    /// Close a task by its local ID
    Close { local_id: i32 },
    /// Reopen a closed task
    Open { local_id: i32 },
    /// Link a child task to a parent task
    Link {
        child_id: i32,
        /// The local ID of the parent task
        #[arg(long)]
        to: i32,
    },
    /// Remove the parent-child relationship from a task
    Unlink { child_id: i32 },
    /// Claim a task and set an assignee
    Claim {
        local_id: i32,
        /// Name of the assignee (e.g., agent name or username)
        #[arg(long)]
        by: Option<String>,
    },
    /// Dump task details and linked file contents for AI context
    Context { local_id: i32 },
    /// Synchronize the local database with partitioned JSON files
    Sync,
    /// Move a linked file and update the path in all relevant tasks
    Mv { old_path: String, new_path: String },
    /// Permanently remove a task from the database and JSON
    Rm { local_id: i32 },
    /// Permanently remove all closed tasks
    Clear,
}

pub fn print_tasks_human(tasks: &[Task], tree: bool) {
    if tree {
        print_task_tree(tasks);
    } else {
        println!(
            "{:<5} {:<10} {:<30} {:<20}",
            "ID", "Status", "Title", "Updated At"
        );
        println!("{}", "-".repeat(70));
        for task in tasks {
            println!(
                "{:<5} {:<10} {:<30} {:<20}",
                task.local_id.unwrap_or(0),
                task.status.to_string(),
                task.title,
                task.updated_at.format("%Y-%m-%d %H:%M").to_string()
            );
        }
    }
}

fn print_task_tree(tasks: &[Task]) {
    let tasks_by_id: HashMap<Uuid, &Task> = tasks.iter().map(|t| (t.global_id, t)).collect();
    let mut children_map: HashMap<Option<Uuid>, Vec<Uuid>> = HashMap::new();

    for task in tasks {
        children_map
            .entry(task.parent_global_id)
            .or_default()
            .push(task.global_id);
    }

    fn print_node(
        id: Uuid,
        tasks_by_id: &HashMap<Uuid, &Task>,
        children_map: &HashMap<Option<Uuid>, Vec<Uuid>>,
        indent: usize,
    ) {
        if let Some(task) = tasks_by_id.get(&id) {
            let prefix = "  ".repeat(indent);
            let status_mark = if task.status == Status::Close {
                "[x]"
            } else {
                "[ ]"
            };
            println!(
                "{}{} {} (ID: {})",
                prefix,
                status_mark,
                task.title,
                task.local_id.unwrap_or(0)
            );

            if let Some(children) = children_map.get(&Some(id)) {
                for child_id in children {
                    print_node(*child_id, tasks_by_id, children_map, indent + 1);
                }
            }
        }
    }

    if let Some(roots) = children_map.get(&None) {
        for root_id in roots {
            print_node(*root_id, &tasks_by_id, &children_map, 0);
        }
    }
}

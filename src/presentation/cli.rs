use crate::domain::task::{Status, Task};
use clap::{Parser, Subcommand};
use std::collections::HashMap;
use uuid::Uuid;

#[derive(Parser)]
#[command(name = "todo")]
#[command(about = "A local TODO CLI for project management", long_about = None)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Initialize the .mytodo directory and database
    Init,
    /// Add a new task
    Add {
        title: Option<String>,
        #[arg(short, long)]
        message: Option<String>,
        #[arg(short, long)]
        parent: Option<i32>,
        #[arg(short, long)]
        files: Vec<String>,
    },
    /// List all tasks
    List {
        #[arg(short, long, default_value = "human")]
        format: String,
        #[arg(short, long)]
        tree: bool,
    },
    /// Close a task
    Close { local_id: i32 },
    /// Open a task
    Open { local_id: i32 },
    /// Link two tasks in a parent-child relationship
    Link {
        child_id: i32,
        #[arg(long)]
        to: i32,
    },
    /// Unlink a task from its parent
    Unlink { child_id: i32 },
    /// Synchronize DB and JSON
    Sync,
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

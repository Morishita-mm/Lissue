use crate::domain::task::Status;
use clap::{Parser, Subcommand};

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
    /// Attach files to an existing task
    Attach {
        /// Local ID of the task
        local_id: i32,
        /// File paths to attach
        files: Vec<String>,
    },
    /// Permanently remove all closed tasks
    Clear,
}

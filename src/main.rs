use anyhow::Result;
use clap::Parser;
use rust_todo_cli::domain;
use rust_todo_cli::presentation::{Cli, Commands};
use rust_todo_cli::usecase::TodoUsecase;
use std::env;

fn main() -> Result<()> {
    let cli = Cli::parse();
    let root_dir = env::current_dir()?;

    match cli.command {
        Commands::Init => {
            TodoUsecase::init(root_dir)?;
            println!("Initialized .mytodo repository.");
        }
        Commands::Add {
            title,
            message,
            parent,
            files: _,
        } => {
            let usecase = TodoUsecase::new(root_dir)?;
            let final_title = if let Some(t) = title {
                t
            } else {
                // TODO: EDITOR起動ロジック
                return Err(anyhow::anyhow!("Title is required currently."));
            };
            let task = usecase.add_task(final_title, message, parent)?;
            println!("Task created with ID: {}", task.local_id.unwrap_or(0));
        }
        Commands::List { format, tree } => {
            let usecase = TodoUsecase::new(root_dir)?;
            let tasks = usecase.list_tasks()?;
            if format == "json" {
                println!("{}", serde_json::to_string_pretty(&tasks)?);
            } else {
                rust_todo_cli::presentation::cli::print_tasks_human(&tasks, tree);
            }
        }
        Commands::Close { local_id } => {
            let usecase = TodoUsecase::new(root_dir)?;
            usecase.update_status(local_id, domain::task::Status::Close)?;
            println!("Task {} closed.", local_id);
        }
        Commands::Open { local_id } => {
            let usecase = TodoUsecase::new(root_dir)?;
            usecase.update_status(local_id, domain::task::Status::Open)?;
            println!("Task {} opened.", local_id);
        }
        Commands::Link { child_id, to } => {
            let usecase = TodoUsecase::new(root_dir)?;
            usecase.link_tasks(child_id, to)?;
            println!("Linked task {} to parent {}.", child_id, to);
        }
        Commands::Unlink { child_id } => {
            let usecase = TodoUsecase::new(root_dir)?;
            usecase.unlink_task(child_id)?;
            println!("Unlinked task {}.", child_id);
        }
        Commands::Sync => {
            let usecase = TodoUsecase::new(root_dir)?;
            usecase.sync()?;
            println!("Synchronized database with tasks.json.");
        }
    }

    Ok(())
}

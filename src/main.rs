use anyhow::Result;
use clap::Parser;
use lissue::domain;
use lissue::presentation::{Cli, Commands};
use lissue::usecase::todo::TaskFilter;
use lissue::usecase::TodoUsecase;
use std::env;
use std::fs;
use std::io::Read;
use std::process::Command;

fn main() -> Result<()> {
    let cli = Cli::parse();
    let root_dir = env::current_dir()?;

    match cli.command {
        Commands::Init => {
            TodoUsecase::init(root_dir)?;
            println!("Initialized .lissue repository.");
        }
        Commands::Add {
            title,
            message,
            parent,
            files,
        } => {
            let usecase = TodoUsecase::new(root_dir)?;
            let (final_title, final_message): (String, Option<String>) = if let Some(t) = title {
                (t, message)
            } else {
                let content = run_editor()?;
                TodoUsecase::parse_editor_content(&content)
            };

            if final_title.is_empty() {
                return Err(anyhow::anyhow!("Title is required."));
            }

            let mut task = usecase.add_task(final_title, final_message, parent)?;
            if !files.is_empty() {
                let mut linked = task.linked_files.clone();
                linked.extend(files);
                task.linked_files = linked;
                usecase.save_task(&task)?;
            }
            println!("Task created with ID: {}", task.local_id.unwrap_or(0));
        }
        Commands::List {
            format,
            tree,
            status,
            unassigned,
        } => {
            let usecase = TodoUsecase::new(root_dir)?;
            let config = usecase.get_config()?;
            let final_format = format.unwrap_or(config.output.default_format);
            let tasks = usecase.list_tasks(TaskFilter { status, unassigned })?;
            if final_format == "json" {
                println!("{}", serde_json::to_string_pretty(&tasks)?);
            } else {
                lissue::presentation::cli::print_tasks_human(&tasks, tree);
            }
        }
        Commands::Next => {
            let usecase = TodoUsecase::new(root_dir)?;
            if let Some(task) = usecase.get_next_task()? {
                let tasks = vec![task];
                lissue::presentation::cli::print_tasks_human(&tasks, false);
            } else {
                println!("No tasks available.");
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
        Commands::Claim { local_id, by } => {
            let usecase = TodoUsecase::new(root_dir)?;
            usecase.claim_task(local_id, by.clone())?;
            println!(
                "Task {} claimed by {}.",
                local_id,
                by.unwrap_or_else(|| "anonymous".to_string())
            );
        }
        Commands::Context { local_id } => {
            let usecase = TodoUsecase::new(root_dir)?;
            let (_, context) = usecase.get_task_context(local_id)?;
            println!("{}", context);
        }
        Commands::Sync => {
            let usecase = TodoUsecase::new(root_dir)?;
            usecase.sync()?;
            println!("Synchronized database with tasks.json.");
        }
        Commands::Mv { old_path, new_path } => {
            let usecase = TodoUsecase::new(root_dir)?;
            usecase.move_file(&old_path, &new_path)?;
            println!("Moved {} to {} and updated tasks.", old_path, new_path);
        }
        Commands::Rm { local_id } => {
            let usecase = TodoUsecase::new(root_dir)?;
            usecase.delete_task(local_id)?;
            println!("Task {} removed permanently.", local_id);
        }
        Commands::Clear => {
            let usecase = TodoUsecase::new(root_dir)?;
            let count = usecase.clear_closed_tasks()?;
            println!("Cleared {} closed tasks.", count);
        }
    }

    Ok(())
}

fn run_editor() -> Result<String> {
    let editor = env::var("EDITOR").unwrap_or_else(|_| "vi".to_string());
    let temp_file = tempfile::NamedTempFile::new()?;
    let temp_path = temp_file.path();

    let status = Command::new(editor).arg(temp_path).status()?;

    if !status.success() {
        return Err(anyhow::anyhow!("Editor failed to exit successfully."));
    }

    let mut content = String::new();
    fs::File::open(temp_path)?.read_to_string(&mut content)?;
    Ok(content)
}

# lissue

[![Crates.io](https://img.shields.io/crates/v/lissue.svg)](https://crates.io/crates/lissue)
**lissue** (short for **Local Issue**) is a powerful, Git-friendly local TODO CLI designed for developers and AI coding agents. It manages your project tasks using a hybrid storage of SQLite (for speed) and JSON (for Git-based sharing).

![Demo](docs/cli_demo.gif)
![TUI_Demo](docs/tui_demo.gif)

## Why lissue?

The name **lissue** comes from "Local Issue". It aims to bring the powerful issue-tracking experience of GitHub/GitLab directly to your local terminal, enabling seamless collaboration between humans and AI agents through Git.

## Key Features

- **Interactive TUI**: A beautiful, Lazygit-inspired terminal interface for humans.
- **Hybrid Storage**: Fast local operations with SQLite + Git-syncable JSON storage.
- **Git-Optimized**: "1 Task per File" architecture prevents merge conflicts.
- **AI-Ready**: Specialized commands for AI agents, including context dumping and auto-locking.
- **Hierarchical Tasks**: Support for parent-child relationships and tree view display.
- **Vim-like Experience**: Familiar keybindings (`/`, `j/k`, `h/l`) for speed.

## Installation

### From crates.io (Recommended)

```bash
cargo install lissue
```

### From Source

```bash
# Clone the repository
git clone https://github.com/Morishita-mm/Lissue
cd Lissue

# Install locally
cargo install --path .
```

## Quick Start

1. **Initialize** the repository in your project root:

   ```bash
   lissue init
   ```

2. **Launch TUI** (Interactive Mode):

   ```bash
   lissue
   ```

3. **Add** a task via CLI:

   ```bash
   lissue add "Main task" -m "Main description"
   # Add a subtask to ID 1 with related files
   lissue add "Sub task" -p 1 -f src/main.rs
   ```

## TUI Guide

Run `lissue` without arguments to enter the interactive mode.

| Key | Action |
| :--- | :--- |
| `j` / `k` | Navigate tasks or files |
| `h` / `l` | Switch between Status tabs (Open, Doing, Pending, Done) |
| `/` | Search tasks or files (Fuzzy matching) |
| `a` | Quick add a new task title |
| `A` | **Attach mode**: Toggle project files to the selected task |
| `m` | Edit full task description in your `$EDITOR` |
| `d` | Mark task as Done |
| `c` | Claim task (Assign to yourself and mark as Doing) |
| `s` | Sync with JSON files |
| `q` / `Esc` | Quit or exit current mode |

## Command Reference

| Command | Options | Description |
| :--- | :--- | :--- |
| `lissue init` | | Initialize `.lissue` directory and database. |
| `lissue add [TITLE]` | `-m`, `-p`, `-f` | Add a new task. `-p`: parent ID, `-f`: linked file. |
| `lissue list` | `-f`, `-t`, `-s`, `-u` | List tasks. `-t`: tree, `-s`: status, `-u`: unassigned. |
| `lissue attach <ID> <FILES>...` | | Link existing files to a task. |
| `lissue next` | | Get the next available task (Open & Unassigned). |
| `lissue claim <ID>` | `--by` | Mark task as In Progress and assign to yourself/agent. |
| `lissue close <ID>` | | Close a task. |
| `lissue open <ID>` | | Reopen a closed task. |
| `lissue link <ID>` | `--to` | Create a parent-child relationship. |
| `lissue context <ID>`| | Dump task details and linked file contents for AI. |
| `lissue mv <OLD> <NEW>`| | Move a file and update all linked tasks. |
| `lissue rm <ID>` | | Permanently remove a task. |
| `lissue clear` | | Permanently remove all closed tasks. |

## Configuration

Settings are stored in `.lissue/config.yaml`:

- `output.default_format`: `human` or `json`
- `output.auto_sync`: Enable/disable implicit sync during `list` or `next`.
- `integration.git_mv_hook`: Use `git mv` during `lissue mv`.
- `context.strategy`: `paths_only` or `raw_content`.

## License

MIT OR Apache-2.0

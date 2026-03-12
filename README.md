# lissue

**lissue** (short for **Local Issue**) is a powerful, Git-friendly local TODO CLI designed for developers and AI coding agents. It manages your project tasks using a hybrid storage of SQLite (for speed) and JSON (for Git-based sharing).

![Demo](docs/demo.gif)

## Why lissue?

The name **lissue** comes from "Local Issue". It aims to bring the powerful issue-tracking experience of GitHub/GitLab directly to your local terminal, enabling seamless collaboration between humans and AI agents through Git.

## Key Features

- **Hybrid Storage**: Fast local operations with SQLite + Git-syncable JSON storage.
- **Git-Optimized**: "1 Task per File" architecture prevents merge conflicts.
- **AI-Ready**: Specialized commands for AI agents, including context dumping and auto-locking.
- **Hierarchical Tasks**: Support for parent-child relationships and tree view display.
- **Secure**: Built-in protection against path traversal and SQL injection.
- **Flexible**: Editor integration, file movement tracking, and extensive filtering.

## Installation

```bash
# Clone the repository
git clone https://github.com/Morishita-mm/rust-todo-cli
cd rust-todo-cli

# Install locally
cargo install --path .
```

## Quick Start

1. **Initialize** the repository in your project root:
   ```bash
   lissue init
   ```
2. **Add** a task:
   ```bash
   lissue add "Implement login feature" -m "Use OAuth2"
   # Or just 'lissue add' to open your $EDITOR
   ```
3. **List** tasks:
   ```bash
   lissue list --tree
   ```
4. **Claim** a task (for AI agents or team members):
   ```bash
   lissue claim 1 --by "Agent-Alpha"
   ```
5. **Sync** with Git (Apply merged JSON files to your local database):
   ```bash
   lissue sync
   ```

## Command Reference

| Command | Description |
| :--- | :--- |
| `lissue init` | Initialize `.lissue` directory and database. |
| `lissue add [TITLE]` | Add a new task. Opens editor if title is omitted. |
| `lissue list` | List tasks. Supports `--format json`, `--tree`, and filters. |
| `lissue next` | Get the next available task (Open & Unassigned). |
| `lissue claim <ID>` | Mark task as In Progress and assign to yourself/agent. |
| `lissue close <ID>` | Close a task. |
| `lissue open <ID>` | Reopen a closed task. |
| `lissue link <ID> --to <PID>` | Create a parent-child relationship. |
| `lissue context <ID>` | Dump task details and linked file contents for AI. |
| `lissue mv <OLD> <NEW>` | Move a file and update all linked tasks. |
| `lissue rm <ID>` | Permanently remove a task. |
| `lissue clear` | Permanently remove all closed tasks. |

## Configuration

Settings are stored in `.lissue/config.yaml`:
- `output.default_format`: `human` or `json`
- `output.auto_sync`: Enable/disable implicit sync during `list` or `next`.
- `integration.git_mv_hook`: Use `git mv` during `lissue mv`.
- `context.strategy`: `paths_only` or `raw_content`.

## License

MIT OR Apache-2.0

# rust-todo-cli

A powerful, Git-friendly local TODO CLI designed for developers and AI coding agents. It manages your project tasks using a hybrid storage of SQLite (for speed) and JSON (for Git-based sharing).

![Demo](docs/demo.gif)

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
   mytodo init
   ```
2. **Add** a task:
   ```bash
   mytodo add "Implement login feature" -m "Use OAuth2"
   # Or just 'mytodo add' to open your $EDITOR
   ```
3. **List** tasks:
   ```bash
   mytodo list --tree
   ```
4. **Claim** a task (for AI agents or team members):
   ```bash
   mytodo claim 1 --by "Agent-Alpha"
   ```
5. **Sync** with Git (Apply merged JSON files to your local database):
   ```bash
   mytodo sync
   ```

## Command Reference

| Command | Description |
| :--- | :--- |
| `mytodo init` | Initialize `.mytodo` directory and database. |
| `mytodo add [TITLE]` | Add a new task. Opens editor if title is omitted. |
| `mytodo list` | List tasks. Supports `--format json`, `--tree`, and filters. |
| `mytodo next` | Get the next available task (Open & Unassigned). |
| `mytodo claim <ID>` | Mark task as In Progress and assign to yourself/agent. |
| `mytodo close <ID>` | Close a task. |
| `mytodo open <ID>` | Reopen a closed task. |
| `mytodo link <ID> --to <PID>` | Create a parent-child relationship. |
| `mytodo context <ID>` | Dump task details and linked file contents for AI. |
| `mytodo mv <OLD> <NEW>` | Move a file and update all linked tasks. |
| `mytodo rm <ID>` | Permanently remove a task. |
| `mytodo clear` | Permanently remove all closed tasks. |

## Configuration

Settings are stored in `.mytodo/config.yaml`:
- `output.default_format`: `human` or `json`
- `output.auto_sync`: Enable/disable implicit sync during `list` or `next`.
- `integration.git_mv_hook`: Use `git mv` during `mytodo mv`.
- `context.strategy`: `paths_only` or `raw_content`.

## License

MIT OR Apache-2.0

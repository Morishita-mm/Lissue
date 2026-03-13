# lissue

[![Crates.io](https://img.shields.io/crates/v/lissue.svg)](https://crates.io/crates/lissue)
**lissue**（Local Issueの略）は、開発者やAIコーディングエージェント向けに設計された、GitフレンドリーなローカルTODO管理CLIツールです。SQLite（高速なローカル操作）とJSON（Gitベースの共有）のハイブリッドストレージを使用してプロジェクトのタスクを管理します。

![Demo](docs/cli_demo.gif)
![TUI_Demo](docs/tui_demo.gif)

## なぜ lissue なのか？

**lissue** という名前は「Local Issue」に由来します。GitHubやGitLabの強力なIssue管理体験をローカルのターミナルに持ち込み、Gitを通じて人間とAIエージェントがシームレスに協力できるようにすることを目的としています。

## 主な特徴

- **インタラクティブなTUI**: Lazygit風の直感的なターミナルインターフェース。
- **ハイブリッドストレージ**: SQLiteによる高速なレスポンスと、Git管理可能なJSON形式での保存。
- **Git最適化**: 「1タスク1ファイル」アーキテクチャにより、Gitマージ時の競合を最小限に。
- **AI Ready**: AIエージェント向けのコンテキスト出力機能や、作業の排他ロック機能を完備。
- **階層構造**: 親子関係のサポートとツリー形式での表示。
- **Vim風の操作感**: `/`, `j/k`, `h/l` など、開発者に馴染みのあるキーバインド。

## インストール

### crates.io から (推奨)

```bash
cargo install lissue
```

### ソースから

```bash
git clone https://github.com/Morishita-mm/Lissue
cd Lissue
cargo install --path .
```

## クイックスタート

1. プロジェクトのルートで**初期化**します：

   ```bash
   lissue init
   ```

2. **TUIを起動**します（対話モード）：

   ```bash
   lissue
   ```

3. CLIから**タスクを追加**します：

   ```bash
   lissue add "メインタスク" -m "詳細な説明"
   # ID 1の子タスクとしてファイルに関連付けて追加
   lissue add "サブタスク" -p 1 -f src/main.rs
   ```

## TUI 操作ガイド

引数なしで `lissue` を実行すると、インタラクティブモードに入ります。

| キー | アクション |
| :--- | :--- |
| `j` / `k` | タスクまたはファイルの選択移動 |
| `h` / `l` | ステータスタブ（Open, Doing, Pending, Done）の切り替え |
| `/` | リアルタイム検索（タスク・ファイル共通） |
| `a` | タスクのクイック追加（タイトルのみ） |
| `A` | **ファイル関連付けモード**: プロジェクト内のファイルをトグル選択 |
| `m` | エディタ（`$EDITOR`）を起動して詳細を編集 |
| `d` | タスクを完了（Done）にする |
| `c` | タスクを担当する（自分をAssigneeに設定） |
| `s` | JSONファイルとの同期（Sync） |
| `q` / `Esc` | 終了、またはモード解除 |

## コマンドリファレンス

| コマンド | オプション | 説明 |
| :--- | :--- | :--- |
| `lissue init` | | `.lissue` ディレクトリとデータベースを初期化します。 |
| `lissue add [TITLE]` | `-m`, `-p`, `-f` | 新規タスクを追加。`-p`: 親ID, `-f`: 関連ファイル。 |
| `lissue list` | `-f`, `-t`, `-s`, `-u` | 一覧表示。`-t`: ツリー, `-s`: ステータス, `-u`: 未割当。 |
| `lissue attach <ID> <FILES>...` | | 既存のタスクにファイルを関連付けます。 |
| `lissue next` | | 次に着手すべきタスク（Open かつ 未割当）を取得します。 |
| `lissue claim <ID>` | `--by` | タスクを「着手中」にし、自分またはエージェントを割り当てます。 |
| `lissue close <ID>` | | タスクを完了にします。 |
| `lissue open <ID>` | | 完了したタスクを再開します。 |
| `lissue link <ID>` | `--to` | タスクの親子関係を設定します。 |
| `lissue context <ID>`| | AI向けにタスクの詳細と関連ファイルの内容を出力します。 |
| `lissue mv <OLD> <NEW>`| | ファイルを移動し、関連する全タスクのパスを更新します。 |
| `lissue rm <ID>` | | タスクを完全に削除します。 |
| `lissue clear` | | 完了済みのタスクを一括削除します。 |

## 設定

設定は `.lissue/config.yaml` に保存されます：

- `output.default_format`: `human` または `json`
- `output.auto_sync`: `list` や `next` 実行時の自動同期。
- `integration.git_mv_hook`: `lissue mv` 実行時に `git mv` を使用するか。
- `context.strategy`: `paths_only` または `raw_content`。

## ライセンス

MIT OR Apache-2.0

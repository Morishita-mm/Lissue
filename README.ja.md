# lissue

[![Crates.io](https://img.shields.io/crates/v/lissue.svg)](https://crates.io/crates/lissue)
**lissue**（Local Issueの略称）は、開発者とAIエージェントのために設計された、GitフレンドリーなローカルTODO管理CLIツール。SQLite（高速なローカル操作）とJSON（Gitによる共有）のハイブリッドストレージを採用しています。

![Demo](docs/demo.gif)

## 名前の由来

**lissue**（リシュー）という名前は、Local Issueに由来します。GitHubやGitLabの強力なIssue管理体験をローカル環境（ターミナル）に持ち込み、Gitを通じて人間とAIエージェントが円滑に連携できる環境を目指しています。

## 主な機能

- **ハイブリッドストレージ**: SQLiteによる高速なレスポンスと、Gitで管理可能なJSON形式の永続化を両立。
- **Git最適化**: 「1タスク1ファイル」アーキテクチャにより、Gitマージ時のコンフリクトを物理的に回避。
- **AIエージェント対応**: コンテキスト出力（context）や排他ロック（claim）など、AIが自律的に動くための専用コマンドを搭載。
- **階層構造**: タスク間の親子関係（リンク）と、直感的なツリー表示をサポート。
- **安全性**: パス・トラバーサルやSQLインジェクションに対する防御を標準実装。
- **柔軟性**: エディタ連携、ファイル移動への追従、詳細なフィルタリング。

## インストール

### crates.io からインストール (推奨)
```bash
cargo install lissue
```

### ソースからビルド
```bash
# リポジトリをクローン
git clone https://github.com/Morishita-mm/rust-todo-cli
cd rust-todo-cli

# ローカルにインストール
cargo install --path .
```

## クイックスタート

1. プロジェクトルートで **初期化** を行います。
   ```bash
   lissue init
   ```
2. タスクを **追加** します。
   ```bash
   lissue add "メインタスク" -m "メインの説明"
   # 親タスク ID: 1 に紐づくサブタスクを関連ファイルと共に作成
   lissue add "サブタスク" -p 1 -f src/main.rs -f src/lib.rs
   # 引数なしで実行するとエディタが開きます
   ```
3. タスクを **一覧表示** します。
   ```bash
   lissue list --tree
   ```
4. タスクを **確保（Claim）** します（AIエージェントやチームメンバー用）。
   ```bash
   lissue claim 1 --by "Agent-Alpha"
   ```
5. Git経由で共有されたデータを **同期** します。
   ```bash
   lissue sync
   ```

## コマンドリファレンス

| コマンド | オプション | 説明 |
| :--- | :--- | :--- |
| `lissue init` | | `.lissue` ディレクトリとデータベースを初期化します。 |
| `lissue add [TITLE]` | `-m`, `-p`, `-f` | タスクを追加します。`-p`: 親ID, `-f`: 関連ファイル。 |
| `lissue list` | `-f`, `-t`, `-s`, `-u` | 一覧表示。`-t`: ツリー, `-s`: 状態, `-u`: 未割当。 |
| `lissue next` | | 次に着手すべきタスク（Open かつ 未割当）を取得します。 |
| `lissue claim <ID>` | `--by` | ステータスを In Progress にし、担当者を割り当てます。 |
| `lissue close <ID>` | | タスクを完了（Close）します。 |
| `lissue open <ID>` | | 完了したタスクを再開（Open）します。 |
| `lissue link <ID>` | `--to` | タスク間に親子関係を構築します。 |
| `lissue context <ID>` | | AI向けにタスク詳細と関連ファイルの内容を出力します。 |
| `lissue mv <OLD> <NEW>` | | ファイルを移動し、タスクに紐付くパスを一括更新します。 |
| `lissue rm <ID>` | | タスクを物理削除します。 |
| `lissue clear` | | クローズ済みのタスクをすべて一括削除します。 |

## ヘルプとコマンドの探索

すべてのコマンドとオプションは、以下の方法で確認できます：
- `lissue help`: 利用可能なサブコマンドの一覧を表示します。
- `lissue <COMMAND> --help`: 特定のサブコマンドの詳細なヘルプを表示します（例：`lissue add --help`）。

## 設定ファイル

設定は `.lissue/config.yaml` で管理されます。
- `output.default_format`: デフォルトの出力形式（`human` または `json`）。
- `output.auto_sync`: `list` や `next` 実行時の自動同期の有効/無効。
- `integration.git_mv_hook`: `lissue mv` 実行時に `git mv` を使用するかどうか。
- `context.strategy`: コンテキスト出力の戦略（`paths_only` または `raw_content`）。

## ライセンス

MIT OR Apache-2.0

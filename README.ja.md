# rust-todo-cli

開発者とAIエージェントのために設計された、GitフレンドリーなローカルTODO管理CLIツール。SQLite（高速なローカル操作）とJSON（Gitによる共有）のハイブリッドストレージを採用しています。

![Demo](docs/demo.gif)

## 主な機能

- **ハイブリッドストレージ**: SQLiteによる高速なレスポンスと、Gitで管理可能なJSON形式の永続化を両立。
- **Git最適化**: 「1タスク1ファイル」アーキテクチャにより、Gitマージ時のコンフリクトを物理的に回避。
- **AIエージェント対応**: コンテキスト出力（context）や排他ロック（claim）など、AIが自律的に動くための専用コマンドを搭載。
- **階層構造**: タスク間の親子関係（リンク）と、直感的なツリー表示をサポート。
- **安全性**: パス・トラバーサルやSQLインジェクションに対する防御を標準実装。
- **柔軟性**: エディタ連携、ファイル移動への追従、詳細なフィルタリング。

## インストール

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
   mytodo init
   ```
2. タスクを **追加** します。
   ```bash
   mytodo add "ログイン機能を実装する" -m "OAuth2を使用すること"
   # 引数なしで実行するとエディタが開きます
   ```
3. タスクを **一覧表示** します。
   ```bash
   mytodo list --tree
   ```
4. タスクを **確保（Claim）** します（AIエージェントやチームメンバー用）。
   ```bash
   mytodo claim 1 --by "Agent-Alpha"
   ```
5. Git経由で共有されたデータを **同期** します。
   ```bash
   mytodo sync
   ```

## コマンドリファレンス

| コマンド | 説明 |
| :--- | :--- |
| `mytodo init` | `.mytodo` ディレクトリとデータベースを初期化します。 |
| `mytodo add [TITLE]` | タスクを追加します。タイトル省略時はエディタを起動します。 |
| `mytodo list` | 一覧表示。`--format json` や `--tree`、フィルタリングが可能です。 |
| `mytodo next` | 次に着手すべきタスク（Open かつ 未割当）を取得します。 |
| `mytodo claim <ID>` | ステータスを In Progress にし、担当者を割り当てます。 |
| `mytodo close <ID>` | タスクを完了（Close）します。 |
| `mytodo open <ID>` | 完了したタスクを再開（Open）します。 |
| `mytodo link <ID> --to <PID>` | タスク間に親子関係を構築します。 |
| `mytodo context <ID>` | AI向けにタスク詳細と関連ファイルの内容を出力します。 |
| `mytodo mv <OLD> <NEW>` | ファイルを移動し、タスクに紐付くパスを一括更新します。 |
| `mytodo rm <ID>` | タスクを物理削除します。 |
| `mytodo clear` | クローズ済みのタスクをすべて一括削除します。 |

## 設定ファイル

設定は `.mytodo/config.yaml` で管理されます。
- `output.default_format`: デフォルトの出力形式（`human` または `json`）。
- `output.auto_sync`: `list` や `next` 実行時の自動同期の有効/無効。
- `integration.git_mv_hook`: `mytodo mv` 実行時に `git mv` を使用するかどうか。
- `context.strategy`: コンテキスト出力の戦略（`paths_only` または `raw_content`）。

## ライセンス

MIT OR Apache-2.0

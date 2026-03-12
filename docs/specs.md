## 1. プロジェクト概要

各プロジェクトごとにTODOを作成・管理できる、GitHub Issuesのローカル版CLIアプリケーション。
プロジェクトルートに専用の管理ディレクトリを作成し、組み込みDB（SQLite）で高速かつ柔軟にデータを管理する。同時にJSONファイルを介してGitでの共有・同期を可能とする。
また、複数のAIエージェントが並列で開発を行う環境を想定し、タスクの排他ロック機能と、LLM向けのコンテキスト出力機能（土管機能）を備える。

## 2. 技術スタック

* **言語:** Rust
* **アーキテクチャ:** クリーンアーキテクチャ、ドメイン駆動設計（DDD）
* **データベース:** SQLite（`rusqlite` または `sqlx` を推奨）
* **CLIパーサー:** `clap` (Derive API)
* **シリアライズ:** `serde`, `serde_json`, `serde_yaml`
* **識別子:** `uuid` (v4)

## 3. ディレクトリ構成とデータ保存方針

プロジェクトのルートディレクトリに `.mytodo` ディレクトリを作成し、以下のファイルを配置する。

* `.mytodo/data.db`: SQLiteデータベースファイル。ローカル環境専用とし、`.gitignore` に追加してGit管理から除外する。
* `.mytodo/tasks.json`: データ共有・同期用のJSONファイル。コマンド実行後にDBの内容を書き出し、Gitで管理する。
* `.mytodo/config.yaml`: アプリケーションの設定ファイル。

## 4. データモデル (DDD Entity)

タスクの操作性を高めるためのローカルID（連番）と、同期時の同一性保証のためのグローバルID（UUID）を併用する。

* **Task**
* `local_id`: Integer (SQLiteのAUTOINCREMENT。CLIでの操作指定用)
* `global_id`: UUID (タスクの一意性保証用)
* `title`: String (必須)
* `description`: Option<String> (詳細説明)
* `status`: Enum (Open, Close, In Progress, Pending)
* `assignee`: Option<String> (AIエージェント名やユーザー名。並列処理時の排他ロック用)
* `parent_global_id`: Option<UUID> (親タスクのUUID。Rust側でツリー構築時に使用)
* `linked_files`: Vec<String> (プロジェクトルートからの相対パスのリスト)
* `created_at`: DateTime (UTC)
* `updated_at`: DateTime (UTC)

## 5. コマンドインターフェース仕様

CLIはAIエージェントと人間の双方にとって使いやすい設計とする。デフォルトの出力形式は設定ファイルに依存する。

* `todo init`
* `.mytodo` ディレクトリ、SQLite DB、`config.yaml`、`.gitignore` への追記（`data.db`の除外）を行う。

* `todo add [TITLE] [-m <DESCRIPTION>] [-p <PARENT_LOCAL_ID>] [-f <FILE_PATH>...]`
* タスクを追加する。引数なしの場合は `$EDITOR` を起動。

* `todo list [--format <human|json>] [--tree]`
* タスク一覧を表示。`--tree` 時はRust側で親子関係を解析しツリー状に出力する。

* `todo claim <LOCAL_ID> [--by <AGENT_NAME>]`
* タスクの `status` を In Progress にし、`assignee` を記録して排他ロックをかける（並列開発時の重複着手防止）。

* `todo close <LOCAL_ID>` / `todo open <LOCAL_ID>`
* タスクのステータスを変更し、必要に応じて `assignee` をクリアする。

* `todo link <CHILD_LOCAL_ID> --to <PARENT_LOCAL_ID>` / `todo unlink <CHILD_LOCAL_ID>`
* 既存タスクの親子関係を操作する。

* `todo mv <OLD_PATH> <NEW_PATH>`
* ファイル移動の追従。OSの移動とDB内の `linked_files` パス更新を同時に行う。

* `todo sync`
* `.mytodo/tasks.json` とローカルDBを同期する（タイムスタンプ優先マージ）。

* `todo context <LOCAL_ID>`
* AI向けのコンテキスト出力機能。タスクの詳細と、`linked_files` に紐づくファイル群のパス（設定によってはファイル内容の結合テキスト）を標準出力にダンプする。

## 6. 同期ロジック (競合解決アルゴリズム)

`todo sync` 実行時のコンフリクト解決はタイムスタンプ優先（Last-Write-Wins）とする。

1. JSONから全タスクを読み込む。
2. DBの全タスクと `global_id` で突き合わせる。
3. 双方に存在する場合は `updated_at` が新しい方で上書き。
4. 片方にのみ存在する場合は新規追加。
5. 処理後、最新状態をDBとJSONの両方に書き戻す。

## 7. 設定ファイル

初期実装時は土管機能として振る舞い、将来的な拡張余地を残す。

```yaml
# .mytodo/config.yaml
output:
  default_format: human # human または json
integration:
  git_mv_hook: true
context:
  # 'paths_only': 関連ファイルパスのリストのみ出力する（土管機能）
  # 'raw_content': 関連ファイルの中身をそのまま結合して出力する
  strategy: paths_only

```

## 8. 実装上の重要要件

* **並列I/O対策:** SQLite接続時はWALモード（`PRAGMA journal_mode=WAL;`）とビジータイムアウトを設定し、複数エージェントからの同時書き込み時のロック競合を回避すること。
* **ステータス管理:** タスクの状態はEnumで永続化し、真理値型は使用しない。
* **親子関係の解析:** DB側で再帰クエリは使用せず、リポジトリ層からはフラットに取得し、ドメイン層でツリーを構築すること。
* **マークダウン出力に関する制約:** 実装中、マークダウンでテキストを出力する機能を設ける場合、「」の前後には強調のためのアスタリスクを含めないこと。

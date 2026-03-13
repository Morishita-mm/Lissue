## 1. プロジェクト概要

各プロジェクトごとにTODOを作成・管理できる、GitHub Issuesのローカル版CLIアプリケーション。
プロジェクトルートに専用の管理ディレクトリを作成し、組み込みDB（SQLite）で高速かつ柔軟にデータを管理する。同時にJSONファイルを介してGitでの共有・同期を可能とする。
また、複数のAIエージェントが並列で開発を行う環境を想定し、タスクの排他ロック機能と、LLM向けのコンテキスト出力機能（土管機能）を備える。

## 2. 技術スタック

* **言語:** Rust
* **アーキテクチャ:** クリーンアーキテクチャ、ドメイン駆動設計（DDD）
* **データベース:** SQLite（WALモード）
* **UI:** TUI (ratatui), CLI (clap)
* **シリアライズ:** `serde`, `serde_json`, `serde_yaml`
* **識別子:** `uuid` (v4)

## 3. ディレクトリ構成とデータ保存方針

プロジェクトのルートディレクトリ（または親ディレクトリへ遡って探索）に `.lissue` ディレクトリを作成し、以下のファイルを配置する。

* `.lissue/data.db`: SQLiteデータベースファイル。ローカル環境専用とし、`.gitignore` に追加してGit管理から除外する。
* `.lissue/tasks/*.json`: データ共有・同期用のJSONファイル。各タスクを個別のファイルとして保存し、Gitでの競合を最小限にする。
* `.lissue/config.yaml`: アプリケーションの設定ファイル。

## 4. データモデル (DDD Entity)

* **Task**
* `local_id`: Integer (SQLiteのAUTOINCREMENT。CLI/TUIでの操作指定用)
* `global_id`: UUID (タスクの一意性保証用)
* `title`: String (必須)
* `description`: Option<String> (詳細説明)
* `status`: Enum (Open, InProgress, Pending, Close)
* `assignee`: Option<String> (排他ロック用)
* `parent_global_id`: Option<UUID> (親子関係)
* `linked_files`: Vec<String> (プロジェクトルートからの相対パスのリスト)
* `created_at`: DateTime (UTC)
* `updated_at`: DateTime (UTC)

## 5. コマンドインターフェース仕様

### CLI サブコマンド
* `lissue init`: リポジトリの初期化。
* `lissue add`: タスクの追加。`-m` で詳細、`-p` で親タスク、`-f` でファイルを指定可能。
* `lissue list`: タスク一覧表示。`--tree` や `--status` フィルタに対応。
* `lissue claim`: タスクの担当者設定とステータス変更。
* `lissue attach`: 既存タスクへのファイル関連付け（実在確認付き）。
* `lissue context`: AI向けコンテキスト出力。
* `lissue sync`: JSONとDBの同期。
* `lissue mv`: ファイル移動に伴うパス更新。
* `lissue rm`: タスクの完全削除。
* `lissue clear`: 完了済みタスクの一括削除。

### TUI モード
引数なしの `lissue` で起動。
* `j/k`: 移動
* `h/l`: タブ切り替え
* `/`: 検索（タスク・ファイル共通）
* `a`: 新規タスク追加（ポップアップ）
* `A`: ファイル関連付け（インタラクティブ選択モード）
* `m`: 詳細編集（エディタ起動）
* `d`: タスク完了
* `c`: クレーム（担当設定）
* `s`: 同期

## 6. 同期ロジック
タイムスタンプ優先（Last-Write-Wins）方式を採用。

## 7. 設定ファイル
`.lissue/config.yaml` で出力形式や同期、コンテキスト出力の戦略を設定可能。

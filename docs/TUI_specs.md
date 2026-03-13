# lissue TUI 仕様書 (v1.0)

## 1. コンセプト

* **Lazygit-inspired**: 思考のスピードを止めない、直感的なペイン移動とキー操作。
* **Terminal-Agnostic**: 特殊な絵文字に頼らず、標準的なNerd FontsやASCII文字でリッチな表現を実現。
* **Bridge for AI & Human**: AIの稼働状況を一目で把握し、人間が即座に介入できるUI。

## 2. 画面レイアウト

画面を4つのメイン領域に分割します。

```text
+-------------------------------------------------------------+
| [1] Status / Tabs (Todo / Doing / Done / All)               |
+-----------------------+-------------------------------------+
| [2] Task List         | [3] Task Detail (Markdown)          |
|                       |                                     |
| > #123 Implement TUI  | ## Description                      |
|   #124 Fix SQLite bug | This task focuses on...             |
| * #125 [AI] Sync logic|                                     |
|                       | ----------------------------------- |
|                       | [4] Related Files                   |
|                       | - src/main.rs                       |
|                       | - src/tui/mod.rs                    |
+-----------------------+-------------------------------------+
| [5] Key Help / Status Line (s: sync, f: filter, q: quit)    |
+-------------------------------------------------------------+

```

## 3. 各コンポーネント詳細

### [2] Task List（タスク一覧）

* **環境依存を避けたステータス表示**:
* Nerd Fonts等のデファクトスタンダード（`[ ]`, `[x]`, `[-]`）または、シンプルに色分けされたASCII記号を使用。
* 割り当て済みタスクには `*` や `[@]` のようなプレフィックスを付け、誰（人間かAIか）が持っているかを明示。

* **インタラクティブ・フィルタリング**:
* `f` キーでファジー検索（`command-palette` 風）を起動。
* タスク名、ID、タグで動的にリストを絞り込み。

### [3] Task Detail（詳細表示）

* **Markdown Rendering**:
* `termimad` や `lowdown` 等のクレートを使用し、ターミナル上で読みやすいMarkdown描画を実現。

* **Context Display**:
* タスクに紐づく「関連ファイル」をセクションとして分離。

## 4. 操作系（Vim-like Keybindings）

Neovimプラグイン化を想定し、Vimユーザーが「指」で覚えられる配置にします。

| Key | Action | Description |
| --- | --- | --- |
| `j` / `k` | 上下移動 | タスクリストの選択移動 |
| `h` / `l` | ペイン切り替え | リスト ↔ 詳細表示（またはタブ切り替え） |
| `Enter` | Edit / Focus | タスクの詳細をフル画面で開く、または編集 |
| `a` | Add | 新規タスク作成（ポップアップ入力） |
| `d` | Done / Close | タスクを完了状態にする |
| `c` | Claim / Unclaim | 自分がタスクを担当する（AIの場合はコマンドから実行） |
| `s` | Sync | `git pull` + `lissue sync` の実行 |
| `/` or `f` | Find | ファジー検索モード起動 |
| `q` / `Esc` | Quit / Back | 前の画面に戻る、または終了 |

## 5. 技術選定

* **Core Framework**: `ratatui`
* **Event Handling**: `crossterm` (クロスプラットフォーム対応)
* **Fuzzy Search**: `fuzzy-matcher`
* **Markdown**: `termimad` (Rust製の柔軟なMarkdownレンダラー)
* **State Management**: 既存の `lissue` Domain/Usecase層をそのまま利用し、TUIはPresentation層として実装。

## 6. Neovimプラグインへの布石

* **headless mode**: TUI自体が独立して動くだけでなく、JSON形式で状態を標準出力できるモードを維持。
* **Remote Control**: Neovim側からRPC（またはシンプルなCLI呼び出し）経由で `lissue` を操作し、Neovimのフローティングウィンドウに描画する設計を意識。

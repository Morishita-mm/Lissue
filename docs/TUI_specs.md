# lissue TUI 仕様書 (v1.1)

## 1. コンセプト

* **Lazygit-inspired**: 思考のスピードを止めない、直感的なペイン移動とキー操作。
* **Vim-like Experience**: `/` キーによる検索、`j/k` による移動など、開発者に馴染みのある操作体系。
* **Zero-Conf Navigation**: プロジェクトルートを自動探索し、どこからでも即座に起動。

## 2. 画面構成 (レイアウト)

ターミナル全体を 5 つの主要領域に分割。

1.  **Status/Tabs (Top)**: 現在のフィルタ状態（Open, InProgress, Pending, Close）を表示。`h/l` で切り替え。
2.  **Left Pane (Main)**: 
    *   **Task List**: タスクの一覧を表示。
    *   **File Selector (Shift-A)**: プロジェクト内のファイルを一覧表示し、`Space` で関連付けをトグル。
3.  **Right Top Pane**: 選択中のタスクの Description を Markdown レンダリング。タイトルは枠線部分に表示。
4.  **Right Bottom Pane**: 関連付けられたファイルパスの一覧。
5.  **Help/Notification Bar (Bottom)**: 利用可能なキーガイド、または一時的なエラー/通知メッセージを表示。

## 3. 入力モード (InputMode)

*   **Normal**: 基本の閲覧・ナビゲーションモード。
*   **Add**: タスク新規追加。中央ポップアップでタイトルを入力。
*   **Search**: `/` または `?` で開始。リアルタイムでタスクまたはファイルを絞り込み。
*   **FileSelect (Shift-A)**: インタラクティブなファイル関連付けモード。

## 4. キーバインド

| キー | アクション | モード |
| :--- | :--- | :--- |
| `q`, `Esc` | 終了 / モード解除 | 共通 |
| `j`, `k` | 上下移動 | Normal / FileSelect |
| `h`, `l` | タブ（ステータス）切り替え | Normal |
| `/`, `?` | 検索開始 | Normal / FileSelect |
| `a` | 新規タスク追加（タイトル入力） | Normal |
| `A` (Shift-A) | ファイル関連付けモード（トグル） | Normal / FileSelect |
| `m` | 詳細編集（エディタ起動） | Normal |
| `d` | タスクを完了にする | Normal |
| `c` | タスクを担当する（Claim） | Normal |
| `s` | 同期（Sync） | Normal |
| `Space`, `Enter` | ファイル関連付けのトグル | FileSelect |

## 5. 特徴的な機能

*   **ハイブリッド更新**: 3秒おきの定期リフレッシュにより、AIエージェントによる背後の更新を検知。
*   **堅牢な描画**: `terminal.clear()` と `Clear` ウィジェットにより、残像や描画崩れを徹底排除。
*   **インライン通知**: エラー発生時もアプリを落とさず、通知バーでユーザーにフィードバック。

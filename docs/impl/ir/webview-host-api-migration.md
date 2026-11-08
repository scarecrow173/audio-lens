---
id: "IMPL-d1uAOCf0thYcE1WXtRkoBZ"
type: "impl"
status: "completed"
title: "al-gui webview host の driver-neutral API 移行 実装記録"
created: "2026-11-08"
updated: "2026-11-08"
owners: []
relations:
  source: []
  changes:
    added: []
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/tasks/webview-host-api-migration.md"
  implemented-by: []
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related: []
  refines: []
  refined-by: []
  derives-from: []
  derived-by: []
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
metadata:
  experiments:
    adopted: []
    rejected: []
---

## 実装記録

### 変更内容

- `crates/al-gui/src/bin/al-gui.rs`: legacy single-window API
  (`ruviform_ui_webview::{HostConfig, run}` + `host.eval(
  dispatch_script(..))`)を削除し、driver-neutral 契約
  `run_multi(&SessionConfig, assets, registry, on_event)` へ移植。
  event routing と close/job policy は `al_gui::DesktopSession`
  (`al-desktop` GPUI と同一実装)に委譲し、binary は
  composition root に徹する。`SessionConfig` は旧 `HostConfig`
  既定値を維持(tick 16ms、`data_dir: None`、`devtools =
  cfg!(debug_assertions)`、単一 window title "AudioLens"、
  `rvf://app/index.html`)。
- 文書チェーン: `docs/{specs,designs,plans,tasks,test-specs}/
  webview-host-api-migration.md` と本記録。managed index を
  `doc_maintenance.js apply` で再生成。

### root cause(調査結果)

- CI run 42431214830(sha `65160914`)の `webview-verify` 3 OS 全てが
  `error[E0432]: unresolved imports ruviform_ui_webview::HostConfig,
  ruviform_ui_webview::run`(`crates/al-gui/src/bin/al-gui.rs:11`)で
  `cargo check` 失敗。
- `al-gui.rs` は ruviform `f3f474b`(legacy `HostConfig`/`run` 存在)
  時点の pin で追加され(a5f683e)、その後 pin が ruviform `38b7a46e`
  (RF#423 / PR #430、`run_multi`/`WindowDriver` 契約への移行で
  legacy API 削除)を越えても AL 側コードが未追従だった。
- 現行 main `0f8a3b0`(pin `863a7a9d`)でも同じ import が残り、
  ローカル `mise run check-ui-webview` で同一 E0432 を再現確認。
- 分類: **AL 側**の未追従コード。ruviform 側の契約問題・CI 環境・
  flaky ではない。ruviform Issue 起票は不要(意図的な API 移行で
  あり AL 側の追従漏れ)。

### 設計上の判断

- `DesktopSession` 再利用: `&mut dyn WindowDriver` 経由の
  driver-neutral session で、canonical mapping と close/job policy
  (close は job を暗黙停止しない、最終 window + job 実行中は
  `Defer`、last-window `exit()`)を実装済み。webview frontend は
  `rvf.ui.command.v1` を発行しないため command 語彙分岐は休眠、
  `job.cancel` op は従来どおり runtime reserved op が処理。
- 挙動差分(意図的): 旧 `handle(CloseRequested)` は全 job cancel +
  即 loop 終了だったが、`DesktopSession` は job を継続させ
  gui-app-architecture.md のライフサイクル契約に一致させる。
  Issue #166 の受入と同一 policy。

### 検証

- `mise run check-ui-webview`: 移行前 E0432 再現 → 移行後 pass。
- `mise run lint-ui-webview` / `test-ui-webview`: pass
  (al-gui webview feature 全テスト、`tests/desktop.rs` の
  FakeDriver lifecycle fixture 22 件を含む)。
- `mise run fmt` / `check` / `lint` / `test` / `build`: pass
  (Windows 11、mise 管理 toolchain)。
- NotRun: 実 OS window での webview 起動・IPC 往復(webview-verify
  lane の契約上 compile/lint/test のみ。必要なら別タスクで手動検証)。

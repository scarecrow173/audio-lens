---
id: "PLAN-HeEdTT7EYY2zZPDtaS0bzN"
type: "plan"
status: "approved"
title: "al-gui webview host の driver-neutral API 移行の実装計画"
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
    - "docs/specs/webview-host-api-migration.md"
  implemented-by: []
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related: []
  refines: []
  refined-by: []
  derives-from:
    - "docs/specs/webview-host-api-migration.md"
    - "docs/designs/webview-host-api-migration.md"
  derived-by: []
  verifies: []
  verified-by:
    - "docs/test-specs/webview-host-api-migration.md"
  references: []
  defers: []
  deferred-by: []
---

## 概要

`al-gui.rs` を ruviform `863a7a9d` の `run_multi`/`SessionConfig`/
`WindowDriver` 契約へ移植し、`DesktopSession` で event routing と
close/job policy を GPUI 側と共有する。pin 更新・ruviform 側変更は
不要(原因は AL 側の未追従コード)。

## タスク

1. **失敗ログと pin 履歴の照合**: run 42431214830 の
   `webview-verify` ログで `error[E0432]` を確認し、pin `3121f8e6`
   → `863a7a9d` の変遷と ruviform `38b7a46e` での legacy API 削除を
   特定。現行 main で `mise run check-ui-webview` を実行し同一
   エラーを再現。→ 済
2. **`al-gui.rs` 移植**: `run_multi` + `SessionConfig`(単一
   `WebViewWindowConfig`、title "AudioLens"、tick 16ms、
   `devtools = cfg!(debug_assertions)`)+ `DesktopSession::handle`
   を callback に接続。→ 済
3. **文書チェーン**: 本 spec/design/plan/task/test-spec/IR を作成。
4. **検証**: `mise run fmt` / `check` / `lint` / `test` /
   `build` と webview lane 3 本を実行。→ 進行中
5. **統合**: `git fetch origin main` → merge → 再生成物の再生成・
   再検証 → commit/push/PR。CI `webview-verify` の結果を確認。

## リスクと対応

- **挙動差分**: 旧 `handle(CloseRequested)` は全 job を cancel して
  いたが、`DesktopSession` は cancel しない(最終 window + job 実行中
  は `Defer`)。これは gui-app-architecture.md のライフサイクル契約
  どおりの意図された差分であり、Issue #166 の受入と同一 policy。
  webview frontend の `job.cancel` op 経路は不変。
- **実窓未検証**: CI lane は compile/lint/test のみ。実 OS window での
  起動・操作は NotRun として PR に明記する(必要なら別タスクで
  手動検証)。
- **`DesktopSession` の webview での休眠分岐**: `rvf.ui.command.v1`
  select/cancel は webview frontend が発行しないため休眠。発行された
  場合も session の canonical 処理がそのまま適用される。

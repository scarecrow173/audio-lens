---
id: "PLAN-034U1kiOgFRwvauxIyObiC"
type: "plan"
status: "approved"
title: "ruviform pin 54439fdc 追従と Spectrogram intensity 実窓検証の計画"
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
    - "docs/specs/ruviform-463-spectrogram-intensity.md"
  implemented-by: []
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related: []
  refines: []
  refined-by: []
  derives-from:
    - "docs/specs/ruviform-463-spectrogram-intensity.md"
    - "docs/designs/ruviform-463-spectrogram-intensity.md"
  derived-by: []
  verifies: []
  verified-by:
    - "docs/test-specs/ruviform-463-spectrogram-intensity.md"
  references: []
  defers: []
  deferred-by: []
---

# ruviform pin 54439fdc 追従と Spectrogram intensity 実窓検証の計画

## 概要

ruviform pin を `863a7a9d` → `54439fdc` へ更新し、RF#463 / PR #466
(Spectrogram cell の intensity alpha 累積飽和を surface_bg 事前合成で
解消)を consumer として取り込む。本計画では pin 追従・実窓検証・
文書化に限定し、AL 側のコード変更は行わない。

## タスク

1. **pin 更新**: `node tools/update-ruviform-rev.mjs
   54439fdc7113b99fb551d370a90c1fc3addcb7a0` で `Cargo.lock` を更新し、
   `mise run sync-ui-deps` で `ui/vendor` を同期。
   → 済(本ブランチで実施済み)
2. **文書チェーン**: 本 spec/design/plan/task/test-spec/IR を作成。
3. **検証**: `mise run` の fmt/check/lint/test/build/
   check-cli-headless/check-ui/test-ui/check-ui-gpui/lint-ui-gpui/
   test-ui-gpui/check-ruviform-lock/check-ruviform-rev/sync-ui-deps を
   実行。`spectrogram_tab_paint`・`gpui_parity` の pass を確認。
4. **実窓検証**: debug build `target/debug/al-desktop.exe` を
   base-dir=`al166-acceptance-verify/jobs`・job=`JOB-Zt7KXJqz7qf8fo9JBcmlX1`
   で起動し、Dataset workspace の Spectrogram tab を pointer 選択。
   capture + pixel 解析で intensity 勾配(中間色・複数色)を確認し、
   証拠を `.devin/evidence/` に保存。
5. **統合**: `git fetch origin main` → merge → 再検証 → commit/push/PR
   (`Refs scarecrow173/ruviform#463`)。

## リスクと対応

- pin 差分は `863a7a9d..54439fdc` = RF#463 / PR #466 の 1 commit のみ
  (確認済み)で、他の ruviform 変更は混入しない。
- capture が断続的に単色 frame を返す DComp artifact の可能性
  (al179/al180 で観測)→ 複数回 capture し、`Responding=True` と
  実描画 frame を記録する。
- fixture job の dataset は既存 evidence 配下の jobs ディレクトリに
  依存する → base-dir を明示して再現性を確保する。

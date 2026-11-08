---
id: "PLAN-034TX99ZbG4qjCPUI15GsW"
type: "plan"
status: "approved"
title: "Spectrogram tab hang 解消の実装計画"
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
    - "docs/specs/spectrogram-tab-hang.md"
  implemented-by: []
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related: []
  refines: []
  refined-by: []
  derives-from:
    - "docs/specs/spectrogram-tab-hang.md"
    - "docs/designs/spectrogram-tab-hang.md"
  derived-by: []
  verifies: []
  verified-by:
    - "docs/test-specs/spectrogram-tab-hang.md"
  references: []
  defers: []
  deferred-by: []
---

## 概要

ruviform pin を `a5b1db1` → `863a7a9d` へ更新し、Spectrogram tab 選択の
hard hang(canvas 0 サイズ起因)と残課題だった primitive O(n²) paint
遅延(RF#461 / PR #462 で ruviform 側解消済み)を同時に取り込む。
本計画では pin 追従・回帰テスト・文書化に限定する。

## タスク

1. **pin 更新**: `node tools/update-ruviform-rev.mjs 863a7a9d` で
   `Cargo.lock` を更新し、`mise run sync-ui-deps` で `ui/vendor` を同期。
   → 済(本ブランチで実施済み。当初 `129786ba` で作業後、RF#461 merge
   commit `863a7a9d` へ再追従)
2. **回帰テスト**: `gpui_parity` の workspace tab selection テストに
   spectrogram drawn assertion を追加。`spectrogram_tab_paint` テストを
   新設し、本番解像度 512×128=65,536 cell の Tabs で tab 選択→settle→
   drawn plan を検証。→ 済
3. **文書チェーン**: 本 spec/design/plan/task/test-spec/IR を作成。
4. **検証**: `mise run` の fmt/check/lint/test/check-cli-headless/
   check-ruviform-lock/check-ruviform-rev/sync-ui-deps/check-ui/
   test-ui/check-ui-gpui/lint-ui-gpui/test-ui-gpui を実行。
   実 OS window(debug `al-desktop`)で Spectrogram tab pointer 選択の
   完了証拠を取得。→ 実施済み(下記リスク節参照)
5. **統合**: `git fetch origin main` → merge → 再検証 → commit/push/PR。

## リスクと対応

- ~~debug build の paint 遅延(約 12 秒/回)~~ → ruviform RF#461 / PR #462
  の `paint_layer` batch 化で解消。pin `863a7a9d` では headless で
  65,536 cells の paint が数十 ms、`test-ui-gpui` 全体が約 0.5 秒で完走。
- pin 退行でテストが hang する可能性は `settle` のタイムアウトではなく
  実際の停止であり、CI では gpui-test lane のタイムアウトで検出される。
- 実機検証メモ: `CopyFromScreen` capture が断続的にほぼ単色 `#2b5f9e`
  の frame を返すことがあるが、プロセスは `Responding=True` で次の
  pointer 操作で実描画へ復帰する。GPUI present/DComp 経路の capture
  artifact の可能性として AL#179 調査へ記録する。

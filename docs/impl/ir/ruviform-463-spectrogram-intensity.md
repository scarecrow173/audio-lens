---
id: "IMPL-034U1KCagcyUJtf8NCqG7M"
type: "impl"
status: "completed"
title: "ruviform pin 54439fdc 追従と Spectrogram intensity 実窓検証の実装記録"
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
    - "docs/tasks/ruviform-463-spectrogram-intensity.md"
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

# ruviform pin 54439fdc 追従と Spectrogram intensity 実窓検証の実装記録

## Summary

ruviform pin を `863a7a9d` → `54439fdc7113b99fb551d370a90c1fc3addcb7a0`
(RF#463 / PR #466: Spectrogram cell の intensity alpha 累積飽和を
surface_bg 事前合成で解消)へ追従し、Dataset workspace Spectrogram
tab の実窓 pixel 解析で intensity 勾配の回復を確認した。AL 側の
コード変更はなく、`Cargo.lock`・`ui/vendor`(gitignore 済み生成物)・
文書チェーン・検証証拠のみ。

## Context

`docs/tasks/ruviform-463-spectrogram-intensity.md` /
`docs/plans/ruviform-463-spectrogram-intensity.md` に基づく
consumer 側後続検証。AL#179 の実窓検証で Spectrogram canvas が
`viz.waveform` 系 2 色に飽和する症状が観測され ruviform#463 として
起票・ruviform 側で修正済み。本タスクは pin 追従と実窓最終確認。

## Implementation

### Changes

- `Cargo.lock`: `tools/update-ruviform-rev.mjs
  54439fdc7113b99fb551d370a90c1fc3addcb7a0` で ruviform 依存 20
  package の source fragment を一括更新。
- `ui/vendor/`: `mise run sync-ui-deps` で `54439fdc71` に再生成
  (gitignore 済み生成物)。
- `docs/`: spec/design/plan/task/test-spec/IR の文書チェーンと
  各 README index。

### Decisions During Implementation

- 実窓検証は al179/al180 の既存手法を再利用し、fixture
  `JOB-Zt7KXJqz7qf8fo9JBcmlX1`(al166-acceptance-verify/jobs 配下)
  で再現性を確保。window は `raise.ps1`(SetWindowPos TOPMOST→
  NOTOPMOST)で最前面化してから capture(フォアグラウンド奪取不可の
  環境でも z-order を上げられる既存手法)。
- pixel 解析は新規 `spectro-intensity.py`(検証専用、証拠 dir 配下)
  で、領域 (8,140)-(1288,835) の exact RGB / blue-family 色数・
  luminance 分布・旧飽和ペア占有率を計測。before 側は al179 の
  `42-spectro-click-after.png` を同コードで再解析して比較。
- 検証用 binary は `cargo build -p al-gui --features gpui-test --bin
  al-desktop`(canonical = mise。bin 専用の mise task はなく、
  `test-ui-gpui` と feature 統一して compile を共有する補助証拠)。

## Related Experiments

### Adopted

- なし(実験 log は作成していない)。

### Rejected

- なし。

## Validation

実測 pass(全て Windows 11、rustc/cargo 1.99.0 mise pin):

- `mise run fmt` / `check` / `lint` / `test` / `build` /
  `check-cli-headless` / `check-ui` / `test-ui` /
  `check-ui-gpui` / `lint-ui-gpui` / `test-ui-gpui` /
  `check-ruviform-lock` / `check-ruviform-rev` / `sync-ui-deps`:
  全て pass。`test-ui-gpui` で `spectrogram_tab_paint`
  (`select_spectrogram_tab_paint_completes`)と `gpui_parity` を含む
  全テスト pass。
- 実窓 pixel 解析(pin `54439fdc`): Spectrogram canvas 領域で
  exact RGB 269 色・blue-family 179 色・blue luminance 56-164、
  旧飽和ペア `#4A90D9`/`#498FD8` は blue pixel の 0.2%。
  before(pin `863a7a9d`、al179 capture を同コードで再解析)は
  blue-family 17 色・飽和ペア 100.0%。証拠:
  `F:/repositry/audio-products/.devin/evidence/al-pin463-verify/`
  (verdict.md・PNG・解析出力)。
- 回帰: Source tab は al179 修正後と同一の色分布
  (`#2B5F9E` x95,499 / `#4A90D9` x46,602)、Analysis tab も応答。
  Spectrogram tab 選択で `Responding=True`・CPU 安定(AL#180 hang
  なし)。

## Risks

- なし(残件なし)。capture の DComp artifact は複数回 capture の
  統計一致で除外済み。

## Follow-ups

- なし。ruviform#463 の consumer 側受入は完了。PR 本文に
  `Refs scarecrow173/ruviform#463` を記載する。

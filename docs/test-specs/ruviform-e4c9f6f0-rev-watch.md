---
id: "TSPEC-7WymGMM6PsfLStbZsmxnIw"
type: "test-spec"
status: "approved"
title: "ruviform e4c9f6f0 pin 追従の検証仕様"
created: "2026-11-18"
updated: "2026-11-18"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/audio-lens/issues/127"
  changes:
    added: []
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/specs/ruviform-e4c9f6f0-rev-watch.md"
  implemented-by: []
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/test-specs/ruviform-b6d0cc6a-rev-watch.md"
  derives-from:
    - "docs/specs/ruviform-e4c9f6f0-rev-watch.md"
  derived-by: []
  verifies:
    - "docs/specs/ruviform-e4c9f6f0-rev-watch.md"
  verified-by: []
---

# ruviform e4c9f6f0 pin 追従の検証仕様

## Feature: pin 一貫性

- Rule: `Cargo.lock` の全 `ruviform-*` git source が単一 resolved SHA
  `e4c9f6f0715da3ac479f8c437d21a021327e51da` に一致する。
- Rule: `ui/vendor/ruviform/REV` が lockfile の resolved SHA と一致する
  （`sync-ui-deps` 生成物、commit 対象外）。
- Rule: `Cargo.lock` の差分は pin bump 手順由来の source fragment
  更新のみで、新規 package（audio-device/midi-device 等）の混入がない。
- 検証: `mise run check-ruviform-lock`、`mise run check-ruviform-rev`、
  `mise run check-ui`。

## Feature: 互換性（OS-L1）

- Rule: 既定 feature の `check`/`lint`/`test al-gui` が非回帰
  — plugin-core/host の additive trait method（default 実装付き）に
  AL 側コード変更が不要であること。
- Rule: `gpui-test` feature の check/lint/test と `gpui` feature の
  native build（`al-desktop`）が非回帰。
- Rule: `webview` feature の check/lint/test が非回帰
  （`ruviform-ui-webview` の pin 追随を含む）。
- Rule: `check-cli-headless` で CLI 依存木の GUI/GPU 非含有を維持 —
  upstream audio-test の新規 dev-dep が consumer 依存解決に波及しない。
- Rule: `test al-analysis` で standing oracle 回帰（rf628_oracle 含む
  f64/長尺/PTS/gap/Validity）。
- Rule: `mise run fmt` と `prek run --all-files` が合格。

## Feature: scenario contract

- Rule: 既存 `SemanticGuiTest`/`ScenarioReport` scenario
  （desktop.jobs / source.seek / dataset.workspace）が新 pin で
  同一契約を維持する。
- Rule: `al-gui-real-scenario --blocked-probe` が catalog 全件
  `NotRun`+`environment_blocked` の fail-closed 形状を維持する
  （`environment_kind` が `real-window` を主張しない）。
- Rule: 実機 Pass は interactive session のみ — headless/semantic の
  Pass を実機 Pass に昇格しない。

## Feature: upstream covering CI

- Rule: candidate 採用判断前に upstream の ci/daily-ci/
  consumer-verify run を確認し、無ければ NotRun と記録する
  （`e4c9f6f0` は run なし — merge 後 AL 対象 SHA の CI 証拠が
  統合検証の正本）。

---
id: "IMPL-034Y1A2b3C4d5E6f7G8h9"
type: "impl"
status: "completed"
title: "ruviform pin abb9f372 追従 + consumer oracle 実装記録"
created: "2026-11-15"
updated: "2026-11-15"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/audio-lens/issues/127"
  changes:
    added:
      - type: "file"
        path: "docs/impl/ir/ruviform-abb9f372-rev-watch.md"
    modified:
      - type: "file"
        path: "Cargo.lock"
      - type: "file"
        path: "Cargo.toml"
      - type: "file"
        path: "crates/al-analysis/Cargo.toml"
    deleted: []
    renamed: []
    moved: []
    generated:
      - type: "file"
        path: "ui/vendor/ruviform"
  implements:
    - "docs/specs/ruviform-abb9f372-rev-watch.md"
  implemented-by: []
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/impl/ir/ruviform-ac10a2f7-rev-watch.md"
    - "docs/impl/ir/issue-127-rf628-consumer-oracle.md"
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

# ruviform pin abb9f372 追従 + consumer oracle 実装記録

## Decisions

- candidate SHA: `abb9f372d3ee95ee824b3c4f7dc68e8083ce2774`
  （ruviform main HEAD、`git ls-remote` + local clone fetch で
  再解決。`crates/ruviform-audio-test/src/pipeline/` 存在確認済み）。
- 採用判断: **採用**。AL 依存木内の差分は additive のみ
  （`ruviform-audio-core::midi_output`、`ruviform-ui-test`
  scenario catalog、`ruviform-ui-gpui` native-slot lane）。
  `ruviform-audio-test` は dev-dep 限定で `pcm`/`oracle` のみ
  消費し、`pipeline`/`backend`/`midi`/`clock` は未使用
  （AL-4 Deferred・AL-5 NotApplicable）。
- lockfile ruviform package: 20 → 26（`ruviform-audio-test` と
  推移的依存 `ruviform-audio`/`ruviform-graph`/`ruviform-rt`/
  `ruviform-plugin-*`/`ruviform-io-symphonia`/`symphonia`/
  `shared_memory` 等）。全 26 package が単一 SHA に解決
  （`check-ruviform-lock`/`check-ruviform-rev` Pass）。
- `ruviform-audio-core` を `al-analysis` dev-deps に追加
  （`PcmFixture::new` の `SampleRate` 型構築に必要）。

## Code Changes

- `Cargo.toml`: `[workspace.dependencies]` に
  `ruviform-audio-test` git dep 追加（`rev` なし）。
- `crates/al-analysis/Cargo.toml`: `[dev-dependencies]` に
  `ruviform-audio-test.workspace = true` と
  `ruviform-audio-core.workspace = true` 追加。
- `Cargo.lock`: `tools/update-ruviform-rev.mjs` + cargo による
  再生成（手編集なし）。
- `crates/al-analysis/tests/rf628_oracle.rs`: 新規（7 test）。

## Evidence

- `cargo test -p al-analysis --test rf628_oracle`: 7/7 Pass
  （Windows debug、`rf628_long_scenario_non_regression` は
  ~85s、他は ~2s）。
- `node tools/ruviform-lock-rev.mjs` / `check-ruviform-rev.mjs`:
  26 package 単一 SHA Pass。
- `mise run sync-ui-deps`: `ui/vendor/ruviform/REV` =
  `abb9f372...`（commit 対象外の生成物）。
- 残り lane（fmt/check/lint/test/headless/prek）は本 slice の
  task doc・PR 本文に実測値を記録する。

## Rollback

`git checkout -- Cargo.lock` → manifest から
`ruviform-audio-test`/`ruviform-audio-core` dev-dep 削除 →
`node tools/update-ruviform-rev.mjs ac10a2f7761c08db73aebd861c0a5c1f53f269b7`
→ `mise run sync-ui-deps` で旧 pin へ復帰。製品コード差分なし。

## Unresolved / Limitations

- AL-4 clock legs: Deferred（live clock consumer 不在）。
- AL-5 device-pipeline legs: NotApplicable（device boundary
  非消費）。`pipeline/`/`backend::*`/`midi::*` は将来の
  device consumer slice で再評価。
- spectral 系の gap 縮退（`argmax` `total_cmp` NaN 規則）は
  既存意味論として観測 pin のみ。改善は別 Issue の判断。
- 3OS CI・cross-consumer・実機検証は merge 後対象 SHA の CI
  証拠（本 slice 範囲外）。

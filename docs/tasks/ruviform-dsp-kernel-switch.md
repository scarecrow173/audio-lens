---
id: "TASK-034P3OOFhUzNrxllVL2POQ"
type: "task"
status: "todo"
title: "f64 DSP 呼出の Ruviform 切替(AL#101)"
created: "2026-10-30"
updated: "2026-10-30"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/audio-lens/issues/101"
  changes:
    added: []
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/plans/ruviform-foundation-migration.md"
  implemented-by: []
  depends-on:
    - "docs/plans/ruviform-foundation-migration.md"
    - "docs/tasks/ruviform-migration-baseline.md"
    - "docs/tasks/ruviform-dependency-ci.md"
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
---

# f64 DSP 呼出の Ruviform 切替(AL#101)

## Work

`al-locate::dsp`・`al-analysis::stft` 等のカーネル呼出先を
`al-math` 自前実装から Ruviform f64 API(#150 基盤・#152 相関・
#153 帯域/STFT)へ切り替える。Issue
[#101](https://github.com/scarecrow173/audio-lens/issues/101) を正本とする。

- `al-locate::dsp` の facade 呼出先を Ruviform へ差替え
  (deinterleave 等の glue は Ruviform buffer/layout API へ寄せる)
- `dsp.rs::conv`(overlap-save convolution、plan/workspace を持つ
  実カーネル)は汎用 DSP として Ruviform 側(#150 系の f64
  plan/workspace または #152)へ配置し、`fft` thin wrapper は
  Ruviform f64 FFT へ置換、`deinterleave` glue は AL に残す
  (責務マップ「境界判定記録」)
- `al-analysis` の STFT/CSD/coherence・帯域演算を Ruviform
  カーネルへ差替え
- f64 契約・元 PCM 尺度を保持し、暗黙の f64→f32 変換を入れない
- plan/workspace 共有境界(run pool 等)を Ruviform plan 契約へ
  再配線

## Classification

- `normal-plan-task`

## Upstream Documents

- `docs/plans/ruviform-foundation-migration.md`
- `docs/designs/ruviform-migration-responsibility-map.md` §1/§3/§4
- Issue [#101](https://github.com/scarecrow173/audio-lens/issues/101)

## Dependencies

- `docs/tasks/ruviform-migration-baseline.md`
- `docs/tasks/ruviform-dependency-ci.md`
- 外部依存: Ruviform [#150](https://github.com/scarecrow173/ruviform/issues/150)
  (f64 基盤)、[#152](https://github.com/scarecrow173/ruviform/issues/152)
  (相関)、[#153](https://github.com/scarecrow173/ruviform/issues/153)
  (帯域/STFT)

## Affected Files

- `crates/al-locate/src/dsp.rs`(呼出先差替え、glue 整理)
- `crates/al-analysis/src/stft.rs`・`band_gain.rs`(カーネル差替え)
- `crates/al-locate/src/pool.rs` 等 plan/workspace 共有箇所
- facade 迂回の `al_math::` 直接 import 箇所:
  `crates/al-locate/src/pool.rs`・`prepared.rs`・`normalize.rs`・
  `mixer.rs`、および `crates/al-locate/src/lib.rs` の公開
  re-export `al_math::linalg::{condition_number, least_squares}`
  (Pre-release 規約で API 変更可・成果物は再生成前提)
- `crates/al-math`(切替後は未使用化・撤去は AL#104)

## Done When

- [ ] f64 精度と数値意味論が維持され、#101 の完了条件を満たす
- [ ] #98 baseline との golden/性能比較記録がある
- [ ] AL 側型が Ruviform 公開 API に混入していない

## Verification

- [ ] 同 fixture/形状で旧実装との golden・差分・性能比較
- [ ] `mise run test` 相当が通る

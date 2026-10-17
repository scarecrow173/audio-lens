---
id: "TASK-034HiHYbV7buE9EkLxWNbB"
type: "task"
status: "done"
title: "shape-adaptive NCC dispatch 実装 (issue #71 P3)"
created: "2026-10-17"
updated: "2026-10-17"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/audio-lens/issues/71"
  changes:
    added: []
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "../designs/adaptive-ncc-dispatch-issue-71-p3.md"
  implemented-by: []
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "./valid-fft-ncc-plan-issue-71-p1.md"
    - "./streaming-bandpass-issue-71-p2.md"
  refines: []
  refined-by: []
  derives-from:
    - "../designs/adaptive-ncc-dispatch-issue-71-p3.md"
    - "../specs/adaptive-ncc-dispatch-issue-71-p3.md"
    - "../adr/ncc-plan-time-dispatch-issue-71-p3.md"
  derived-by: []
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# shape-adaptive NCC dispatch 実装 (issue #71 P3)

## Work

`CorrelationPlan::new_ncc` に直接法経路(`PlanMode::NccDirect`)を
追加する。`K = signal_len − template_len` の小さい形状で分子を
axpy 蓄積(`acc[k] += template[i]·signal[k+i]`)で計算し、
正規化は共通パイプに合流させる。direct plan の workspace は
FFT バッファを確保しない。

## Classification

- `normal-plan-task`

## Upstream Documents

- `docs/specs/adaptive-ncc-dispatch-issue-71-p3.md`
- `docs/adr/ncc-plan-time-dispatch-issue-71-p3.md`
- `docs/designs/adaptive-ncc-dispatch-issue-71-p3.md`

## Dependencies

- P1(`PlanMode` 分離)と P2 がマージ済みであること

## Affected Files

- `crates/al-math/src/correlation.rs` — `PlanMode::NccDirect`、
  `new_ncc` の mode 選択、`Correlation::run` の direct 分岐、
  `CorrelationWorkspace::new` の条件付き確保
- `crates/al-math/src/vec.rs` — 必要なら axpy 蓄積用ヘルパ
  (既存 `axpy` をそのまま使えるか確認)
- `crates/al-math/tests/` — mode 選択 pin・経路一致・
  scalar↔dispatch テスト

## Done When

- [x] `new_ncc` がコストモデル `L·(K+1) ≤ C·n·log2(n)` で
      `NccDirect`/`NccValid` を選択し、6 代表形状で実測最速側と一致
- [x] `NccDirect` plan の `ncc` が `NccValid` plan と全有効ラグ一致
      (argmax・tie 含む、誤差 ~1e-13 未満)
- [x] `template_energy==0` early-zero・無音窓・max_lag clamp・
      空 shape が direct でも同一
- [x] `gcc_phat` が direct plan で panic
- [x] direct plan の workspace が FFT バッファを確保しない
- [x] `pool`・`dsp::ncc`・`band_gain` が無変更で動作

## Verification

- [x] `cargo test -p al-math -p al-locate -p al-analysis` 全 pass
- [x] `cargo clippy --workspace --all-targets` clean
- [x] `node .agents/skills/doc-status/scripts/audit_docs.js --type all`
- [x] 実サンプル run で `locate::band_ncc`/`full_ncc` busy と
      メモリを before/after 記録し discovery へ反映
      (band_ncc −84%、full_ncc −90%、JOB-kaXkVSPpXfhPKlVdUTCDip)

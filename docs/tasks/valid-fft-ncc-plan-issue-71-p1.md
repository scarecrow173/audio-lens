---
id: "TASK-034H5pLyx9R2CtnfPe4b03"
type: "task"
status: "todo"
title: "Valid FFT NCC plan 実装 (issue #71 P1)"
created: "2026-10-16"
updated: "2026-10-17"
owners: []
relations:
  source:
    - "../designs/valid-fft-ncc-plan-issue-71-p1.md"
    - "https://github.com/scarecrow173/audio-lens/issues/71"
  changes:
    added: []
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "../designs/valid-fft-ncc-plan-issue-71-p1.md"
  implemented-by: []
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related: []
  refines: []
  refined-by: []
  derives-from:
    - "../designs/valid-fft-ncc-plan-issue-71-p1.md"
    - "../specs/valid-fft-ncc-plan-issue-71-p1.md"
    - "../adr/ncc-valid-fft-plan-gcc-phat-plan.md"
  derived-by: []
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# Valid FFT NCC plan 実装 (issue #71 P1)

## Work

`CorrelationPlan` に NCC-valid コンストラクタ(FFT 長 `nextpow2(S)`)を
追加し、NCC 経路全体を切り替える。`Correlation::run`・workspace・
SIMD カーネルは変更しない。

## Classification

- `normal-plan-task`

## Upstream Documents

- `docs/specs/valid-fft-ncc-plan-issue-71-p1.md`
- `docs/adr/ncc-valid-fft-plan-gcc-phat-plan.md`
- `docs/designs/valid-fft-ncc-plan-issue-71-p1.md`

## Dependencies

- なし(#40 の CorrelationPool・#52 の plan 分離はマージ済み)

## Affected Files

- `crates/al-math/src/correlation.rs` — mode フィールド、NCC-valid
  コンストラクタ、`gcc_phat` の mode 検査、フリー関数 `ncc` の切替
- `crates/al-locate/src/pool.rs` — pool キーへの mode 追加
- `crates/al-analysis/src/band_gain.rs` — `windows` の plan 切替
- `crates/al-analysis/src/cheap.rs` — `ipd_lags` の plan 切替
- `crates/al-math/tests/ncc_workspace.rs` / 各クレートのテスト —
  回帰・mode 分離・panic テスト

## Done When

- [ ] `new_ncc`(仮称)で `nextpow2(S)` の plan が生成され、全有効ラグの
      出力が従来 plan と既存許容基準内で一致
- [ ] `new_ncc` 由来 plan での `gcc_phat` が panic
- [ ] pool で同一 shape の ncc/phat が別 plan になり、`dsp` フリー関数と
      pool 結果が一致
- [ ] `dsp::ncc` / `band_gain` / `ipd_lags` が valid plan を使用
- [ ] `dsp::gcc_phat` / `pool.gcc_phat` が従来値を維持

## Verification

- [ ] `cargo test -p al-math` / `-p al-locate` / `-p al-analysis` 全 pass
- [ ] `cargo clippy --workspace --all-targets` clean
- [ ] `node scripts/audit_docs.js` clean
- [ ] 実サンプル run で band_ncc / band_gain busy 改善を確認・記録

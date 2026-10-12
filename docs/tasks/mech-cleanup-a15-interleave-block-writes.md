---
id: "TASK-034EO4rKpEFHb4NpDpz5py"
type: "task"
status: "todo"
title: "機械的 caller クリーンアップ A15: interleave/deinterleave のブロック書き込み化"
created: "2026-10-12"
updated: "2026-10-12"
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
    - "docs/plans/mechanical-caller-cleanup.md"
  implemented-by: []
  depends-on:
    - "docs/plans/mechanical-caller-cleanup.md"
    - "docs/tasks/mech-cleanup-a9-a12-cheap-pass-consolidation.md"
  blocks:
    - "docs/tasks/mech-cleanup-a16-row-major-fast-paths.md"
  supersedes: []
  superseded-by: []
  related: []
  refines: []
  refined-by: []
  derives-from:
    - "docs/designs/mechanical-caller-cleanup.md"
  derived-by: []
  verifies: []
  verified-by:
    - "docs/test-specs/mechanical-caller-cleanup.md"
  references: []
  defers: []
  deferred-by: []
---

# 機械的 caller クリーンアップ A15: interleave/deinterleave のブロック書き込み化

## Work

plan T5 = design の A15-opt コミット(step 5)。
per-sample の `extend_from_slice`(8B)/`push` を、design
で確定した chunk 粒度(書き込み側 = staged row +
ブロック転送、読み出し側 = channel-block 単位 strided
収集)へ置き換える。frame-major の配置順は両方向で不変
(transpose しない)。

3 サイト(design sketch 通り):

- `crates/al-locate/src/normalize.rs:168-171`
  (interleave → bytes): `let ch = channels.len();`、
  `Vec::with_capacity(frames * ch * 8)` で事前確保し、
  再利用する `row = vec![0u8; ch * 8]` scratch へ 1
  frame 分の channel サンプルを `to_le_bytes` +
  `copy_from_slice` で詰め、frame ごとに 1 回
  `bytes.extend_from_slice(&row)` する。per-sample の
  8B extend(~17M 規模)を per-frame extend へ削減
- `crates/al-analysis/src/reference.rs:1163-1170`
  (residual interleave → `Vec<f64>`): `n * channels`
  を事前確保し `interleaved[i * c + ch_idx] = ch[i]`
  の indexed write(frame-major)
- `crates/al-locate/src/dsp.rs:17-29` (deinterleave):
  `(0..channels).map(|ch| interleaved.chunks_exact(
  channels).map(|frame| frame[ch]).collect())` による
  channel-block strided gather。`chunks_exact` により
  trailing partial frame drop は現行と同一

- **等価の根拠**: 配置のみの変更であり値は変わらない。
  バイト列・要素列が完全一致すれば bit-identical
- 等価性ゲート: T1 の characterization(normalized
  `.f64le` バイト列・`deinterleave` 返却列・residual
  interleave 要素列の完全一致)が opt 前後で green
- 再計測: `bench_dsp`(T1 で決定した Instant 区間)を
  再実行し `target/bench-baseline/` と比較、
  before/after を `docs/impl/ir/` へ転記

## Classification

- `normal-plan-task`

## Upstream Documents

- `docs/plans/mechanical-caller-cleanup.md`(T5)
- `docs/designs/mechanical-caller-cleanup.md`(A15 chunk
  粒度決定・3 サイト sketch)
- `docs/specs/mechanical-caller-cleanup.md`(A15 要件)

## Dependencies

- `docs/tasks/mech-cleanup-bench-baseline-characterization.md`
  (T1: A15 characterization + `bench_dsp` 計測経路 +
  baseline)
- `docs/tasks/mech-cleanup-a9-a12-cheap-pass-consolidation.md`
  (T4: `reference.rs`/`cheap.rs` の先行 opt — 直列適用)

## Affected Files

- `crates/al-locate/src/normalize.rs`(`:168-171` staged
  row + 事前確保)
- `crates/al-analysis/src/reference.rs`(`:1163-1170`
  indexed write)
- `crates/al-locate/src/dsp.rs`(`:17-29` channel-block
  gather)
- `docs/impl/ir/<record>.md`(A15 before/after 転記)

## Done When

- [ ] `normalize.rs` が frame 毎 staged row + 1 回
  `extend_from_slice` + `with_capacity` になっている
- [ ] `reference.rs` residual interleave が `n*c` 事前
  確保の indexed write(frame-major)になっている
- [ ] `dsp.rs` deinterleave が `chunks_exact` による
  channel-block strided gather になっている
- [ ] T1 の A15 characterization(`.f64le` バイト列・
  deinterleave 列・residual interleave 列の完全一致)が
  opt 前後両時点で green
- [ ] `bench_dsp` の before/after が IR に記録されている
- [ ] 本コミットが char・A6-opt・A7A8-opt・A9A12-opt
  各コミットより後の git 履歴にある

## Verification

- [ ] `mise exec -- cargo test -p al-locate` が green
  (locate.rs・dsp.rs tests 含む)
- [ ] `mise exec -- cargo test -p al-analysis --test
  reference` が green(residual interleave 列)
- [ ] `mise exec -- cargo run --example bench_dsp
  --release -p al-locate` が完走し baseline との差分を
  確認済み
- [ ] `mise exec -- cargo clippy --workspace --all-targets
  -- -D warnings`・`mise exec -- cargo fmt --all --
  --check` が green
- [ ] `mise exec -- cargo test --workspace` が green
  (golden 含む)

## Out of Scope

- `dsp.rs`/`normalize.rs` の公開シグネチャ変更
- production caller(`decode.rs:125`/`:224`/`:377`)の
  呼出し変更
- A16(T6)の他サイト

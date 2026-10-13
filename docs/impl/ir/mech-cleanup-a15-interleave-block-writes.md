---
id: "IMPL-034EQyd6pL3XswxoyXQgJA"
type: "impl"
status: "completed"
title: "機械的 caller クリーンアップ A15: interleave/deinterleave のブロック書き込み化"
created: "2026-10-12"
updated: "2026-10-12"
owners: []
relations:
  source: []
  changes:
    added: []
    modified:
      - type: "code"
        path: "crates/al-locate/src/normalize.rs"
      - type: "code"
        path: "crates/al-locate/src/dsp.rs"
      - type: "code"
        path: "crates/al-analysis/src/reference.rs"
      - type: "doc"
        path: "docs/tasks/mech-cleanup-a15-interleave-block-writes.md"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/tasks/mech-cleanup-a15-interleave-block-writes.md"
  implemented-by: []
  depends-on:
    - "docs/plans/mechanical-caller-cleanup.md"
    - "docs/impl/ir/mech-cleanup-bench-baseline-characterization.md"
    - "docs/impl/ir/mech-cleanup-a9-a12-cheap-pass-consolidation.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related: []
  refines: []
  refined-by: []
  derives-from:
    - "docs/designs/mechanical-caller-cleanup.md"
    - "docs/test-specs/mechanical-caller-cleanup.md"
    - "docs/specs/mechanical-caller-cleanup.md"
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

# 機械的 caller クリーンアップ A15: interleave/deinterleave のブロック書き込み化

## Summary

plan T5(design step 5 の A15-opt コミット相当)。
interleave/deinterleave 3 サイトの per-sample
`extend_from_slice`(8B)/`push` を design 確定の chunk 粒度
(書き込み側 = staged row + ブロック転送、読み出し側 =
channel-block strided gather)へ置き換えた。frame-major の
配置順は両方向で不変(transpose なし)で、byte/element 列は
完全に一致するため bit-identical。T6(A16)と同一作業
セッションで適用したが、変更ファイルは A15 の 3 サイトと
A16 の 4 サイトで完全に分離されており、T1
characterization は全て green。

## Context

task `docs/tasks/mech-cleanup-a15-interleave-block-writes.md`
(TASK-034EO4rKpEFHb4NpDpz5py)、plan
`docs/plans/mechanical-caller-cleanup.md`(T5)、design
`docs/designs/mechanical-caller-cleanup.md`(A15 chunk 粒度
決定・3 サイト sketch)に従う。baseline・characterization
は T1、先行 opt は T2–T4(直列適用済み、T4 は `cd3688c`)。
ユーザー指示により**本タスクではコミットを作成していない**
(Done When の履歴順序はコミット実施時に担保が必要)。

## Implementation

### Changes

- `crates/al-locate/src/normalize.rs`(旧 `:167-172`):
  per-sample の `bytes.extend_from_slice(&channel[frame]
  .to_le_bytes())`(~frames × ch 回の 8B extend)を、
  `Vec::with_capacity(frames * ch * 8)` 事前確保 + 再利用
  `row = vec![0u8; ch * 8]` scratch への `copy_from_slice`
  充填 + frame ごと 1 回の `bytes.extend_from_slice(&row)`
  へ置き換え(design sketch どおり)
- `crates/al-analysis/src/reference.rs`(旧 `:1165-1173`
  residual interleave): `Vec::with_capacity(n * c)` +
  `v.push(ch[i])` を `vec![0.0; n * c]` 事前確保 +
  `v[i * c + ch_idx] = ch[i]` の indexed write へ置き換え。
  frame-major の配置順は不変
- `crates/al-locate/src/dsp.rs`(`deinterleave`):
  `vec![Vec::with_capacity(frames); channels]` + 入れ子
  push を `(0..channels).map(|ch| interleaved
  .chunks_exact(channels).map(|frame| frame[ch]).collect())
  .collect()` の channel-block strided gather へ置き換え。
  `chunks_exact` により trailing partial frame drop は
  現行と同一、`channels == 0` の early return も維持

### Decisions During Implementation

- 全サイト design sketch の写経。等価の根拠は配置変更のみで
  値を変更しないこと — `.f64le` バイト列・deinterleave
  返却列・residual interleave 要素列の完全一致
  characterization(T1 設置済み)が opt 前後で green
- `deinterleave` の返却は iterator `.collect()` 由来の
  exact-size Vec になるが、`Vec` の容量は観測契約外(値列・
  外側長は一致)

## Related Experiments

### Adopted

<!-- No experiment logs: mechanical task per task-doc. -->

### Rejected

<!-- No experiment logs: mechanical task per task-doc. -->

## Bench 計測結果(post-optimization)

環境: `mise exec -- cargo run --release --example bench_dsp
-p al-locate`。生ログは
`target/bench-baseline/bench_dsp-a15.txt`。baseline は T1
採取の `bench_dsp-pre.txt`(同区間、5 rep mean)。

### Instant — bench_dsp A15 区間

| 計測点 | pre (mean) | post | delta |
| --- | --- | --- | --- |
| deinterleave 2ch × 1M(production 関数) | 7.1 ms | 4.4 ms | −38% |
| deinterleave 2ch × 1M(channel-block gather 候補腕) | 6.0 ms | 5.3 ms | —(不変腕・環境 drift) |
| interleave 2ch × 1M(frame-major loop) | 5.2 ms | 4.7 ms | —(不変腕) |
| interleave 2ch × 1M(indexed write) | 5.2 ms | 5.0 ms | —(不変腕) |
| interleave 2ch × 1M(staged-row extend) | 7.2 ms | 7.1 ms | —(不変腕) |

- `deinterleave …(current)` 腕は production
  `al_locate::dsp::deinterleave` を呼ぶため、post は新実装
  (channel-block gather + `collect`)の実測: 7.1 → 4.4 ms。
  bench 内の候補腕(push 型 gather)より `collect` 版が速い
  のは exact-size 確保の差と考えられる
- interleave 系 3 腕は bench ファイル内の自己完結
  reproduction(`Vec<f64>`)であり production
  `normalize.rs`(u8 バイト列、8B extend → per-frame
  extend)・`reference.rs`(indexed write)を直接計時しない。
  production 側の効果は `bench_reference` / stage 級計測に
  混ざる(T6 IR の累積値を参照)

## Validation

- `mise exec -- cargo test -p al-locate`: green —
  `dsp::tests::a15_deinterleave_element_order_and_edges`・
  `a15_deinterleave_byte_layout_roundtrip`・
  `tests/locate.rs` の
  `a15_normalized_pcm_f64le_bytes_are_exact` を含む全件
- `mise exec -- cargo test -p al-analysis --test
  reference`: green —
  `mechanical_cleanup::a15_residual_interleave_is_element_exact`
  を含む 12 件
- `mise exec -- cargo test --workspace`: green(39 スイート
  全件、golden 含む)
- `mise exec -- cargo clippy --workspace --all-targets --
  -D warnings`: green
- `mise exec -- cargo fmt --all -- --check`: green

## Risks

- Instant 値は本マシンの run。不変腕の drift(6.0→5.3 ms
  等)が示すとおり絶対差より同一 run 内の腕比較を信頼する
  こと。`deinterleave` production 腕の −38% は drift を
  差し引いても一貫した改善
- `normalize.rs` staged-row は `channels.len() == 0` では
  `row` が空で `extend_from_slice(&[])` の no-op となるが、
  旧実装も 0 チャンネルで bytes 空のままのため出力は同一
  (バイト列 0B)。`channels == 0` は `prepare_input` 上流の
  `input.channels` 整合チェックで実質到達不能

## Follow-ups

- T6(A16-opt、同セッションで適用済み): `reference.rs`
  の psd→power 側・dom scratch・video 融合・
  `read_range` fast path
- T7(measurement-ir-closeout): 全 bench の最終比較・
  char→opt 履歴順序の確認(コミット実施時)

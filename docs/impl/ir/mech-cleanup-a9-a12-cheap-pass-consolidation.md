---
id: "IMPL-034EQ29Ki4PdDQzV0LJwRB"
type: "impl"
status: "completed"
title: "機械的 caller クリーンアップ A9/A12: cheap 側 power 一括化・mean_row 融合(A12 は bench 棄却)"
created: "2026-10-12"
updated: "2026-10-12"
owners: []
relations:
  source: []
  changes:
    added: []
    modified:
      - type: "code"
        path: "crates/al-analysis/src/cheap.rs"
      - type: "doc"
        path: "docs/tasks/mech-cleanup-a9-a12-cheap-pass-consolidation.md"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/tasks/mech-cleanup-a9-a12-cheap-pass-consolidation.md"
  implemented-by: []
  depends-on:
    - "docs/plans/mechanical-caller-cleanup.md"
    - "docs/impl/ir/mech-cleanup-bench-baseline-characterization.md"
    - "docs/impl/ir/mech-cleanup-a7-a8-csd-inlining.md"
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

# 機械的 caller クリーンアップ A9/A12: cheap 側 power 一括化・mean_row 融合(A12 は bench 棄却)

## Summary

plan T4(design step 4 の A9A12-opt コミット相当)。
`cheap.rs` で `magnitude()`・`power()`・`psd()` の 3 回の
`mag_sq` pass を `m.power()` 1 pass に一元化し、`mag`・`psd`・
`mean_row` を全て caller 側で導出する形へ置き換えた(A9)。
psd は `v * inv`(`inv = 1.0 / norm`、norm =
`(0.375 * window) * sample_rate`)の逆数乗算のみで、
`magnitude()` の `sqrt`・`psd()` の `scale(…, 1.0/norm, …)`
と同じ演算のため bit-identical。`mean_row` は旧列走査
(`(0..half).map(|b| mag.iter().map(|r| r[b]).sum() / len)`)
と加算対象・加算順(0.0 始まり・row 昇順)・除算が同一の
row-major 累積を `mag` 導出ループへ融合し、`*v /= mag.len()`
を維持。全導出と `m` field 読みは `channel_stft.push(Some(m))`
前に完了し、旧 `:668` の `half` 重複定義は除去した
(=`m.fft_size / 2` を再利用)。

A12(HashSet 事前計数)は **bench 棄却でコード不変更**:
T1 baseline が示したとおり HashSet 候補は両経路で現行を
下回り(in-gate 7.29→14.17 ms、>1M 27.63→55.18 ms)、
early-exit 変種は support(正確な distinct count)契約を
破る。本再計測でも同傾向(14.07 vs 6.44 ms・50.11 vs
26.74 ms)を確認し、仕様上許容される高速化代替がないため
棄却を記録した(詳細は Decisions / Rejected)。

## Context

task `docs/tasks/mech-cleanup-a9-a12-cheap-pass-consolidation.md`
(TASK-034EO4qUmlyu3qleVAOHoo)、plan
`docs/plans/mechanical-caller-cleanup.md`(T4)、design
`docs/designs/mechanical-caller-cleanup.md`(A9 sketch・
`half` 重複除去・mean_row 融合・ordering・A12 sketch)に
従う。baseline・characterization は T1、先行 opt は
T2(A6)・T3(A7/A8、直列適用済み)。A9 の
`reference.rs:653-654` 側は T6(A16a と同一変更単位)で
扱うため本タスク外。ユーザー指示により**本タスクでは
コミットを作成していない**(Done When の履歴順序はコミット
実施時に担保が必要)。

## Implementation

### Changes

`crates/al-analysis/src/cheap.rs`(1 ファイルのみ、production
は `:630` 付近の 1 ブロック):

- 旧 `:630-632` の `m.magnitude()`/`m.power()`/`m.psd()` 3
  呼出しを design sketch どおりの単一 `power()` pass +
  caller 側導出へ置き換え:
  - `let pow = m.power();` — 唯一の `mag_sq` pass
  - `let norm = (0.375 * m.window as f64) * m.sample_rate
    as f64;`・`let inv = 1.0 / norm;`・
    `let half = m.fft_size / 2;`
  - `mag`: `pow` 各行を `sqrt` して集めつつ、同じループ内で
    `mean_row[k] += v`(`r.iter().take(half)`、row 昇順・
    0.0 始まり)を融合し、collect 後に `*v /= mag.len()`
  - `psd`: `pow` 各行を `v * inv` で写す(`/ norm`
    リテラルなし)
- 旧 `:666` の `let half = mag[0].len() / 2;` を削除
  (push 前の `half` を再利用。`mag` 行長 = `fft_size` で
  同値)
- 旧 `:792-794` の列走査 `mean_row` を削除(融合済みの
  `mean_row` を `spec.slope`・`spec.narrowband_*` がそのまま
  消費)
- ordering: 全導出と `m` field 読みは
  `channel_stft.push(Some(m))`(現 `:687`)より前に完了
- 変更なし(監査済み単一消費者例外): `:962` の
  `mm.magnitude()`、`reference.rs` 側の psd/magnitude 各所
- **A12(`:577-603` 付近の uniq sort+dedup)は不変更**
  (棄却、下記)

### Decisions During Implementation

- **A12 を棄却(deviation)**: task の Done When には
  HashSet 化が列挙されているが、bench-first の方針に従い
  計測結果で判断した。選択肢の検討:
  - (a) early-exit HashSet(1M+1 で挿入停止): **契約違反**
    — `dig.quant_step_est` の support は正確な distinct
    count(`uniq.len()` 同値)が必要で、1M+1 で止めると
    support が実値を下回り characterization
    (`a12_quant_step_over_1m_skip_is_bit_exact` の
    `support_count` 一致 assertion)が破壊される。正確な
    count を維持するには全要素を処理し続けるしかなく skip
    効果も消える。仮に契約を無視しても、典型入力(PCM
    由来 f64、distinct は ≤65 536 程度で in-gate)では全
    要素を hash して ~14 ms(現行 ~7 ms の約 2 倍)を払い、
    >1M 入力でも ~1M+1 回の HashSet 挿入(~35 ms 換算)が
    sort 全走査 ~27 ms を上回るため両経路で不利
  - (b) 棄却: **採用**。コード不変更のため両経路の
    characterization は当然 green で、record 契約も完全に
    維持される
  - (c) サンプリング等の安価な判別子: spec が「近似
    distinct count 禁止」を明記しており contract 外。
    高速 hasher(foldhash 等)の導入は新規依存が必要で
    「`HashSet` は std のみ」制約にも反する
  - 結論: 正確な distinct count を要する本関数では、
    連続 `u64` の `sort_unstable`+`dedup` が std HashSet
    (SipHash + 成長する bucket 走査)に対して常に優位で、
    gate 外での f64 実体化・`total_cmp` sort の skip は現行
    形状でも既に効いている。改善余地なしと判断し、
    design sketch からの deviation として task doc・本 IR
    へ証拠付きで記録
- **`half` は新設側を採用**: `mag[0].len() / 2` より
  `m.fft_size / 2` の方が move 前に確定し、design の
  「`:668` 側を削除」指示どおり新設の `let half` に統一
- **mean_row の除算位置**: collect 完了後の
  `*v /= mag.len() as f64` — 除算回数・値とも旧式
  (`sum / len` per bin)と同一で bit-identical

## Related Experiments

### Adopted

<!-- No experiment logs: mechanical task per task-doc. -->

### Rejected

<!-- No experiment logs: mechanical task per task-doc. A12
棄却は設計候補の bench-first 判断で、exp log ではなく
Decisions During Implementation に記録。 -->

- **A12 `HashSet<u64>` 事前計数**: T1 baseline
  (`target/bench-baseline/criterion-mech-cleanup-pre.txt`)
  で in-gate 7.2851 → 14.171 ms・>1M 27.628 → 55.179 ms と
  候補腕が両経路で現行を下回った。本タスクの再計測
  (`target/bench-baseline/quant_step-mech-cleanup-a9a12.txt`)
  でも in-gate 6.44 vs 14.07 ms・>1M 26.74 vs 50.11 ms と
  同一傾向を確認。early-exit 変種は support 契約(正確な
  distinct count)を破るため採用不可。production コードは
  不変更とし、characterization・bench 腕は今後の再評価
  用に残置

## Bench 計測結果(post-optimization)

環境: `mise exec -- cargo bench -p al-analysis --bench
spectral_pass -- --baseline mech-cleanup-pre`・`--bench
quant_step -- --baseline mech-cleanup-pre`(criterion median、
baseline は T1 が保存した `mech-cleanup-pre`)。生ログは
`target/bench-baseline/spectral_pass-mech-cleanup-a9a12.txt`・
`quant_step-mech-cleanup-a9a12.txt`。初回 run は 2 bench の
並行実行による競合で全腕 +10–38% の偽 regression を示した
ため、**以下は逐次再実行の clean run の値**。

### criterion — spectral_pass(127 フレーム × 4096 bin)

| arm | baseline (median) | post-change (median) | change |
| --- | --- | --- | --- |
| three_passes_mag_pow_psd(旧形状) | 6.1926 ms | 3.2473 ms | −47.6% |
| single_power_derived(採用形状) | 3.4120 ms | 2.9444 ms | −13.7% |
| mean_column_scan(旧形状) | 553.73 µs | 475.80 µs | −8.5% |
| mean_row_major(採用形状) | 151.44 µs | 133.29 µs | −13.9% |
| dom_magnitude_matrix(A16b 旧) | 1.2490 ms | 991.97 µs | −17.3% |
| dom_mag_sq_scratch(A16b 候補) | 446.68 µs | 367.78 µs | −18.7% |

腕コードは bench ファイル内で自己完結・不変のため、全腕の
一律改善(変更のない A16b 腕含む)は baseline 採取時との
run 環境 drift であり、baseline との絶対差は参考値。
**同一 run 内の腕比較が採用差分の実測**: 今回 run では
three_passes 3.25 ms → single_power 2.94 ms(約 10% 削減)
に加え、production では消えた mean 列走査 476 µs → 融合
row-major 133 µs(約 3.6 倍)が乗る。baseline 時点の腕比較
(6.19 → 3.41 ms、約 1.8 倍)が long-run の採用期待値。

### criterion — quant_step(A12、棄却の再確認)

| arm | baseline (median) | post-change (median) | change |
| --- | --- | --- | --- |
| in_gate_double_sort(現行=維持) | 7.2851 ms | 6.4405 ms | −11.6% |
| in_gate_hashset_gate(棄却) | 14.171 ms | 14.065 ms | −0.7% |
| over_1m_double_sort(現行=維持) | 27.628 ms | 26.740 ms | −3.2% |
| over_1m_hashset_gate(棄却) | 55.179 ms | 50.114 ms | −9.2% |

同一 run 内で HashSet は in-gate 2.18 倍・>1M 1.87 倍遅く、
baseline の判断が再現。production は現行形状維持。

### Instant — bench_reference(3 run、`bench_reference-pre.txt`・
`t5` との比較)

| 計測点 | pre (mean) | post T2+T3 (t5) | post T4 (mean of 3) |
| --- | --- | --- | --- |
| stft 1M samples (4096/2048) | 32.5 ms | ~31.9 ms | ~32.7 ms (30.3/37.2/30.5) |
| analyze_pair 2s mono (R0–R3, fir_model) | 631.8 ms | ~580.5 ms | ~548.6 ms (543.3/549.2/553.3) |
| prepare_residual 2s 2src × 2cap | 10.9 ms | ~7.1 ms | ~7.8 ms (8.2/7.8/7.5) |

`analyze_pair` は内部で `cheap::analyze_input`
(`reference.rs:172`)を呼ぶため A9 の効果を含む。
pre 比 −13.2% は A6+A7/A8+A9 の累積、t5 比 −5.5%
(580.5 → 548.6 ms)のうち A9 寄与は残りが run noise 含み
で分離不能だが方向は一貫。

## Validation

- `mise exec -- cargo test -p al-analysis`: green —
  139(lib 93 + avsync 7 + expensive 8 + golden 19 +
  reference 12)。A9 characterization
  (`a9_mag_pow_psd_tensors_are_bit_exact`・
  `a9_spec_slope_record_is_bit_exact`)と A12 両経路
  (`a12_quant_step_in_gate_is_bit_exact`・
  `a12_quant_step_over_1m_skip_is_bit_exact`)を含む全
  mechanical_cleanup テストが bit-identical green
- `mise exec -- cargo test --workspace`: green(golden 含む)
- `mise exec -- cargo clippy --workspace --all-targets --
  -D warnings`: green
- `mise exec -- cargo fmt --all -- --check`: green
- `grep -n "/ norm\|/norm" cheap.rs`: `let inv = 1.0 / norm;`
  の 1 箇所のみ(逆数定義。psd 導出は `v * inv` で
  `/ norm` リテラルなし)
- `grep -n "m\.magnitude()\|m\.psd()\|m\.power()" cheap.rs`:
  production は `m.power()` 1 回(新 `:635`)と単一消費者
  例外 `mm.magnitude()`(`:962`)のみ。残りは
  `#[cfg(test)]` oracle
- `bench`: `spectral_pass`・`quant_step` を
  `--baseline mech-cleanup-pre` で比較実行(clean 再 run を
  記録)、`bench_reference` 3 run を
  `target/bench-baseline/bench-reference-t4.txt` へ保存

## Risks

- criterion 値は本マシンの run。`three_passes` の −47.6% の
  ように baseline 採取時との環境 drift が大きく、
  baseline 絶対差より同一 run 内の腕比較を信頼すること
- 初回の bench 並行実行で競合 noise が乗った経緯があり、
  記録値は逐次 clean run に限定した。以降のタスクも
  bench は逐次実行が前提
- A9 の mag 導出は `map` 内で `mean_row` へ副作用的に累積
  する shape(design sketch どおり)。走査順が変わる将来の
  書き換え(parallel iterator 等)で加算順が変わると
  bit-identical が崩れる点に注意 — characterization が
  ゲートとして残る
- A12 は棄却だが bench 腕(`quant_hashset_gate`)は bench
  ファイル内に残置。今後 std HashSet の性能改善や別 gate
  方式の再評価時にそのまま比較できる

## Follow-ups

- T5(A15-opt): interleave/deinterleave ブロック化 —
  `.f64le` バイト列・deinterleave 列・residual interleave
  列ゲート + `bench_dsp` A15 区間再計測
- T6(A16-opt): `reference.rs:653-654` の A9 派生側(A16a、
  psd 非実体化 + 行優先 `mean_psd_*`)・dom scratch・
  video 融合・`read_range` fast path
- T7(measurement-ir-closeout): 全 bench の最終比較・char→
  opt 履歴順序の確認(コミット実施時)。A12 棄却の記録は
  本 IR で完結しているため T7 で再確認のみ

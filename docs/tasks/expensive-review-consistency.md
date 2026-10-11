---
id: "TASK-034dLdqYk3HXx4PdtOgnVz"
type: "task"
status: "done"
title: "al-analysis: expensive ファミリの残存精度/一貫性(PR #26 レビュー LOW)"
created: "2026-10-10"
updated: "2026-10-10"
owners: ["scarecrow173"]
relations:
  source: []
  implements:
    - "docs/plans/residual-fixes.md"
  implemented-by: []
  depends-on:
    - "docs/plans/residual-fixes.md"
    - "docs/designs/residual-fixes.md"
    - "docs/adr/residual-issue-boundaries.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related: []
  refines: []
  refined-by: []
  derives-from: []
  derived-by: []
  verifies: []
  verified-by:
    - "docs/test-specs/residual-fixes.md"
  references:
    - "https://github.com/scarecrow173/audio-lens/issues/29"
  defers: []
  deferred-by: []
---

# al-analysis: expensive ファミリの残存精度/一貫性(PR #26 レビュー LOW)

## Work

#29 の残件を以下の原文棚卸しに沿って確認する。Issue 全体を対応単位とし、修正・文書化・承認済み defer の各結果と証拠を項目ごとに残す。複数の独立修正を実施するときは承認後に task を分割し、この対応を保持する。

## Classification

- `bug-fix`
- approved の residual-fixes chain と今回の実装指示に基づく実装 task。

## Upstream Documents

- `docs/plans/residual-fixes.md`
- `docs/specs/residual-compliance.md` AC-29
- `docs/designs/residual-fixes.md`
- Issue #29

## Dependencies

spec/ADR はチェーンの受理可能な status を満たし、design/plan と test-spec は承認済みのため status todo。

## Affected Files

- `crates/al-analysis/src/expensive/`
- `crates/al-analysis/tests/expensive.rs`
- `crates/al-analysis/src/expensive/tests.rs`

## Issue Inventory (2026-10-10)

### 背景

PR #26(`feat(al-analysis)`)のレビューで検出された非ブロッカー指摘。HIGH は `d9909db` で修正済み。

### 項目

- `channel.rs` `exp.mix_band_coeff`: 帯域の condition > 1e3 でも `up_val`(Valid)を維持する。ヘッダコメントは ill-conditioned 帯域の demote を謳うが、実際に demote されるのは 3 つの summary レコードのみ
- `channel.rs`: `cap_segs` が `len >= o1` でフィルタするため、チャネル長が不均一なとき `ChannelSelector::Channel(c)` がフィルタ後リストを index し、ch0 脱落時に ch1 を ch0 と誤ラベルする
- `mod.rs`: `&c[s0..s1.min(c.len())]` が非プライマリチャネルが `s0` より短いとき panic(start > end)。`segs` ベクタで露出が拡大(分割前からの既存パターン)
- `timing.rs` `exp.splice_first_s`: `count == 0` で NaN+Valid を出力;`codec.rs` `exp.codec_block_size` は全候補スキップで 0+Valid;`exp.src_candidate_rate` は全 fold 相関 ≤ 0 で 0.0 Hz+Valid を出力しうる
- `mod.rs` `weakest_validity`: 列挙に深刻度順がなく最初の非 Valid を保持するだけ(見た目の問題)
- `ir.rs`: early/late 窓が循環 wrap しないため、ゼロ遅延 IR の pre-peak リンギングが late energy に混入
- `codec.rs`: 「energy-derivative energy」コメントと生ブロックエネルギー畳み込み実装の乖離
- pair 由来 summary レコード(level_transfer_*, mix_* summary, ir_*, local_time_map)が `Emit::push` により `Channel(0)` で記録される(全チャネル/ペアにまたがる測定なのに誤解を招く)
- emit_invalid カバレッジの継承分: `transfer.rs`(total_power ゲート)、`codec.rs`(qmax>qmin、total_power)、`timing.rs`(n >= 4*block)は分割前からの暗黙スキップが残る

Refs #26

## Steps

1. 原文の各項目を実装と既存 spec に照合し、修正か判断かを記録する。
2. 違反する契約を固定する回帰テストを追加する。
3. 対象テストを実行し、期待する失敗を確認する。
4. 所有境界で修正する。判断項目は承認後に着手する。
5. 対象検証を再実行する。
6. 項目ごとの証拠と文書を更新する。
7. 意味のある変更単位でコミットする。

## Done When

- [x] 不均一チャネル長で panic とチャネル誤ラベルが起きない。ill-conditioned 帯域の係数にも妥当性が反映され、支持数ゼロや候補なしの測定を NaN/0 + Valid として出さない。IR の窓、ペア summary のチャネル意味、暗黙スキップとコメントの整合を fixture で検証する。
- [x] 原文全項目に修正・文書化・承認済み defer と検証証拠の対応がある。
- [x] 関連 spec/ADR/design/test-spec が実施内容と一致する。

## Verification

- [x] `mise exec -- cargo test -p al-analysis` が exit 0。原文項目に対応した回帰テストを含む。
- [x] `mise run fmt` と `mise run lint` が exit 0。
- [x] 文書と Issue 項目の対応を review する。

## Implementation Inventory (2026-10-10)

`gh issue view 29 --repo scarecrow173/audio-lens --json body,title` で原文を再取得し、上記 9 項目に差分がないことを確認した。AC-29 と test-spec Examples #29 の代表例を含め、以下の単体テストは `crates/al-analysis/src/expensive/tests.rs` に置く。defer はない。

| 原文項目 | 対応と所有境界 | 回帰・証拠 |
| --- | --- | --- |
| ill-conditioned 帯域の係数 | 修正: channel family で condition > 1e3 の係数を LowCoherence とし理由を付ける。condition 自体は測定した指標なので Valid を維持する | `ac29_ill_conditioned_coefficients_are_demoted`: 近似従属の source 2ch で該当係数が Valid になる red → green |
| capture channel 誤ラベル | 修正: capture の slice と元の index を組で保持し、欠けた channel には InsufficientSupport 係数を残す | `ac29_mix_keeps_channel_identity_after_short_channel_is_excluded`: ch0 が窓始点より短く ch1 のみ支持する family 境界で ch1→ch0 の red → green。orchestrator の ch0 timeline 早期 return を通らず family の責務を直接検証 |
| 不均一長 panic | 修正: region 境界で始点・終点を各 channel 長へ clamp し、空 slice でも元の位置を保持する。coherence family は空/短い ch1 を暗黙スキップせず ChannelPair(0,1) の未支持を出す | `ac29_short_secondary_channel_is_unsupported_without_panicking`: Examples #29 の ch0=4096/ch1=512、1024 Hz、[1,2)、window=64/hop=32 で mod.rs:623 の slice panic を実際に再現(red) → green |
| splice/codec/SRC の支持・候補なし | 修正: splice_count=0 は有効な観測数、first_s は未支持。codec は支持された候補を Option で保持し、全 skip/無エネルギーでは size/score を NaN+InsufficientSupport。SRC は評価した image score と正相関の rate 候補を区別し、候補なしの rate を NaN+InsufficientSupport | `ac29_no_splice_has_valid_zero_count_but_no_first_time`、`ac29_no_codec_candidate_is_not_valid_zero`、`ac29_zero_fold_scores_have_no_source_rate_candidate`: それぞれ Valid 出力の red → green |
| weakest_validity の名称 | 文書化・命名修正: 非 Valid 分類間に深刻度順は設けず、既存の最初の非 Valid を保持する方針を `first_non_valid` とコメントに明示。分類の順序を新設しない | `ac29_validity_categories_keep_first_non_valid_without_severity_order`: 入力順による分類保持と Valid 非昇格を固定 |
| IR early/late の wrap | 修正: IFFT の循環表現を pre-peak guard 始点で一度だけ並べ直す。early/late と peak 後の decay が同じ順序を使い、pre-ringing を late/decay に含めない | `ac29_ir_windows_are_invariant_to_circular_peak_position`: peak=0 の ratio=3.846… の red → 125 の green、peak=900 の wrap でも同一。既存 `ir_family_recovers_synthetic_decay` 維持 |
| codec のコメント | 文書化: 実装は生 block energy の境界/内部比であり、energy derivative という誤説明を修正。推定方式は変更しない | `ac29_codec_periodicity_measures_raw_boundary_energy`: 576 samples ごとに振幅 10 倍の 64-sample block を与え、size=576、energy ratio=100 を固定 |
| pair summary の channel 意味 | 修正: 共通 Emit::push_pair で単一 capture channel selector を外す。level/mix/IR/time_map の成功、未支持、invalid-trigger schema に適用。alignment_id を保持し、level/time_map が実際に使う source/capture channel 0 と mix の支持 capture index を params に記録 | `ac29_pair_summaries_do_not_claim_capture_channel_zero`: Channel(0) の red → green。`ac29_measured_pair_summaries_keep_pair_scope_and_alignment` と expensive 統合 `pair_evidence_populates_pair_dependent_families` で成功/欠落の保存形も確認 |
| 継承された暗黙スキップ | 修正: transfer の total/one-sided power、codec の qmax<=qmin/total_power、timing の短い repeat 窓は明示的な NaN+InsufficientSupport と理由を出す。spectral record の resolution も保持 | `ac29_unsupported_gates_emit_records`: 6 ids の欠落を一度に red で確認 → green。`ac29_empty_quefrency_interval_emits_invalid_echo_records`: 4 echo ids の欠落を red → green |

### Verification Results

- `mise install --locked python uv pipx:apm-cli pipx:serena-agent` と `mise run setup`: exit 0。setup が生成した範囲外の apm.lock.yaml 差分は元に戻した。外部依存は変更していない。
- `mise exec -- cargo test -p al-analysis`: exit 0、98 tests（unit 68、avsync 2、expensive 6、golden 19、reference 3）。追加の AC-29 unit regression は 13 件。
- `mise run fmt`: exit 0。
- `mise run lint`: exit 0（workspace/all-targets、warnings を error として扱う）。
- `mise run test`: exit 0（workspace 全体）。
- 保存形の assertion 追加後に `mise exec -- cargo test -p al-analysis --test expensive` を再実行: exit 0、6 tests。fmt/lint も再実行し exit 0。
- task/index の done 同期、Issue inventory 9 項目と AC-29 の照合、`git diff --check`: 成功。

spec/ADR/design/plan/test-spec の approved status と AC は変更していない。上流文書の過去の draft/承認待ちに関する説明は文書整備時点の記録であり、今回の実装指示の前提をこの task に記録する。

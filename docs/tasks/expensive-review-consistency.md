---
id: "TASK-034dLdqYk3HXx4PdtOgnVz"
type: "task"
status: "todo"
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
- draft plan のレビュー用候補。依頼により先に文書化するが、通常の承認済み計画由来の実装 task ではない。

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

- [ ] 不均一チャネル長で panic とチャネル誤ラベルが起きない。ill-conditioned 帯域の係数にも妥当性が反映され、支持数ゼロや候補なしの測定を NaN/0 + Valid として出さない。IR の窓、ペア summary のチャネル意味、暗黙スキップとコメントの整合を fixture で検証する。
- [ ] 原文全項目に修正・文書化・承認済み defer と検証証拠の対応がある。
- [ ] 関連 spec/ADR/design/test-spec が実施内容と一致する。

## Verification

- [ ] `mise exec -- cargo test -p al-analysis` が exit 0。原文項目に対応した回帰テストを含む。
- [ ] `mise run fmt` と `mise run lint` が exit 0。
- [ ] 文書と Issue 項目の対応を review する。

現時点では文書整備のみであり、上記実装検証は未実行。

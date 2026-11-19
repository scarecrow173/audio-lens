---
id: "EVID-uM3qVsXzRwPnYjKbHgTf"
type: "evidence"
status: "approved"
title: "検証記録: 性能計測・最適化・baseline 管理"
created: "2026-11-19"
updated: "2026-11-19"
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
  implements: []
  implemented-by: []
  depends-on: []
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

# 検証記録: 性能計測・最適化・baseline 管理

特性化・最適化・等価性ゲート・baseline 管理に関する検証の事実記録。

## 特性化と baseline 構築

- AL#98 の run matrix(canonical + variant-*×5 + small + resume + aa の 9 run)で baseline を固定。全体で約 40 分・生成物 ~20GB。
- `tools/baseline/` の `capture.mjs`(run matrix 実行)、`compare.mjs`(contracts.json による golden/index 比較、閾値 rel 1e-9/abs 1e-12)、`lib/store.mjs`(al-dataset 読取)、`lib/procmon.mjs`(子プロセス CPU/RSS ポーリング)を構築。
- `crates/al-analysis/examples/bench_alloc.rs` で counting allocator による代表処理の alloc 計測。
- `baseline/` 成果物: environment.json / inputs.json / config.json / coverage.json / contracts.json / runs/*/ / bench/。

## 階層的位置合わせ(locate)

- canonical e2e 368.0s→186.8s、locate stage 270.2s→87.6s、`window_anchors_in` 252.0s→68.0s。
- Golden 比較(9 run、rel 1e-9/abs 1e-12): 全 run 0 diffs(bit-exact 維持)。

## caller 側最適化

- `analyze_pair` ~691.2→581.0ms(-15.9%)。STFT -13.22%/-12.51%(1M/512k)。`prepare_residual` 19.3→7.1ms(-63.2%)。`lstsq solve` の 2 列回帰は承認済みトレードオフとして記録。

## Ruviform 置換の性能影響

- AL#101: e2e ネット -1.7%(124612→122529ms)。locate -2.8%、reference_analysis +24.8%(band_gain 3rd-octave×窓 NCC ~380ms/pair + f0 corr0 sweep)、expensive_analysis +18.6%。カーネル単位は corr0 sweep 4.0x、NCC plan +23%、conv +45% 等の scalar 化トレードオフ。
- AL#104: e2e 8/9 run で中立〜改善(-8.2%〜+0%)。`bench_dsp` legacy_fft 4M 325.0ms→ruviform fft 184.6ms/plan 再利用 120.6ms/rfft 68.5ms、`bench_residual` sub_scaled 1src -19%・2src -53%。
- AL#133 mixed-radix: canonical e2e +10.2%/locate +17.1% の net 回帰。変換長 narrow_lo 65536→64000(-2.3%)、narrow_hi 262144→135000(-48.5%)、widen_mid 1048576→583200(-44.4%)、widen_hi 2097152→1062882(-49.3%)、full 4194304→2430000(-42.1%)。tpl≈sig メソッドは direct→FFT 反転で ncc 134.38→218.61ms(+63%)。

## PcmView 計測

- mmap `open` p50 81–86µs。全長 materialize(debug、N=30): 36MB baseline 248ms/mmap view 219ms/owned view 161ms、368MB 2548ms/2242ms/1584ms。Owned が mmap より p50 高速だが alloc bytes 半分で mmap-first 維持。

## 計測ノイズの管理

- 並行負荷時は span busy/call が 3-6 倍に膨らむため、初回 `*-pre133.json`(canonical 208.2s/small 223.1s)を棄却しアイドル時 `*-pre133b.json` を採用。初回で「canonical -32%」と読めたのは計測誤差。

## 未検証・制約

- baseline の詳細ベンチマーク(criterion)は `target/criterion` 参照で baseline にはサマリのみ。
- 性能比較の #98 基準との定量追跡は tools/baseline 資産の対象。
- AL#133 の net 回帰は未解消(ruviform Issue 起票提案として残置)。

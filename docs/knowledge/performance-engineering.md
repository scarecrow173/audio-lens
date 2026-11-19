---
id: "KNOW-hT5vXnWzQmRbKpYjFgSd"
type: "knowledge"
status: "approved"
title: "性能工学: 特性化・baseline・等価性ゲート・最適化方法論"
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

# 性能工学: 特性化・baseline・等価性ゲート・最適化方法論

性能改善は「特性化→baseline 計測→単一最適化→等価性ゲート→事後計測→完了報告」の一貫した手順に従う。本書はこの方法論と、各最適化で得られた教訓をまとめる。

## 方法論の基本構造

- **特性化を先行**: 実ジョブの wall-clock/CPU プロファイルで支配コストを特定してから最適化する。「何が支配的か」を実測で確立し、直感で最適化対象を選ばない。
- **baseline を固定**: 変更前の計測値を記録し、最適化後は同一バイナリ・同一 fixture・同一マシンで再計測する。並行負荷時の計測は数倍に膨らむため、アイドル時の再計測を必須とする。
- **等価性ゲート**: 性能変更は数値等価を前提とする。bit-exact(整数・`to_bits`・byte 一致)と tolerance(`|a-b| <= 1e-12 * max(1,|a|,|b|)`)を変換の性質で選択する。
- **退行の帰属分類**: 性能退行を「承認済みトレードオフの既知クラス」と「新規退行」に分離し、今回の変更に起因する新規退行が無いことを論証する。

## 等価性の基準

- **bit-exact 維持契約**: 変更が演算順序を変えない場合(同一 FP 演算・同一順序)。`to_bits`/`byte`/`exact tuple` で比較。
- **浮動小数許容契約**: scalar vs SIMD の累積順序差・FFT の mixed-radix 変換長差など、数値的に等価だが bit 異なる変更。rel 1e-9/abs 1e-12 の tolerance で比較し、最大誤差の定量化を記録する。

## 得られた性能数値と教訓

- 階層的位置合わせ(1/8 間引き → 64 サンプル粗相関 → ≤256 候補 → 全解像度精細化)で canonical e2e 368.0s→186.8s、locate 270.2s→87.6s(支配 stage)。粗結果は候補提案のみで、受入は全解像度での判定に委ねる。
- caller 側最適化:`analyze_pair` ~691→581ms(-15.9%)、STFT -13.2%/-12.5%、`prepare_residual` 19.3→7.1ms(-63.2%)。`lstsq solve` の 2 列回帰は承認済みトレードオフ。
- PcmView(mmap): mmap `open` p50 81–86µs で materialize の 0.00–0.04% に過ぎず、job-scoped cache は不採用。Owned が mmap より p50 高速(36MB/368MB いずれも)だが alloc bytes 半分で mmap-first を維持した。
- scalar 化(Ruviform 置換):corr0 sweep ~4x、reference_analysis +24.8% のカーネル単位退行だが e2e はネット中立(-1.7%)。「カーネル単位退行≠e2e 退行」は支配 stage の構成比で決まる。
- `bench_alloc`: `analyze_pair` 619.1ms/15788 allocs/240.9MB。alloc 計測は counting allocator で実施する。
- `bench_dsp`: legacy_fft 4M 325ms→ruviform fft 184.6ms/plan 再利用 120.6ms/rfft 68.5ms。

## 計測上の罠

- 並行負荷(ビルド並走)時は span busy/call が歴代の 3-6 倍に膨らむ。初回計測を棄却しアイドル時再計測を採用する。
- `span busy/call` は並列下の概算値であり結論は方向性に依拠する。絶対値でなく変化の方向で判断する。
- `serde_json` の `float_roundtrip` 無効の既定パーサは f64 を 1-ULP 丸める —— 永続化値を生 f64 と直接比較するテストは失敗し得る。
- tasklist LIST 形式の parse はロケール非依存(日本語ロケールでも可)。

## 性能回帰の未解消例

Ruviform pin `258df5a`(mixed-radix FFT)で canonical e2e +10.2%/locate +17.1% の net 回帰を観測し、未解消として記録した。ruviform 側の `prefers_direct` 係数 8.0 が pow2 radix-4 前提で誤判定すること、Full mode で tpl≈sig の smooth 化恩恵が薄いことを起票提案として残した。変換長削減(narrow -2.3%〜-49.3%)は tpl≪sig の NCC(anchor 経路)を改善するが、tpl≈sig メソッド(direct→FFT 反転)を悪化させるため、負け側を直せば全体で効く構成となる。

関連 Issue/PR: AL#98、#101、#104、#117、#127、#133、RF#255、RF#256、PR #130。

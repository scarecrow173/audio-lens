---
id: "IMPL-034PBt7Fxp5gyMIM7qY05a"
type: "impl"
status: "completed"
title: "Issue #71 band_gain pair 内並列化 (al-analysis)"
created: "2026-10-31"
updated: "2026-10-31"
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
  related:
    - "docs/impl/ir/perf-reeval-2026-10.md"
  refines: []
  refined-by: []
  derives-from: []
  derived-by: []
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
metadata:
  experiments:
    adopted:
      - "52+2 タスク run_pool(ordered slot・channel-major 復元)と worker-local Ncc plan/workspace"
    rejected:
      - "window 粒度タスク(~5.2万/pair: slot 管理・収集コストに対し 52 タスクで十分占有できる)"
      - "BandFilter 共有(band_into が &mut self のため直列化する。ruviform #183 へ)"
      - "直算 NCC API(ruviform #168: band_win shape で ~45x 遅いと再評価済み)"
---

# Issue #71 band_gain pair 内並列化 (al-analysis)

## Summary

reference_analysis の `band_gain`(analyze_pair の ~86%、
~28.2s/pair)を、(channel, band) = 2×26 = 52 タスクの pair 内
並列化へ置き換えた。locate `run_pool` と同一パターン
(available_parallelism・atomic counter・ordered slot 収集)で、
channel ごとの coherence も同一 pool 内で重畳する。結果の収集順は
逐次実行と完全一致し、9 ラン golden 全件 0 diff(bit-exact)を
確認した。

## Context

- 再評価(`perf-reeval-2026-10`): `band_gain` ~28.2s/pair が
  `analysis::reference`(~32.6s busy/pair)の ~86%。内訳は
  windows NCC ループ ~69%(25,917 windows/ch × ~0.374ms)+
  `bandpass_many` ~28% + filter/coherence ~3%。
- locate 側 `band_ncc` は移行で 6.3× 高速化済みで対象外。残対象は
  reference 側 `band_gain` のみで、直算 API(ruviform #168)はこの
  shape では ~45× 遅く不適。構造変更(並列化)が正解と確定済み。
- `analyze_pair` は pair 逐次(`stage.rs`)のまま。pair 内部並列が
  本 Issue の範囲。

## Implementation

### Changes

**`crates/al-analysis/src/band_gain.rs`**

- `run_pool`: al-locate::stage 同名関数と同一実装を crate 内に複製。
  `available_parallelism` 個の scoped worker が atomic counter で
  タスクを奪取し、`Vec<Mutex<Option<T>>>` の slot へ結果を書き、
  join 後にタスク順で回収する。
- `Ncc`: `CorrelationPlan::new_ncc` + `CorrelationWorkspace` の
  ワーカーコンテキスト。shape (window, window+2·max) は全タスクで
  同一のため worker ごとに 1 回構築して使い回す(従来は band ごと
  `windows()` 内で再構築していた)。
- `windows_with` + `windows`: `windows` の plan/workspace 構築を
  外側へ分離。`windows` は `#[cfg(test)]` の薄いラッパーとして残し、
  本番経路は `windows_with` が受け取った `&mut Ncc` を使う。
- `analyze` の並列化:
  - タスク列 = `n_chan` 件の coherence + `n_chan*n_bands` 件の
    (channel-major) band タスク。coherence を先頭に置くことで、
    早い段階で取り出され帯域仕事と重なる。
  - 各 band タスクは `bandpass(x, rate, lo, hi)` で src/cap 両方の
    帯域波形を自前で生成し `windows_with` へ渡す。帯域波形は
    タスク内で生成・消費されるため、P2 の全帯域同時保持回避
    (streaming)は維持される。
  - `TaskOutput::{Coherence, Windows}` で slot 収集し、先に
    n_chan 件の coherence を、続いて channel-major の `all`
    (Vec<Vec<Window>>)を復元する。record/tensor 構築ループは
    逐次のまま変更なし。
- `analysis::band_gain` info_span を追加(pair・len フィールド)。
  span busy が pair あたりの band_gain wall を直接記録する。

### Decisions During Implementation

- **帯域波形生成をタスク内に含める**: (ch,band) タスクは
  `bandpass` 2 回(= `BandFilter::new` + `band` 各 2 回)を行う。
  `BandFilter::band` が `&mut self` を要求するため入力共有では
  直列化してしまい、(入力, 帯域) 間でスペクトルを共有するには
  ruviform 側の新 API(shareable spectrum + per-call scratch)が必要。
  現行公開 API の範囲では、forward FFT の再計算(タスクあたり 2 回、
  pair あたり ~100 回の冗長 forward)を受け入れてタスク独立性を
  取る設計とした。冗長分の解消は ruviform #183 に記録。
- **coherence を同一 pool に混ぜる**: coherence は channel ごと独立
  (~0.4s/ch)。別 pool 呼出だと ~0.4s の逐次区間が残るため、タスク列
  先頭に配置して帯域仕事と重畳した。計算自体・評価順は不変更。
- **ワーカー ctx で plan を使い回す**: 逐次版は band ごとに plan を
  作り直していた(52 回/pair)。worker ごとの使い回し(≤16 回/pair)
  は数値に影響しない scratch のみで、規約「plan はスレッドごとに
  構築する」にも適合する。

## Related Experiments

### Adopted

- (ch,band) 52 + coherence 2 = 54 タスクを locate と同型の
  `run_pool` で実行し、ordered slot で逐次と同一の収集順を復元。
  9 ラン golden 全件 0 diff、性能改善あり(下記 Validation)。

### Rejected

- **window 粒度タスク**: 約 51,800 windows/pair を 1 タスクずつ
  管理すると slot 生成・収集の固定コストが大きく、52 タスクでも
  16 core を 3 wave 以上占有できるため採用せず。帯域波形の
  共有単位としても (ch,band) が自然。
- **`BandFilter` による入力共有**: `band_into`/`band` が
  `&mut self` のため共有は直列化に等しい。共有スペクトル +
  分離スクラッチの API 追加は ruviform 側の責務(#183 起票済み)。
- **直算 NCC API(ruviform #168)**: 再評価で band_win shape
  (tpl=4800, sig=8640)に対し ~45× 遅いと測定済み。不適。

## Validation

- [x] `mise run fmt` / `check` / `lint` / `test` / `build` /
  `verify-toolchain` / `check-cli-headless` 全 pass
  (`cargo fmt` 1 箇所整形 + 新規証跡 JSON 2 件の末尾改行を
  `biome check --write` で修正)
- [x] 単体テスト(`band_gain`)4 件 + 機械系 2 件 pass
  - `independent_band_delays_and_channel_polarity_are_recovered`
  - `top_half_is_selected_per_polarity_with_time_ties_and_true_median`
  - `silence_short_input_and_search_edge_are_explicit`
  - `unrelated_loud_capture_never_becomes_supported_frequency_response`
  - `a7_band_gain_m2_coherence_params_are_bit_exact`
  - `a16a_band_gain_psd_ratio_is_bit_exact`
- [x] 9 ラン golden 比較: 同一ブランチ pre/post(変更前 binary で
  `baseline/runs/<name>-pre71` を capture し、変更後と
  `compare.mjs` 閾値 rel=1e-9/abs=1e-12 で比較)で **全件 0 diff**
  - canonical / variant-desync / variant-clipping / variant-reverb /
    variant-filter / variant-clicks / small: いずれも pre71 比 0 diff
  - resume: pre71 比 0 diff、canonical(post)比 0 diff
  - aa(決定性確認ラン): pre71 比 0 diff、canonical(post)比 0 diff
  - peak RSS: canonical で pre/post とも ~4.3GB(増分なし。
    worker あたりの一時領域増は locate 支配の e2e ピークに埋没)
- [x] 性能計測(`tools/perf-reeval/capture.mjs`、同一ツール同一
  手順で base/after を取得。after は
  `perf-reeval-2026-10/{canonical,small}-post71.json`)

  | run | 指標 | base | after | 倍率 |
  | --- | --- | --- | --- | --- |
  | canonical | e2e wall | 186.8s | 157.0s | ~1.19x |
  | canonical | reference_analysis stage | 68.0s | 39.9s | ~1.70x |
  | canonical | `analysis::reference` span busy | 66.5s/2pair | 38.4s/2pair | ~1.73x |
  | canonical | `analysis::band_gain` span busy | ~56.4s/2pair(推定) | 28.6s/2pair(14.3s/pair) | ~2.0x |
  | small | e2e wall | 85.3s | 84.6s | ~1.01x |
  | small | reference_analysis stage | 3.55s | 2.46s | ~1.44x |
  | small | `analysis::reference` span busy | 2.85s/4call | 1.74s/4call | ~1.63x |
  | small | `analysis::band_gain` span busy | —(推定~0.6s/pair) | 0.98s/3call(0.33s/pair) | ~1.8x |

  base の band_gain 値は再評価の推定(~28.2s/pair = reference
  busy の ~86%)を 2 pair 分に展開したもの。変更前には専用 span が
  なく直接計測できないため推定値と表記し、本変更で追加した
  `analysis::band_gain` span の busy を after とする。
  reference span の減少分(−28.1s)は band_gain の改善分
  (~−27.8s)と整合する。

  **見積もりとの差異**: spec 見積もり ~4-7s/pair に対し実測は
  ~14.3s/pair(~2.0×)。主因はタスク内で再計算する forward FFT
  (~4.19M 点 × ~100 回/pair、逐次版は入力あたり 1 回で 26 帯域に
  共有していた)と、FFT 重タスクを 16 worker で同時実行した際の
  メモリ帯域競合。冗長 forward の解消は ruviform #183(共有
  スペクトル + 分離スクラッチ API)がフォローアップ。それでも
  band_gain は reference stage の ~74%(28.6/38.4s)に留まり、
  e2e の支配項は locate stage(~86s)へ移った。

## Risks

- 帯域波形生成の冗長 forward FFT(タスクあたり 2 回)が CPU を
  追加消費する。共有スペクトル API は ruviform 側の責務のため
  恒久対応はそちらに記録した(ruviform #183)。
- worker 同時実行数に比例した `BandFilter` の一時メモリ増。
  golden capture の peak RSS で増分なし(~4.3GB 前後不変)を確認。

## Follow-ups

- ruviform #183: `BandFilter` の共有スペクトル/分離スクラッチ API。
  `(input, band)` タスクが forward FFT を共有できる形があれば、
  本並列化の冗長分(~100 forward/pair)が消え、pair あたり数 s
  規模の追加改善が見込める。
- #54 stage 内並列化: pair/入力粒度の並列化は本件と独立に残る。
  band_gain 縮小後は locate stage(~86s)と reference 残部
  (~10s/pair)が支配項。

---
id: "IMPL-034PDpB1abYsEy5uTTUgS4"
type: "impl"
status: "completed"
title: "Issue #54 stage inner parallelization (pair/input granularity)"
created: "2026-10-31"
updated: "2026-10-31"
owners: []
relations:
  source: []
  changes:
    added:
      - type: "code"
        path: "crates/al-analysis/src/pool.rs"
      - type: "doc"
        path: "docs/impl/ir/issue-54-stage-inner-parallelization-pair-input-granularity.md"
    modified:
      - type: "code"
        path: "crates/al-analysis/src/lib.rs"
      - type: "code"
        path: "crates/al-analysis/src/band_gain.rs"
      - type: "code"
        path: "crates/al-analysis/src/stage.rs"
    deleted: []
    renamed: []
    moved: []
    generated:
      - type: "doc"
        path: "docs/impl/ir/perf-reeval-2026-10/canonical-post54.json"
      - type: "doc"
        path: "docs/impl/ir/perf-reeval-2026-10/small-post54.json"
  implements: []
  implemented-by: []
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/impl/ir/perf-reeval-2026-10.md"
    - "docs/impl/ir/issue-71-band-gain-pair-al-analysis.md"
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
      - "crate::pool::run_pool(max_workers) + ordered slot 回収、pair/入力/region タスク化と逐次 write phase の分離"
    rejected:
      - "ruviform-exec OfflineDriver(process が Result を返せず Io in-place 変換と per-stage Processor adapter が必要。Issue 記載の『無理に Adapter は設けない』に抵触)"
      - "expensive の candidate 走査・alignment 読込ループの並列化(集約順序が契約で、analyze_region が支配コスト)"
      - "window/region 内部粒度の更なる並列化(#71 で pair 内 band は完了済み、region 内並列化は効果未確認)"
---

# Issue #54 stage inner parallelization (pair/input granularity)

## Summary

cheap/reference/expensive stage の入力・pair・(input, region) 粒度の
逐次ループを、#71 と同型の ordered-slot ワーカープール
(`crate::pool::run_pool`、al-locate::stage `run_pool` パターン)で
並列化した。dataset への書き込み・`registry.issue()` 採番・
ArtifactRecord push は全て回収後の逐次 write phase に残し、
manifest/artifact の記録順序と FeatureRecord の発行順を逐次実行と
完全一致させている。計算・読込のみ並列で、共有可変状態は導入
しない。

## Context

- Issue #54: locate のみ `run_pool` 並列で、analysis 系 stage は
  入力/pair 単位で逐次実行だった。
- `perf-reeval-2026-10` 再評価: pair 粒度並列の上限は pair 数
  (canonical 2 pair で ~2×)。pair 内並列は #71 で実装済み
  (band_gain 52+2 タスク)。残対象は stage レベルの入力/pair/region
  粒度のみ。
- resume は al-job runner の stage 粒度 verify/replay;stage 内の
  per-iteration キャッシュは存在しない(確認済み)。並列化しても
  resume 単位・artifact 永続化タイミングは変わらない。

## Implementation

### Changes

**`crates/al-analysis/src/pool.rs`(新規)**

- band_gain.rs 内の private `run_pool` を crate 共有に移し、
  `max_workers` 引数を追加: 実効 worker 数は
  `min(n_tasks, available_parallelism, max_workers)`。`usize::MAX`
  で従来動作。decode 済み PCM/band signal 級のワーキングセットを
  持つ外側タスク向けに常駐メモリ上限を設計可能にした。

**`crates/al-analysis/src/band_gain.rs`**

- ローカル `run_pool` を削除し `crate::pool::run_pool` へ切替
  (`usize::MAX`、#71 の動作不変)。

**`crates/al-analysis/src/stage.rs`**

- `MAX_STAGE_TASKS = 4`: 入力/pair タスクの同時実行上限。
  各タスクは decode 済み PCM(数十〜数百 MB)を保持し、reference
  pair タスクは内部で band_gain pool(≤cores)を起動するため、外側
  並列度を上げても throughput は頭打ちでメモリのみ線形増。
  canonical/small の pair・入力数(≤4)は全並列に収まる。
- `CheapAnalysisStage`: 入力ごとの decode_input・video/origin
  読込・validate_video・roi_records・analyze_input・sync_records・
  stamp を run_pool タスク化(`CheapInputOut`)。video/cheap tensor
  書込・write_json・artifact push は逐次 write phase。`prov` の
  video params はタスク内で値確定(VideoFeatureArtifact 本体は
  持ち出さない)。
- `ReferenceAnalysisStage`: locate_result の parse のみ逐次
  (`PairUnit`)。PCM 読込・整列・`analyze_pair` を pair タスク化
  (`PairDone`)。PCM はタスク内 drop で常駐は同時実行数に比例。
  tensor/residual_audio/features JSON の書込と `issue()` は
  write phase で pair 順。
- `ExpensiveAnalysisStage`: decode ループを run_pool で並列化
  (結果は全件 HashMap に残るため常駐不変)。(input, region) を
  `RegionWork` に平坦化して run_pool(`usize::MAX`: PCM は共有
  borrow のみで増えない)。calibration・pair evidence・dictionary・
  candidate 走査は集約順序が契約のため逐次のまま。
- エラー順序: タスクは `Result<_, StageError>` を slot に入れ、
  write phase が入力順に `?` で最初のエラーを返す — 逐次時と同じ
  エラーが同じ順序で表面化し、成功した先行タスクの成果物も
  逐次時と同様に先に永続化される。

### Decisions During Implementation

- **PCM 読込をタスク内に含める**(reference): 事前に全 pair の
  PreparedInput を確保すると pair 数分の PCM が常駐するが、タスク内
  読込なら同時実行数分のみが生存し、analyze 終了で即 drop される。
  I/O と計算の重畳効果もある。cheap でも decode_input をタスク内で
  実行。
- **expensive region タスクは `usize::MAX`**: chans/pair は `decoded`
  map と `loaded_alignments` への共有 borrow で、PCM 常駐は増えない。
  領域出力 tensor は小さいためコア数まで占有してよい。
- **OfflineDriver は不採用**: `run_parallel` は `process(&mut io)` が
  値を返さず(in-place Io 変換)、`P: Clone + Send + Sync` の
  Processor/RtProcessor 実装を stage ごとに要する。ordered slot +
  per-worker ctx + エラーを値として運ぶ現行 `run_pool` で既に
  決定性が保証されており、Adapter を設ける根拠がない(Issue 記載
  の方針どおり)。
- **resume 境界**: stage 粒度の verify_completed_artifacts /
  Pending 再実行は変更なし。並列化は stage 内部のみで、artifact の
  永続化単位・順序・runner への返却型(`Vec<ArtifactRecord>`)は
  不変。

## Related Experiments

### Adopted

- (実装中) `crate::pool::run_pool` に `max_workers` を追加し、
  計算フェーズと write フェーズを分離。ordered slot で逐次と同一の
  収集順を復元。`mise exec -- cargo check/clippy/test -p al-analysis`
  全 pass(143 tests)。

### Rejected

- ruviform-exec OfflineDriver — 上記 Decisions 参照。
- candidate 走査・alignment/calibration 読込の並列化 — `scanned`
  provenance リスト・`all_events`/`calibration_evidence` の追記順が
  契約であり、支配コストの analyze_region と比較にならない。
- window/region 内部粒度 — pair 内 band 並列は #71 済み、region 内
  並列化は効果未確認で本 Issue 範囲外。

## Validation

<!-- Record the checks that were run. -->

- [x] `mise exec -- cargo check --workspace --all-targets` pass
- [x] `mise exec -- cargo clippy --workspace --all-targets -- -D warnings` pass
- [x] `mise exec -- cargo test -p al-analysis` 143 tests pass
- [x] `mise run fmt check test build verify-toolchain check-cli-headless` 全 pass
- [x] `cargo clippy --workspace --all-targets -- -D warnings`(lint の Rust 側)pass
- [ ] `mise run lint` の `biome check` 側は `crates/al-gui/assets/app.js`
  (a5f683e で導入、本変更で未編集)の useTemplate/noUnusedFunctionParameters
  2 件で HEAD 時点から fail — 本 Issue 範囲外の既存問題として報告
- [x] 9 ラン golden 比較 全件 0 diff(`baseline/runs/<name>` post-change
  vs `<name>-pre54` 同一ブランチ pre-change 再取得。`aa` は初回 collect が
  途中切断していたため pre-change binary で `--run aa` を再取得した上で
  0 diff を確認。collect 側の一過性フレークでコード起因ではない)
- [x] `resume` vs `canonical` 0 diff、`aa` vs `canonical` 0 diff
  (決定性・resume 境界不変)
- [x] perf 実測(`tools/perf-reeval/capture.mjs`、canonical + small、
  `*-post54.json` に保存)

### 計測結果(i9-9900K 16 論理コア、release build)

canonical(2 pair / 3 入力、`canonical-post54.json` vs `canonical-post71.json`):

| stage / 指標 | post71(pre54) | post54 | 変化 |
|---|---|---|---|
| cheap_analysis | 5,207 ms | 3,471 ms | -33% |
| reference_analysis | 39,922 ms | 33,919 ms | -15% |
| expensive_analysis | 8,907 ms | 6,714 ms | -25% |
| e2e wall | 156.9 s | 141.2 s | -10% |

- `analysis::reference` span busy 2 calls 合計 38.4s → 64.3s
  (= stage wall 33.9s を超過 = 2 pair span が wall 上でオーバーラップ)。
  pair 内 band_gain pool が既に 16 コアを使い切るため、canonical の
  pair 並列効果は ~2× 上限ではなく非 band_gain 部分とストラグラー
  吸収分の -15% に留まる。理論上限どおり pair 数依存。

small(4 pair / 4 入力、`small-post54.json` vs `small-post71.json`):

| stage / 指標 | post71(pre54) | post54 | 変化 |
|---|---|---|---|
| cheap_analysis | 2,548 ms | 2,351 ms | -8% |
| reference_analysis | 2,458 ms | 1,916 ms | -22% |
| expensive_analysis | 1,668 ms | 1,337 ms | -20% |
| e2e wall | 84.6 s | 80.9 s | -4% |

### peak RSS(baseline capture の procStats 1s ポーリング下限推定)

- canonical 形状: post 5.72 GB(`aa` ラン、非輻輳)vs pre 4.15 GB
  → pair 2 本の band signal/FFT workspace 重畳で約 +1.6 GB。
  同形状の `canonical` ランは外部負荷で輻輳した計測で 7.55 GB
  (locate 273s と異常値を含むため参考値)。基準 ~4.3GB に対し
  ~+1.4〜3GB の増加は pair 並列の設計上許容範囲で、
  `MAX_STAGE_TASKS=4` により常駐ペア数を上限化済み。
- small: post 7.39 GB vs pre 5.63 GB(輻輳計測値、参考)。
- `git merge origin/main`: Already up to date(000abdc)。

## Risks

<!-- Record remaining risks. -->

- pair 並列は pair 数分の PCM + band signal(~26 band × 2.4M
  sample)+ band_gain 内部 pool の workspace を同時生存させる。
  canonical(2 pair)で実測 ~+1.6GB、4 pair 以上の大規模 job では
  `MAX_STAGE_TASKS=4` が上限として効くが、入力サイズに比例して
  増えるため、より大きな入力では常駐メモリに注意が必要。

## Follow-ups

<!-- Record follow-up tasks or next actions. -->

- 入力サイズに応じた動的上限(例: PCM 総量閾値で pair 同時実行数を
  絞る)は将来の拡張候補。現行は YAGNI で固定上限 4。
- baseline capture の collect が一過性に途中切断する事象を観測
  (aa-pre54 のみ。job 側 artifact index は完全) — capture ツール
  側の堅牢化余地は別 Issue 候補。
- #60(residual gather/interleave)・Ruviform #183(shared spectrum
  API)は別 Issue として継続。

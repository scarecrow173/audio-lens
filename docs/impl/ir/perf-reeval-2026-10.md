# perf Issue #76/#71/#54/#60 移行後再評価 (perf-reeval-2026-10)

## Summary

Ruviform 基盤移行(#95 系、main `bc2846e`)後のコードで、移行前に起票された
perf Issue 4件の前提を実測で再検証した。**測定・分析のみで本番コードの
最適化変更は行っていない**。結論:

- **#76 `window_anchors_in`: 依然として有効(方向を絞り込み)**。
  locate wall への支配比率は報告の ~83% から **~93%** に上昇
  (canonical 252.0s / 270.2s)。ただしコストの主因は当初想定の
  plan 再構築 churn ではなく、**narrow → widen(×8) → full の
  再試行エスカレーション**: anchor CPU 時間の ~90% が widen+full
  fallback(全 window の ~44%/~18% が到達)に集中する。
- **#71 band 系相関: 部分吸収 + 依然として有効**。locate 側の
  `band_ncc` メソッドは移行で 10.9s/call → 1.7s/call(~6.3×)に
  高速化し、もはやホットスポットではない。残る対象は reference 側の
  **`band_gain`(28.2s/pair、analyze_pair の ~86%)** で、その内訳は
  windows NCC ループ ~69% + `bandpass_many` ~28%。
- **#54 stage 内並列化: 前提は変わらないが、単独効果は限定**。
  cheap/reference は依然として入力/pair 逐次だが、pair 粒度の並列化は
  canonical(pair 数=2)では ~2× が上限。より大きなレバーは pair 内部の
  並列化であり、それは #71 の領域と重なる。
- **#60 residual gather / interleave: ほぼ吸収済み**。residual の
  gather は既に contiguous `vec::sub_scaled` パスへ置換済みで、
  減算自体は ~0.29s/call。残る interleave/copy 経路は
  e2e の ~1.5% 規模で、close または defer を推奨する。

推奨着手順は **#76 → #71 → #54 → #60** の当初順を維持するが、各 Issue の
攻めどころは移行前の想定から更新が必要(後述)。

## 計測方法

### 環境

- worktree: `al-perf-reeval`、commit `bc2846e`(+ 一時計時
  instrumentation、後述)
- ruviform pin: `080c228d197ebc645b8f81cd0082be442abaf10c`
- CPU: Intel Core i9-9900K @ 3.60GHz、16 logical cores
- OS: Windows_NT 10.0.26200 x64
- toolchain: rustc 1.99.0 (b940084d7)、Node v26.12.1
- ビルド: `mise exec -- cargo build --release -p al-cli`
  (release + debuginfo)

### 手順

1. `tools/perf-reeval/capture.mjs` で `al run` を実行。
   `--log al_locate=info,al_analysis=info,al_job=info
   --log-format file=json` で `<base_dir>/logs/al.*.log` に
   JSON-lines を出力し、span close の `time.busy` と計時イベントを
   集計して `docs/impl/ir/perf-reeval-2026-10/<run>.json` に保存。
2. 一時 instrumentation(計測後に除去。コミット対象外):
   - `al-locate::stage`: `window_anchors_in` の完了イベント
     (windows/range/narrow・widen・full 試行回数と各ティアの
     累計計時/pool builds・hits/anchors/elapsed)、
     `method_tasks_pool` イベント、`locate::locate_channels` /
     `locate::residual_iter` / `locate::prepare_channels` span。
   - `al-locate::normalize`: `locate::prepare_input` span。
   - `al-locate::residual`: `locate::prepare_residual` span。
   - `al-analysis::band_gain`: `analysis::band_gain` span と
     `band_gain_phase` イベント(filter_new_coherence /
     band_signals / windows)。
   - `al-analysis::reference`: `ref_phase` イベント
     (lag/gain/spectral/band_family/similarity/channel/dynamics/
     residuals)。
3. `crates/al-locate/examples/bench_reeval.rs` で `CorrelationPlan`
   の plan build / workspace / ncc transform コストを分離計測
   (min-of-N、単一スレッド)。

### 実行した計測

| run | 入力 | stages | 結果ファイル |
|-----|------|--------|--------------|
| canonical | dropout.avi + normal.avi + source-bgm.wav | 全 stage | `canonical.json` |
| canonical-locate | 同上 | ingest,locate | `canonical-locate.json` |
| small | small 系 4 入力 | 全 stage | `small.json` |
| bench_reeval | 合成信号 | — | `bench-reeval.txt` |

stage 時間は job manifest の `duration_ms` 系フィールドから取得。
span busy は複数タスク並列実行時に合計が stage wall を超え得る
CPU 時間相当の値であり、wall time とは区別して扱う。

## 移行前後のステージ比較 (canonical)

移行前は `docs/impl/ir/issue-104-baseline/perf/canonical.json`
(commit `fe3028d`、同じ計測基盤・同一マシン)。

| stage | 移行前 (ms) | 移行後 (ms) | Δ |
|-------|------------|------------|---|
| ingest | 1,510 | 1,601 | +0.1s |
| locate | 261,528 | 270,225 | +3.3% |
| cheap_analysis | 5,326 | 5,020 | −5.7% |
| reference_analysis | 67,137 | 66,719 | −0.6% |
| expensive_analysis | 8,989 | 9,074 | +0.9% |
| dataset | 226 | 241 | — |
| export | 14,821 | 14,796 | −0.2% |
| e2e wall | 360,080 | 368,041 | +2.2% |

ステージ粒度では移行前後でコスト構造はほぼ不変(差は誤差〜数%の範囲)。
**変化があったのは locate / reference の内部構成**であり、以下の Issue 別
評価の対象となる。

## Issue 別評価

### #76 `window_anchors_in` 構造最適化

**元の前提**: locate wall の ~83% を占め、capture あたり
~802–804 の `best_window_match` 呼出・~3,200 NCC 呼出と報告。

**移行後実測(canonical、2 calls)**:

| call | windows | narrow | widen | full | narrow CPU | widen CPU | full CPU | wall |
|------|---------|--------|-------|------|------------|-----------|----------|------|
| 1 | 514 | 514 | 227 (44%) | 93 (18%) | 193.8s (10%) | 665.7s (35%) | 1,026.8s (54%) | 119.4s |
| 2 | 514 | 514 | 221 (43%) | 94 (18%) | 210.7s (10%) | 765.0s (38%) | 1,059.9s (52%) | 130.0s |

- 2 calls 計 ~252s = locate 270.2s の **93.3%**(元前提 83% から上昇。
  メソッド sweep が高速化したため相対的に悪化)。
- `best_window_match` 呼出 834/call(514+227+93)、NCC 呼出
  ~3,336/call(×4 channel pair) — 呼出回数の前提は成立。
- CPU 合計 ~1,890〜2,040 CPU-s / wall 119〜130s → 16 worker でほぼ
  飽和並列(ストラグルなし)。
- pool: builds ~800 / hits ~2,530 / call。bwm 呼出あたり ~0.96 build
  (=ほぼ毎回新 shape)。**ただし plan build コストは microbench で
  0.4〜30ms/shape で、anchor CPU の ~2% 以下にすぎない**。
- fallback のコスト集中度: widen+full 再試行が anchor CPU の
  **~89〜90%**。full fallback(18% の window)だけで ~53%。
- small run では fallback 率がデータ依存(1 call は 85 windows 中 80 が
  full 到達、別 call は 31/85) — ドリフト量に応じて変動する。
- microbench(単発、無競合): full shape(tpl=512, sig=2.4M)の
  ncc 1 回 = 208〜217ms。本番では 16 並列メモリ帯域競合下で
  ~2.76s/call(~13× の slowdown)。transform 自体が支配コストであり
  メモリ帯域律速。

**判定: 依然として有効(方向の絞り込み)**。

- 「NCC 呼出回数が多い」は正しいが、削減すべきは回数よりも
  **再試行の単価と頻度**。narrow パス自体は CPU の ~10% でしかない。
- plan 再利用(MAX_SHAPES 超過時の全破棄、shape 量子化など)は
  効果 ~2% 級で主題ではない — 当初の最適化仮説は部分的に前提変化。
- 推奨方向(優先度順):
  1. **full/widen fallback の単価削減**: 失敗 window に対し decimated
     (例 8× 間引き)信号で粗い全域 NCC → ピーク近傍で full-rate 精密化。
     4.19M FFT(2.76s 競合下)を ~524K FFT(~50〜100ms)に置き換える
     階層化で full 分(~54%)を ~10× 級削減でき、anchor 全体で
     ~40〜50% 削減見込み。数値契約の再設計が要る中規模変更。
  2. **fallback 頻度削減**: 隣接 anchor の dcapture/dsource から
     expected 位置をドリフト対応で予測(現状 slack = w_off×5%+512 の
     率ベース)。44%→数% に下がれば widen 分(~36%)も圧縮できる。
     セマンティクス(anchor 採否・スコア)を守る必要がある中リスク。
  3. shape 量子化による plan 再利用: 補助的(数%級)。単独ではやらない。

### #71 band 系相関

**元の前提**: locate `band_ncc` が ~174s busy / 16 calls ≈
10.9s/call、reference `band_gain` が ~23s/pair ≈ analyze_pair の
83%、~47,600 window 相関/pair と報告。

**移行後実測(canonical)**:

- locate `band_ncc`: busy 27.68s / 16 calls = **1.73s/call**
  (元 10.9s/call から ~6.3× 高速化)。移行で `bandpass_many` +
  `CorrelationPlan::new_ncc`(短縮 FFT)へ置き換わり、集約 busy でも
  locate 全体の ~10% 相当に低下 — もはやホットスポットではない。
  (他メソッド: gcc_phat ~2.1s、spectral_envelope ~1.3s、
  full_ncc ~0.47s/call)
- reference `band_gain`(2 pair × 2ch、band_gain_phase 実測):

  | phase | per-ch (ms) | pair 計 (ms) | 比率 |
  |-------|------------|-------------|------|
  | filter_new_coherence | ~420 | ~0.85s | ~3% |
  | band_signals (bandpass_many, 26 band) | ~3,880 | ~7.8s | ~28% |
  | windows (NCC ループ, 25,917 windows/ch) | ~9,700 | ~19.4s | ~69% |
  | **合計 band_gain** | ~14,000 | **~28.2s** | — |

- `analysis::reference` busy 65.2s / 2 calls = 32.6s/pair、
  band_gain はその **~86%**(56.3s/65.2s。元報告の ~83% と整合)。
- windows NCC: 25,917 calls/ch × ~0.374ms 実効(= microbench
  plan.ncc 0.35ms + argmax 等) — 個々の呼出はすでに効率的。
- `ref_phase` の他 family は小さい(similarity ~2.0s、residuals ~1.2s、
  残り合計 ~1.3s / pair)。

**判定: 前提が半分吸収、残半分は依然として有効**。

- `band_ncc`(locate 側)は移行で吸収済み — 対象から外してよい。
- `band_gain`(reference 側)は前提どおり依然として支配的。
- 推奨方向:
  1. **pair 内並列化**: channel × band(2×26=52)もしくは window
     粒度が独立で、run_pool と同じパターンで並列化可能。52 タスクを
     16 core に流せば band_gain ~28s → ~4〜7s 見込み。純粋計算で
     結果順序は既存の ordered slot 方式を踏襲できる低〜中リスク。
  2. band_signals の並列化も同枠組みでカバー可能。
  3. 構造的案(上位互換として検討余地): per-window NCC 25,917 回を
     「帯域信号上の局所相関コレログラム」一括計算へ — ruviform 側に
     新 primitive が必要で、効果見込みは並列化と同桁のため
     並列化を先にやるのが合理的。
- ruviform 直算 API(#168)はこの shape では不適: tpl=4800/sig=8640 で
  `ncc_direct` は 15.8ms vs plan.ncc 0.35ms(実測 ~45× 遅い)。
  コストモデル `prefers_direct` が正しく FFT を選んでいる。

### #54 stage 内並列化

**元の前提**: locate は stage 内で既に並列(worker pool)だが、
cheap/reference は入力/pair 単位で逐次実行。

**移行後実測 + コード確認**:

- reference: `analyze_pair` が `locate_result` artifact を `.iter()`
  で逐次ループ(`crates/al-analysis/src/stage.rs:320`)。canonical では
  pair 数=2(2 capture × 1 source)で ~32.6s/pair を逐次に 2 回。
- cheap も入力逐次(5.0s/3 inputs、expensive 9.1s も同構造)。
- locate 側は既に `run_pool`(available_parallelism)で並列。
  並列化されていないのは analysis 系 stage のみ。

**判定: 依然として有効だが、単独効果は限定的・優先度は #71 より下**。

- pair 粒度並列化の上限は pair 数(canonical では 2×、reference
  66.7→~33s)。入力数に依存し、16 core の恩恵は受けにくい。
- より大きな効果は pair 内部の並列化 = #71 の範囲(band_gain 内部で
  ~7×)。#54 を先に実装しても band_gain がシリアルなままでは Amdahl で
  頭打ちになる → **#71 の pair 内並列化を先行し、#54 は pair/入力
  粒度並列として後続に回すのが合理的**。
- 設計上の注意(実装時): deterministic な出力順序(manifest/artifact
  の記録順)、resume 境界との整合、pair 並列時のメモリ倍加
  (band signal は ~26 band × 2.4M sample 級)。

### #60 residual gather / interleave-copy

**元の前提**: residual 収集の gather と interleave/copy の小項目
(lod.rs は可視化撤去で対象外)。

**移行後実測 + コード確認**:

- `subtract_fit` は既に per-source contiguous `vec::sub_scaled`
  パス(`residual.rs:181-188`) — 転置 gather は移行前の呼出側
  最適化で解消済み。`locate::prepare_residual` busy は canonical で
  2 calls 計 ~0.58s(~0.29s/call、fit_mix + 減算)。
- `residual_iter` span busy は canonical 両 run で 2 calls 計
  8.0〜10.3s(~4〜5s/call)。その大半は residual 上の
  child method sweep(NCC)であり gather/減算ではない。
- interleave/copy 残存: `prepare_input` の frame-major interleave
  ~51〜356ms/input、`prepare_channels` 1.3〜1.4s/input、
  ingest AVI decode ~1.5s 計。合わせて e2e の ~1.5% 程度。

**判定: ほぼ吸収済み — close または最後尾 defer を推奨**。

- gather 本筋は解決済み。残る interleave/copy は ~5〜6s/368s 規模で、
  #76/#71 と比較にならない。lod 系は issue 上も対象外。

## ruviform API が選択肢を変えるか

| API (ruviform issue) | 対象パスへの影響(実測・契約確認) |
|----------------------|-------------------------------|
| `corr` 直算 NCC(#168) | anchor shape(巨大ラグ域)と band_gain window shape(4,800×3,841 lag)ではコストモデル上 FFT が正解。microbench で band shape の `ncc_direct` は ~45× 遅い実測。適用余地なし。なお method 全域 shape(2.4M/2.4M、241 lag)は `prefers_direct` で自動的に NccDirect 経路(133.9ms) — 既に最適経路を通っている。 |
| `CorrelationPlan` eviction 契約 | タスク記述の「ruviform #171」は実際には UI `DataRegistry` eviction/unregister の Issue で、相関 plan とは無関係(番号の不一致を確認)。AL 側 `CorrelationPool` はローカルに `MAX_SHAPES=8`・全破棄 eviction を持つが、**plan build は anchor CPU の ~2% 以下**で最適化主題ではない。 |
| `spec::*`(#169) / `complex_solve`(#167) | #117 で採用済み。今回のホットパスには直接関係しない。 |
| 新規提案余地(記録のみ、実装しない) | (a) 階層化/decimated NCC(anchor fallback 用)、(b) lag-window 部分曲線計算(エスカレーション時の重複区間スキップ)、(c) 帯域信号に対する windowed correlogram(band_gain 一括計算案)。いずれも ruviform primitive として置けるが、呼出側並列化(#71)や anchor 階層化(#76)で先に効果が取れる。 |

## 推奨着手順

**#76 → #71 → #54 → #60**(当初順を維持、根拠は更新):

1. **#76**: e2e の ~68%(anchor ~252s/368s、locate 自体は ~73%)を
   占める最大項。
   widen+full fallback の頻度×単価の削減で ~100〜150s 級の改善余地。
   リスクは anchor 採否セマンティクスの保存(中)。
2. **#71 (band_gain のみ)**: ~28s/pair → pair 内並列化で ~4〜7s
   見込み。低〜中リスクで確実に効く。#76 とは独立で並行も可能。
3. **#54**: pair/入力粒度並列として再定義。#71 後だと残りが小さい
   (reference ~数 s + cheap/expensive)。canonical では ~2× 上限。
4. **#60**: close/defer 推奨。残件は e2e ~1.5%。

依存関係: #76 と #71 は独立。#54 の pair 粒度並列は #71(pair 内並列)
と共有できる worker モデルの設計を踏まえると無駄がない。#60 は独立だが
効果が最小。

## 附録: データファイル

- `perf-reeval-2026-10/canonical.json` — canonical 全 stage 実行
  (e2e 368.0s、stage duration_ms、span busy、anchors/band_gain/ref_phase
  イベント、JOB-mVag45GUT6ASRLW2eo2jW6)
- `perf-reeval-2026-10/canonical-locate.json` — canonical の
  ingest,locate 実行(anchors tier 別 CPU 計時入り、
  JOB-VQIY52klgmYGCl4Yg9IDSy)
- `perf-reeval-2026-10/small.json` — small 全 stage 実行(e2e 126.9s)
- `perf-reeval-2026-10/bench-reeval.txt` — plan build / transform 分離
  microbench 出力
- `tools/perf-reeval/capture.mjs` — 再現用計測ドライバ
  (`node tools/perf-reeval/capture.mjs --run <name>
  [--stages <csv>] [--out <name>] [--keep-job] [--skip-run]`)
- `crates/al-locate/examples/bench_reeval.rs` — 再現用 microbench
  (`cargo run --release -p al-locate --example bench_reeval`)
- 一時 instrumentation は計測後に除去済み(計測時の worktree は dirty、
  `environment.worktree_dirty` を参照)。

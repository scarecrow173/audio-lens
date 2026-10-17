---
id: "DISC-034GZnKJkvCGFXedK2VVUe"
type: "discovery"
status: "active"
title: "reference_analysis analyze_pair 内訳計測: band_gain が支配的"
created: "2026-10-16"
updated: "2026-10-17"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/audio-lens/issues/54"
    - "https://github.com/scarecrow173/audio-lens/issues/60"
    - "https://github.com/scarecrow173/audio-lens/issues/71"
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
    - "./remaining-issues-roadmap.md"
    - "./issue-40-locate-pair-plan-cache.md"
    - "./issue-53-local-lag-fft.md"
  refines: []
  refined-by: []
  derives-from:
    - "./remaining-issues-roadmap.md"
  derived-by:
    - "../specs/valid-fft-ncc-plan-issue-71-p1.md"
    - "../specs/streaming-bandpass-issue-71-p2.md"
    - "../specs/adaptive-ncc-dispatch-issue-71-p3.md"
  verifies: []
  verified-by: []
  references:
    - "../research/issue-71-band-correlation/algorithm-research-report.md"
    - "../research/issue-71-band-correlation/oss-deep-dive.md"
  defers: []
  deferred-by: []
---

# reference_analysis analyze_pair 内訳計測: band_gain が支配的

## 確定方針(2026-10-17 deep-dive 確定)

### スコープ

- **#71 は「既存推定量の互換維持する構造高速化」のみ**に限定。
  multirate / SOS / FIR / ERB / ViSQOL 型の別推定器、音声指紋・H1 等の
  新特徴量は**別 issue**(別 feature_id・メタデータ版管理)とする。
- GCC-PHAT は Valid FFT の**対象外**(位相正規化が FFT 長依存のため
  別契約)。CorrelationPlan の NCC/PHAT は plan/mode で分離する。
- STOMP 型更新・全ラグ prefix 積は演算量上非現実的で**棄却**(調査 §4.E/F)。

### 実施順(各 Phase は独立 PR)

| Phase | 内容 | 期待効果 | 受入 |
| --- | --- | --- | --- |
| **P1 Valid FFT NCC** | NCC 専用 plan で `nextpow2(S)` を選択(GCC-PHAT は `nextpow2(S+L-1)` 維持)。`nextpow2(S) < nextpow2(S+L-1)` の形状のみ適用する**形状適応** | band_ncc/full_ncc: 8.4M→4.2M に半減(レート非依存、ほぼ常時)。band_gain: 44.1kHz で 16K→8K(形状依存、48kHz では効果なし) | 全有効ラグ・peak/tie/順位・曖昧性・テンソルが既存契約内一致 + busy 半減前後を実測 |
| **P2 Streaming Bandpass** | band_gain の帯域波形を全帯域同時保持(26帯域×2入力×2ch ≈ 1.8GB)から、帯域ごと IFFT→窓解析→破棄の逐次生成へ。`prepare_with_bands` が無駄に計算する未使用 `coarse` resample も除去 | ピークメモリ大幅削減(主成果)。CPU は IFFT/NCC 回数不変だが coarse 除去で一部改善の可能性 | 全 Tensor・record・帯域順・guard・400Hz anchor 一致 + peak RSS 削減を実測 |
| **P3 適応 dispatch + Blocked FFT** | (template_len, signal_len, n_lags, memory) で direct SIMD / Valid FFT / Blocked を選択(SciPy `choose_conv_method` 型)。band_ncc の K=237 で直接法、band_gain 窓で 16384→8192×2 を実測比較 | 形状によっては direct 化でさらに改善。Blocked FFT は試算 ~23% 減だが実測必須 | 3方式の形状ベンチで採否。効かない形状は従来経路へ dispatch |

### 共通受入基準

- 全有効ラグ値・peak/tie/順位・evidence(候補/背景/幅/曖昧性)・
  全テンソルが既存契約・scalar↔runtime-dispatch 基準内で一致
- busy・wall・peak RSS・FFT 回数/サイズ・alloc bytes を同一入力・
  同一環境で実測し、効かない形状は従来経路へフォールバック
- `corr0` 意味・guard・polarity 選択・support 計数・`ref.band_*` /
  locate evidence 契約を維持

## P1 実装結果(2026-10-17 実測)

### Valid FFT NCC(plan 分離)の効果

`CorrelationPlan::new_ncc`(FFT `nextpow2(S)`)+ pool キー mode 分離を実装し、
実サンプル run(JOB-91hlZ4jzSCEkXwqbQqwLYg、44.1kHz)で span busy を
比較した(基準: JOB-OImbl1Liin7N49xrIp3APk の変更前計測)。

| 対象 | 変更前 | 変更後 | 変化 |
| --- | --- | --- | --- |
| `locate::band_ncc` busy/呼出 | ~11.0s | ~6.26s | **−43%** |
| `locate::full_ncc` busy/呼出 | ~5.23s | ~3.08s | **−41%** |
| `locate::gcc_phat` busy/呼出 | ~4.6s | ~4.5s | 不変(設計通り) |
| `locate::spectral_envelope` | ~2.1s | ~2.2s | 不変 |
| `analysis::reference`(analyze_pair) | 28.7s/pair | 20.8s/pair | **−27.5%** |

- FFT 長は band_ncc/full_ncc で 8.4M→4.2M、band_gain 窓で 16K→8K と
  半減したが、norm/prefix 積分・バッファ充填など O(S) 線形分が残るため
  実効は −41~43%(band_gain 窓は −27.5% 全体に寄与)に留まった。
- 数値契約: 全有効ラグ一致(argmax 一致含む)・scalar↔dispatch 同等・
  `new_ncc`+`gcc_phat` は panic で検出 — 全テスト pass。

### 重要な追加発見: locate wall は window_anchors_in が支配

P1 適用後も locate ステージ wall は ~274s でほぼ不変だった。
ステージ内の非計装区間を一時計装で分解した結果(JOB-ieq4MI3zdTwiAGbWp4I8eT):

| フェーズ | cap0 | cap1 | 内容 |
| --- | --- | --- | --- |
| `prepare_input`(resample+正規化PCM書出) | 2.54s | 0.24s | 小さい |
| `prepare_channels`(bandpass_many+coarse) | 1.75s | 1.74s | 小さい |
| `locate_channels`(全16メソッド並列) | 7.26s | 6.49s | 既に並列で十分速い |
| **`window_anchors_in`** | **113.8s** | **112.6s** | **locate の ~83%** |

- 514 窓 × `best_window_match`(1回につき最大 4ch-pair `pool.ncc`)。
  802/804 回の narrow+フォールバック呼出、推定 ~3200 NCC 呼出。
- コスト構造(推定): (a) narrow 信号 hi-lo が窓ごとに変動 → pool 形状
  キーが毎回異なり MAX_SHAPES=8 を超えて plan/workspace 再構築が
  頻発、(b) narrow 失敗時の全範囲フォールバック(signal≈2.2M、
  valid FFT で短縮不可の 4.19M FFT×4ch-pair)、(c) narrow NCC 自体
  (valid FFT 化済みだが小さい)。
- つまり **#71 で想定した「band_ncc が e2e 最大」は誤りで、locate
  の真の壁は anchor 探索だった**。band_ncc busy 削減は並列で
  ~7s/pair の壁にしか効かず、wall をほぼ改善しなかった。
- anchor 探索の構造的改善は #71 の P1 スコープ外(新たな形状問題:
  窓ごとの信号長変動→plan 再構築、範囲限定検索の効率化、
  フォールバック削減)。P2/P3 より優先すべき大きい対象として
  **issue #76** に切り出した。

### 訂正: `analysis::reference` の busy 比較は計測アーティファクト

P2 実測時に判明: `analysis::reference` span は `time.busy` と
`time.idle` に大きく分解され、**busy だけの比較は有効な指標では
なかった**。同一ジョブ形式の span close レコードを並べると:

| Job | busy | idle | wall(busy+idle) |
| --- | --- | --- | --- |
| JOB-1HTX(P1 前) | 27.5s / 27.7s | 26.6s / 24.0s | ~52-54s |
| JOB-OImbl(P1 前) | 29.1s / 28.3s | 1.94s / 2.01s | ~30-31s |
| JOB-91hl(P1) | 20.6s / 21.0s | 29.5s / 36.0s | ~50-57s |
| JOB-br2L(P2) | 51.5s / 52.1s | 2.15s / 2.01s | ~53-54s |

- span は他スレッドの仕事と交代するたびに exit/enter し、その
  非占有分が `time.idle` に計上される。スケジューリング状態で
  busy/idle の内訳が大きく揺れるため、**比較には wall
  (busy+idle) または stage 合計を使うべき**。
- これにより上表の「`analysis::reference` 28.7→20.8s −27.5%」は
  アーティファクトと訂正する(wall ではほぼ不変〜数%範囲)。
  なお JOB-OImbl の wall ~30s は計装版バイナリで条件が異なる
  ため直接は比較しない。
- `locate::band_ncc` 等の busy はワーカースレッド内の連続計測で
  idle 分解がほぼなく、P1 の −41~43% はこの影響を受けない。

## P2 実装結果(2026-10-17 実測)

### Streaming Bandpass の効果

`BandFilter`(forward FFT 1 回 + 帯域ごと mask→IFFT→切り戻し)へ
`band_gain` を移行し、全帯域波形の同時保持を解消。`bandpass_many`
は同一 `FilterCore` を経由するため仕様は字句通り一致。

**同一セッション・同一データ(2,205,000 サンプル × 2ch、26 帯域)
での直接対決(release 計測):**

| 経路 | 帯域生成 | 窓解析 | 計/ch |
| --- | --- | --- | --- |
| 旧(`prepare_with_bands` ×2、全帯域保持 + coarse) | 18.6s | 3.8s | ~22.4s |
| 新(`BandFilter` ×2 + 逐次 `band()`) | 17.6s | 3.7s | ~21.4s |

- 帯域生成の速度は同等(計測分解: `band()` ≈ 330ms/回、
  `bandpass_many` の帯域あたりと一致)。差の ~1s/ch は除去した
  coarse resample 分で **P2 はわずかに高速**。
- 帯域 IFFT 52 回が band_gain コストの本体(実ジョブの
  analyze_pair wall ~54s/pair 中 ~80%)。このコストは P2 で
  変わらない設計であり、削減は P3(適応 dispatch/Blocked FFT)
  の対象。

**メモリ(構造検証、`crates/al-math/tests/band_filter.rs` の
counting allocator):**

- len=1<<20、26 帯域: streaming max_live **16.8MB**(帯域 2 本分)
  vs batch max_live **276.8MB** → **16× 削減**。
- 実ジョブ形状(2.2M×8B×26帯域×2ch)換算: 帯域波形の同時保持
  ~915MB → ~35MB。

**実 run の WS(1s 間隔ポーリング × ログ timestamp 相関):**

| 指標 | baseline(main、JOB-H2F4) | P2(JOB-zPn6) | 差 |
| --- | --- | --- | --- |
| **reference ステージ中 peak WS** | **2186MB** | **937MB** | **−57%(−1.25GB)** |
| pair wall(busy+idle) | 56.4s / 57.6s | 57.3s / 64.4s | 同等(実行分散内) |
| プロセス全体 peak WS | 6.35GB | 6.49GB | 不変(locate ステージが支配) |

- P2 の主成果である reference ステージのメモリ削減を確認。
  帯域波形 ~915MB 分＋バッファ差分がそのまま −1.25GB に対応。
- CPU は設計通り同等(coarse 除去分の微差は run 分散より小さい)。
- プロセス peak は locate(`window_anchors_in` 周辺)が支配する
  ため変わらず — 全行程の削減には #76 側が効く。

## P3 実装結果(2026-10-17 実測)

### 適応 NCC dispatch(plan 構築時の形状選択)

`CorrelationPlan::new_ncc` が `(L, S)` から `K = S−L` を求め、
`L·(K+1) ≤ 8·n·log2(n)` かつ `K+1 ≤ 2048` のとき FFT なしの
直接計算 plan(`NccDirect`、分子を `axpy` 蓄積)を選択。
分子だけを置き換え、窓エネルギー prefix 積・normalize・
max_lag クランプ・ゼロエネルギー early return は FFT 経路と
同一のまま。Blocked FFT は形状ベンチで有効域が消えたため棄却
(大きい近似形状は direct、窓・狭帯域形状は Valid FFT が既に
最速のため)。

**形状ベンチ(release、`crates/al-math/tests/direct_ncc_bench.rs`):**

| 形状 | adaptive(`new_ncc`) | Full FFT(`new`) | 選択 |
| --- | --- | --- | --- |
| band_ncc/full_ncc(L=2.2M,K=236) | **79.6ms** | 433.1ms | direct(5.4×) |
| coarse_ncc(L=551K,K=59) | **9.0ms** | 111.2ms | direct(12×) |
| band_gain window(L=4410,K=3528) | **262µs** | 317µs | Valid FFT |
| narrow anchor 小(L=4410,K=8820) | **345µs** | 620µs | Valid FFT |
| narrow anchor 中(L=4410,K=61.7K) | 4.6ms | **3.7ms** | Valid FFT |
| tiny 8kHz(L=800,K=638) | **44.6µs** | 66.9µs | Valid FFT |

誤差は全形状で ≤2e-16(契約内)。`new` 比は P1 前の一律経路との
対比で、adaptive 列には P1 の Valid FFT 効果も含む。

**実サンプル run(JOB-kaXkVSPpXfhPKlVdUTCDip、`--log info`):**

| span | P1+P2 後 | P3 | 変化 |
| --- | --- | --- | --- |
| `locate::band_ncc` | ~6.26s/呼出 | **~1.0s/呼出**(0.85–1.15) | **−84%** |
| `locate::full_ncc` | ~3.08s/呼出 | **~0.32s/呼出**(0.25–0.48) | **−90%** |
| `locate::gcc_phat` | (Full plan) | 2.3–2.9s | 経路不変 |
| `analysis::reference` | ~52s/pair | ~51.8s/pair | 不変(予測通り) |
| プロセス peak WS | 6.49GB | 6.49GB | 不変(locate 支配) |

- P1 前(~11.0s/5.2s)からの累計で band_ncc **−91%**、
  full_ncc **−94%**。
- `band_ncc`/`full_ncc` はワーカー内連続計測で idle 分解が
  ほぼないため busy 比較は有効。
- locate ステージ wall は `window_anchors_in`(非計装 ~227s)
  が支配するため e2e はほぼ不変 — 残ターゲットは #76。
  anchor 探索の narrow/fallback 形状は K が大きく Valid FFT
  を維持するため P3 では変わらない(設計通り)。

## Exploration Goal

#54(stage 内並列化)に着手する前に、reference_analysis の本体である
`analyze_pair`(実測 busy ~27.5s/pair、JOB-OImbl1Liin7N49xrIp3APk)が
どの kernel に時間を使っているかを実測で分解し、次の最適化対象を
証拠ベースで決める。併せて、先行計測で一度浮上した「I/O 支配」仮説を
再検証する。

## 計測方法

- `reference.rs` の 8 セクション境界(lag / gain / spectral / band /
  similarity / channel / dynamics / residuals)と `band_gain.rs` 内部
  (prepare_with_bands / coherence / windows / select_records)に一時
  `Instant` 計装を挿入(計測後に revert 済み)。
- 実行器: 実ジョブと同形状の aligned pair(46s @48kHz、n=2,205,000、
  src 1ch × cap 2ch)を合成して `analyze_pair` を直接呼ぶ一時 example。
- 実ジョブ側は `--log info` の span close 計時(FmtSpan::CLOSE)で
  stage busy / idle を取得済み。

## 計測結果

### analyze_pair セクション別(合成 1ch src ペア、合計 15.0s)

| セクション | 時間 | 比率 |
| --- | --- | --- |
| **band 系(`band_gain::analyze`)** | **11.54s** | **77%** |
| similarity(bandpass×8帯域、gcc_phat、envelope) | 1.78s | 12% |
| residuals(fir_model + R0–R3 の STFT/PSD/band) | 0.99s | 7% |
| channel(fit_mix、stereo coherence) | 0.29s | 2% |
| spectral(STFT×2、Welch PSD/CSD、transfer) | 0.17s | 1% |
| lag(`OverlapCorrPlan`、#53 適用済み) | 0.15s | 1% |
| dynamics / gain | 0.04s | <1% |

### band_gain 内部(1ch あたり)

| 内部処理 | 時間 | 内容 |
| --- | --- | --- |
| `prepare_with_bands` | **4.35s** | `dsp::bandpass_many`: 26帯域スペクトルマスクフィルタ(全信号 FFT + 帯域ごと IFFT)+ coarse resample |
| `windows`(NCC ループ) | **6.92s** | 26帯域 × ~915窓 × `CorrelationPlan::ncc`(1窓 ~0.26ms、窓4800・hop2400・max_lag1920・FFT長16384) |
| coherence / select_records | 0.18s | 軽微 |

帯域ごとの時間はほぼ均一(~260ms/帯域)。窓数が帯域によらず ~915 で
一定のため。

### 実ジョブ(27.5s/pair)との整合

- 実 pair は **src 2ch**(manifest に `channel_0`/`channel_1` の
  band_lag artifact を確認)。band_gain は ch 数に線形なので
  ~23s + similarity/residuals の ch 比例分で **~27.5s と一致**。
- よって reference_analysis の約 83% が `band_gain` に帰属する。

### I/O 支配仮説の再検証(棄却)

- 第1回計測 run で cheap_analysis=123s / reference=106s と観測されたが、
  細粒度計装付き第2回 run では cheap=10.1s / reference=61.6s に復帰。
- write path 自体は健全: `write_atomic`(relaxed tier、fsync なし)+
  4MB chunk で実測 ~200MB/s(実 PCM で 425MB → 2.4s を再現計測)。
- cheap の tensor 書込みは ~2s/入力、reference の tensor+residual WAV
  書込みは ~2s/pair。第1回の低速は dirty-page スロットリングか外部
  スキャンと推定される一過性の環境要因であり、構造的ボトルネックではない。

## Key Issues and Assumptions

- `windows()` は各窓ごとに `CorrelationPlan::ncc` を呼ぶ
  (template=窓4800、signal=窓+2max=8640 → FFT 16384)。
  pair あたり ~47,600 呼(26帯域×915窓×2ch)で、**窓ごとの FFT を
  繰り返す構造自体**がボトルネック。plan 再利用(#52)は済んでおり、
  残る改善はアルゴリズム構造側。
- `bandpass_many` は全信号 FFT + 26 本の帯域マスク IFFT。
  4.35s/ch は変換本数に対し妥当だが、plan/buffer 共有化の余地あり。
- locate stage の `band_ncc`(174s busy)は依然 e2e 最大であり、
  band_gain の改善はその次のターゲット。

## Alternatives and Comparison

| Option | Pros | Cons | Lean |
| --- | --- | --- | --- |
| band_gain の kernel 構造最適化(Valid FFT / Blocked FFT / dispatch + Streaming Bandpass) | reference の ~8割を直撃、#54 と独立に進められる | 数値再現性の検証が必要、設計が要る。窓相関の一括計算は調査で非成立と判明 | yes |
| #54 stage 並列化を先に実施 | 前提 issue は全て解決済み | 支配部(band_gain)の逐次コストは残る。並列化の効果はコア数依存 | maybe |
| band_gain 最適化を #60 に統合 | issue を増やさない | #60 は P3/S の棚卸し枠で規模が合わない(L 級の変更) | no |

## Tentative Conclusions and Hypotheses

- **confirmed**: `analyze_pair` の支配要因は `band_gain::analyze` で
  約83%。内部は「窓ごと FFT-NCC(~47,600呼/pair、~6.9s/ch)」と
  「26帯域スペクトルマスクフィルタ(~4.4s/ch)」が半々。
- **confirmed**: 「I/O 支配」仮説は棄却。write path は ~200MB/s で
  健全、異常 run は環境要因の一過性悪化。
- **confirmed(修正)**: 当初仮説の「窓相関を帯域信号全体の一括相関に
  再構成」は**数学的に不成立**と判明(調査レポート §4.F/§5.3)。
  分子は lag ごとに異なる積時系列で、全ラグ prefix 化は O(NR) に
  なり既定形状では巨大。また時間窓の切り出しと周波数マスクは
  可換でない。
- **confirmed(調査由来)**: `band_gain` の帯域波形同時保持は
  source+capture×2ch で **~1.8GB**(26帯域×2.2M×8B×2入力×2ch)。
  Streaming Bandpass(帯域ごとに IFFT→窓解析→破棄)でピークメモリを
  大幅削減できる。これは CPU 時間よりもメモリ削減が主成果。
- **hypothesis(調査由来、互換維持の最優先案)**: NCC 有効ラグ専用
  FFT長 — テンプレートが信号内に完全収まる valid lag のみなら
  `nextpow2(S+L-1)` ではなく `nextpow2(S)` で十分(STUMPY `sdp.py`
  と同型)。等長 band_ncc で FFT 長を半減できる。**GCC-PHAT には
  適用不可**(周波数ビン位相正規化が FFT 長依存のため plan 分離必須)。
- **hypothesis(調査由来)**: `band_gain` 局所 NCC の Blocked FFT
  (現行 16384×3 → 8192×5、N log N モデルで ~23% 演算量減)は
  実測比較が必要な次候補。OpenCV `templmatch.cpp` が先例。
- **hypothesis(調査由来)**: Direct SIMD / Valid FFT / Blocked FFT の
  形状適応 dispatch(SciPy `choose_conv_method` 型)を基盤 API 化。
- **confirmed(調査由来)**: 音声専用 OSS(librosa multirate、pyfar
  SOS/FIR、ViSQOL 段階整列、Essentia/aubio 帯域集約、指紋系)は
  すべて**別推定器**であり #71 の互換維持最適化には直接使えない。
  導入するなら別 feature_id/メタデータ版管理で別 issue。

## Open Questions

- ~~band_gain 最適化は新 issue とするか、#60 のスコープに編入するか~~
  → **解決(2026-10-16)**: 新 issue #71 として起票。locate の
  `band_ncc` と `band_gain` を同一の構造問題として扱う(L 級のため
  #60 の小項目枠には編入しない)。
- `windows()` の局所 NCC を一括計算化する場合、guard/端点除外・
  polarity 選択・support 計数の既存契約をどう保持するか。
  → 調査により一括計算は非成立と判明。代替は Blocked FFT /
  dispatch で、契約保持の検証項目はレポート §9.1 に列挙済み。
- 改善後の効果上限: band_gain が ~0 になっても reference は
  ~4-5s/pair 残る(similarity/residuals)。locate band_ncc との
  優先度比較をどう置くか。
- (調査由来) Valid FFT 短縮が効く条件 `nextpow2(S+L-1) >
  nextpow2(S)` を実ジョブ形状がどの程度満たすかは未計測。
- (調査由来) Streaming Bandpass 導入時のメモリ上限と #54 並列化の
  相互作用(worker 数 × 帯域バッファ)はピーク RSS 計測後に決める。

## Promotion Candidates

- [ ] spec-doc needed: band_gain 高速化の要件(精度契約・スコープ)が
      固まった時点で作成
- [ ] adr-doc needed: 窓相関の計算構造(plan/一括化)の選択を記録

## Sources and Relations

- 実測 run: `samples/_runs/JOB-OImbl1Liin7N49xrIp3APk`(計装付き、
  --log info)、`JOB-1HTXimeq88yelFUExgzI0O`(I/O 異常 run)
- 上流: `remaining-issues-roadmap.md` の #54/#60/#71 エントリ
- 関連: `issue-40-locate-pair-plan-cache.md`(CorrelationPool)、
  `issue-53-local-lag-fft.md`(OverlapCorrPlan。本調査の lag セクション
  が既に軽いことの根拠)
- 調査証拠(2026-10-17 取り込み、references):
  - `docs/research/issue-71-band-correlation/algorithm-research-report.md`
    — 改善案 A–I の比較・実施 Phase 計画・受入試験要件。14 OSS の
    実装調査に基づく。新規ベンチ未実施の調査結論である点に注意。
  - `docs/research/issue-71-band-correlation/oss-deep-dive.md`
    — 各 OSS の設計思想・具体コード・ライセンスの詳細。

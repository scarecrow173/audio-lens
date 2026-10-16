---
id: "DISC-034GZnKJkvCGFXedK2VVUe"
type: "discovery"
status: "active"
title: "reference_analysis analyze_pair 内訳計測: band_gain が支配的"
created: "2026-10-16"
updated: "2026-10-16"
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
  derived-by: []
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# reference_analysis analyze_pair 内訳計測: band_gain が支配的

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
| band_gain の kernel 構造最適化(窓相関の一括計算・bandpass plan 化) | reference の ~8割を直撃、#54 と独立に進められる | 数値再現性の検証が必要、設計が要る | yes |
| #54 stage 並列化を先に実施 | 前提 issue は全て解決済み | 支配部(band_gain)の逐次コストは残る。並列化の効果はコア数依存 | maybe |
| band_gain 最適化を #60 に統合 | issue を増やさない | #60 は P3/S の棚卸し枠で規模が合わない(L 級の変更) | no |

## Tentative Conclusions and Hypotheses

- **confirmed**: `analyze_pair` の支配要因は `band_gain::analyze` で
  約83%。内部は「窓ごと FFT-NCC(~47,600呼/pair、~6.9s/ch)」と
  「26帯域スペクトルマスクフィルタ(~4.4s/ch)」が半々。
- **confirmed**: 「I/O 支配」仮説は棄却。write path は ~200MB/s で
  健全、異常 run は環境要因の一過性悪化。
- **hypothesis**: `windows()` の各窓 NCC は、局所相互相関を帯域信号
  全体に一括計算する構造(OverlapCorrPlan 系の FFT 化と同系の発想、
  または sliding NCC の共有化)に置き換えられれば数倍効く可能性。
  分子は窓対応の局所相互相関、分母は prefix 二乗和で構成できる。
- **hypothesis**: `prepare_with_bands` の `bandpass_many` は
  FFT plan・中間バッファの共有でさらに削れる可能性。

## Open Questions

- ~~band_gain 最適化は新 issue とするか、#60 のスコープに編入するか~~
  → **解決(2026-10-16)**: 新 issue #71 として起票。locate の
  `band_ncc` と `band_gain` を同一の構造問題として扱う(L 級のため
  #60 の小項目枠には編入しない)。
- `windows()` の局所 NCC を一括計算化する場合、guard/端点除外・
  polarity 選択・support 計数の既存契約をどう保持するか。
- 改善後の効果上限: band_gain が ~0 になっても reference は
  ~4-5s/pair 残る(similarity/residuals)。locate band_ncc との
  優先度比較をどう置くか。

## Promotion Candidates

- [ ] spec-doc needed: band_gain 高速化の要件(精度契約・スコープ)が
      固まった時点で作成
- [ ] adr-doc needed: 窓相関の計算構造(plan/一括化)の選択を記録

## Sources and Relations

- 実測 run: `samples/_runs/JOB-OImbl1Liin7N49xrIp3APk`(計装付き、
  --log info)、`JOB-1HTXimeq88yelFUExgzI0O`(I/O 異常 run)
- 上流: `remaining-issues-roadmap.md` の #54/#60 エントリ
- 関連: `issue-40-locate-pair-plan-cache.md`(CorrelationPool)、
  `issue-53-local-lag-fft.md`(OverlapCorrPlan。本調査の lag セクション
  が既に軽いことの根拠)

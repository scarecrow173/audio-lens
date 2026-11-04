# DESIGN Documents

<!-- doc-suite:generated-index -->

Directory: `docs/designs`

| ID | Title | Status | File |
| --- | --- | --- | --- |
| DESIGN-XhBzf2DS7KALGUBXv2xcuH | AudioLens 設計概要 | draft | [overview.md](./overview.md) |
| DESIGN-034HiHXIY2zsfaMpUtzj8x | shape-adaptive NCC dispatch (issue #71 P3) | draft | [adaptive-ncc-dispatch-issue-71-p3.md](./adaptive-ncc-dispatch-issue-71-p3.md) |
| DESIGN-034DXiPrxAy7If9Jsw6gop | al-math 公開 API のバックエンド中立化と split re/im FFT | approved | [al-math-api-boundary.md](./al-math-api-boundary.md) |
| DESIGN-034Epg45jvGomamsJiywFK | al-math vec プリミティブのモジュール構造と呼出し移行設計 | approved | [al-math-vec-primitives.md](./al-math-vec-primitives.md) |
| DESIGN-034yYLbNoalTLdhNR8vS37 | 解析ステージ・イベント設計 | approved | [analysis-stages.md](./analysis-stages.md) |
| DESIGN-034GsNQAvnJk0iYj6jFzOI | Artifact durability tiers and resume verification | approved | [artifact-durability-tiers-resume-verification.md](./artifact-durability-tiers-resume-verification.md) |
| DESIGN-034Gs3MhX5g3FwTsySzAiT | audio fallback merge: native container authority + symphonia AAC decode | approved | [audio-fallback-merge-native-container-authority-symphonia-aac-decode.md](./audio-fallback-merge-native-container-authority-symphonia-aac-decode.md) |
| DESIGN-T1zXGDg8nn70x2f6GwIeNc | AudioLens 独立 CI: 3 OS lane・依存 dual-mode アクセス・証拠レポート設計 | approved | [audiolens-independent-ci.md](./audiolens-independent-ci.md) |
| DESIGN-034EIsK2SiNLNZYXmumzOe | caller 側最適化: FIR 畳み込み FFT 化・STFT plan 再利用・LeastSquares 列指向の実装設計 | approved | [caller-side-optimizations.md](./caller-side-optimizations.md) |
| DESIGN-034yYLaOKRSfRJMRKiYgQV | 分析データセット設計 | approved | [dataset-schema.md](./dataset-schema.md) |
| DESIGN-034G5QmSv6Yzg756IThZZi | decoded/normalized PCM read-only view 設計 | approved | [decoded-pcm-artifact-view.md](./decoded-pcm-artifact-view.md) |
| DESIGN-034AaszqnyPGmacvhSzO83 | Expensive 追加解析ファミリ設計 | approved | [expensive-families.md](./expensive-families.md) |
| DESIGN-034Rp43HK02Vpr5d3rbv2b | Export 可搬投影と staging publish | approved | [export-portable-projection.md](./export-portable-projection.md) |
| DESIGN-034AasB7gEcddWOpUHrKX8 | ゴールデン fixture マトリクス設計 | approved | [golden-validation.md](./golden-validation.md) |
| DESIGN-035guiAppArch000000000000 | GUI アプリアーキテクチャ(al-service / al-gui) | approved | [gui-app-architecture.md](./gui-app-architecture.md) |
| DESIGN-034yYLa18yjs2eEhATeu20 | インジェストとAVタイミング設計 | approved | [ingest-av.md](./ingest-av.md) |
| DESIGN-034yYLbiufAAGV4Pj6UKJc | ジョブオーケストレーション設計 | approved | [job-orchestration.md](./job-orchestration.md) |
| DESIGN-034GNsEgwjRPsPqegmSEWU | local_lag FFT 化の設計 | approved | [local-lag-fft.md](./local-lag-fft.md) |
| DESIGN-034yYLaqu0owt08BLocFDQ | ソース定位・トラッキング設計 | approved | [localization.md](./localization.md) |
| DESIGN-034Fn25qh0mE8cz5ymYKfD | 合議 argmax 曖昧性検出の実装設計 | approved | [locate-consensus-argmax-ambiguity.md](./locate-consensus-argmax-ambiguity.md) |
| DESIGN-034GMCyaWWhAaXk0LVJK4j | locate 相関 plan ワーカープール | approved | [locate-plan.md](./locate-plan.md) |
| DESIGN-034ENh1Caa1pUN8HBonU2b | 機械的 caller クリーンアップ: bandpass バッチ化・csd インライン化・pass 集約 (A6-A16) の実装設計 | approved | [mechanical-caller-cleanup.md](./mechanical-caller-cleanup.md) |
| DESIGN-034DlwOAFNKWY4VRafyYzo | メディア形式拡大・正規化アーキテクチャと解析精度拡張 | approved | [media-format-analysis-extensions.md](./media-format-analysis-extensions.md) |
| DESIGN-034AasAnWroBLn28EPuQ5Y | MP4 demux とタイミング観測設計 | approved | [mp4-ingest.md](./mp4-ingest.md) |
| DESIGN-034FPWNHT7czdzzGSNBU2Q | NCC plan/workspace 分離と2サイト再利用設計 | approved | [ncc-plan-workspace-reuse.md](./ncc-plan-workspace-reuse.md) |
| DESIGN-034EdQdU4O8Ml4IzUkoFFY | observability foundation: sink・activation・計時の配線設計 | approved | [observability-foundation.md](./observability-foundation.md) |
| DESIGN-034P4pSHzTNg4S497UokuT | pre-migration baseline の実装設計 | approved | [pre-migration-baseline.md](./pre-migration-baseline.md) |
| DESIGN-034DlyKvtWhb8oy1WLAsPV | 残存課題の責務境界と検証設計 | approved | [residual-fixes.md](./residual-fixes.md) |
| DESIGN-034P5eOr1bEaMgiImL3DHd | Ruviform Git 依存・SHA 固定・Private repo CI アクセス設計 | approved | [ruviform-dependency-ci.md](./ruviform-dependency-ci.md) |
| DESIGN-034PdHI4tDTk3iOLaSJO54 | Ruviform media I/O ingest 移行(Issue #100) | approved | [ruviform-ingest-migration.md](./ruviform-ingest-migration.md) |
| DESIGN-6TbEQKqXEHQV8F3VPpYZej | Ruviform 依存の Cargo.lock 正本化設計 | approved | [ruviform-lockfile-source-of-truth.md](./ruviform-lockfile-source-of-truth.md) |
| DESIGN-034P3nlejLy074yTmYSAcJ | Ruviform 移行の処理単位責務マップ | approved | [ruviform-migration-responsibility-map.md](./ruviform-migration-responsibility-map.md) |
| DESIGN-034Dlsvf9IuZSuybyJHn8w | SIMD 算術クレート基盤と DSP カーネル高速化 | approved | [simd-dsp-foundation.md](./simd-dsp-foundation.md) |
| DESIGN-034FPeXgGtwiUXHI98tTsU | Issue 33 確定仕様の文書適用設計 | approved | [spec-residual-contract-finalization.md](./spec-residual-contract-finalization.md) |
| DESIGN-UuNFjPBJ3weQkbJMWWpkh0 | spectral_envelope の fft_size 入口 assert と底上げ契約の明文化 | approved | [spectral-envelope-fft-size.md](./spectral-envelope-fft-size.md) |
| DESIGN-034H77LaRZ9cyfPIcdYe0w | Streaming Bandpass 設計 (issue #71 P2) | draft | [streaming-bandpass-issue-71-p2.md](./streaming-bandpass-issue-71-p2.md) |
| DESIGN-034H5ozhtFnMNl0ZABihBS | Valid FFT NCC plan 設計 (issue #71 P1) | draft | [valid-fft-ncc-plan-issue-71-p1.md](./valid-fft-ncc-plan-issue-71-p1.md) |
| DESIGN-034AasyGL3izivzyzZjS0j | 映像フレーム特徴と A/V 同期設計 | approved | [video-features.md](./video-features.md) |

## Settled <!-- doc-suite:settled-index -->

| ID | Title | Status | File |
| --- | --- | --- | --- |
| DESIGN-034yYLceYoIOUbV56hSfsg | 可視化・エクスポート設計 | superseded | [visualization-export.md](./visualization-export.md) |

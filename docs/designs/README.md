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
| DESIGN-byFehhoxeAg9rB1zUWZc59 | canvas paint 空の設計（DatasetWaveform での振幅正規化） | approved | [canvas-paint-empty.md](./canvas-paint-empty.md) |
| DESIGN-034c5j6d48zGZdh9KX0KM | ci.yml workflow_dispatch 専用化: PR/push trigger 撤去と expected-sha 無条件照合 | approved | [ci-dispatch-only-issue-147.md](./ci-dispatch-only-issue-147.md) |
| DESIGN-034WQJLvsRIXxWUeAMY3BF | Issue #206 OS-L1 GPUI Desktop CI 統合: gpui-verify lane 設計 | approved | [ci-os-l1-gpui-desktop.md](./ci-os-l1-gpui-desktop.md) |
| DESIGN-034SKTJUE6Wsd8z241uUcC | Issue #159 Tracker reconcile 入口(consumer 展開): 共有 pin 更新と別 workflow schedule/manual 入口 | approved | [ci-tracker-reconcile-issue-159.md](./ci-tracker-reconcile-issue-159.md) |
| DESIGN-034yYLaOKRSfRJMRKiYgQV | 分析データセット設計 | approved | [dataset-schema.md](./dataset-schema.md) |
| DESIGN-034G5QmSv6Yzg756IThZZi | decoded/normalized PCM read-only view 設計 | approved | [decoded-pcm-artifact-view.md](./decoded-pcm-artifact-view.md) |
| DESIGN-034AaszqnyPGmacvhSzO83 | Expensive 追加解析ファミリ設計 | approved | [expensive-families.md](./expensive-families.md) |
| DESIGN-034Rp43HK02Vpr5d3rbv2b | Export 可搬投影と staging publish | approved | [export-portable-projection.md](./export-portable-projection.md) |
| DESIGN-034Smu60yQfTlBNu23J8PP | Issue #158 Full CI 統一(安全な第一段階): 全実行化・共通 ci-summary・expected_sha 契約 | approved | [full-ci-ci-summary-issue-158.md](./full-ci-ci-summary-issue-158.md) |
| DESIGN-034AasB7gEcddWOpUHrKX8 | ゴールデン fixture マトリクス設計 | approved | [golden-validation.md](./golden-validation.md) |
| DESIGN-035guiAppArch000000000000 | GUI アプリアーキテクチャ(al-service / al-gui) | approved | [gui-app-architecture.md](./gui-app-architecture.md) |
| DESIGN-034yYLa18yjs2eEhATeu20 | インジェストとAVタイミング設計 | approved | [ingest-av.md](./ingest-av.md) |
| DESIGN-034WsMaALpLsJSgSFhJLyp | Issue #199 impl index 生成対応: front matter 是正と managed marker 設計 | approved | [issue-199-impl-index-generation-design.md](./issue-199-impl-index-generation-design.md) |
| DESIGN-034WuVuaQKt33yJv9vseJb | Issue #202 ir missing-section 補完設計 | approved | [issue-202-ir-missing-sections-design.md](./issue-202-ir-missing-sections-design.md) |
| DESIGN-034XpYvyDBAl0hCspnQyXV | main CI hygiene/verify 失敗の修正設計 | approved | [issue-210-ci-failure-fix-design.md](./issue-210-ci-failure-fix-design.md) |
| DESIGN-034yYLbiufAAGV4Pj6UKJc | ジョブオーケストレーション設計 | approved | [job-orchestration.md](./job-orchestration.md) |
| DESIGN-034GNsEgwjRPsPqegmSEWU | local_lag FFT 化の設計 | approved | [local-lag-fft.md](./local-lag-fft.md) |
| DESIGN-034yYLaqu0owt08BLocFDQ | ソース定位・トラッキング設計 | approved | [localization.md](./localization.md) |
| DESIGN-034Fn25qh0mE8cz5ymYKfD | 合議 argmax 曖昧性検出の実装設計 | approved | [locate-consensus-argmax-ambiguity.md](./locate-consensus-argmax-ambiguity.md) |
| DESIGN-034GMCyaWWhAaXk0LVJK4j | locate 相関 plan ワーカープール | approved | [locate-plan.md](./locate-plan.md) |
| DESIGN-034ENh1Caa1pUN8HBonU2b | 機械的 caller クリーンアップ: bandpass バッチ化・csd インライン化・pass 集約 (A6-A16) の実装設計 | approved | [mechanical-caller-cleanup.md](./mechanical-caller-cleanup.md) |
| DESIGN-034DlwOAFNKWY4VRafyYzo | メディア形式拡大・正規化アーキテクチャと解析精度拡張 | approved | [media-format-analysis-extensions.md](./media-format-analysis-extensions.md) |
| DESIGN-034AasAnWroBLn28EPuQ5Y | MP4 demux とタイミング観測設計 | approved | [mp4-ingest.md](./mp4-ingest.md) |
| DESIGN-WjCNQiQJztajyjGCJIjb0y | Native desktop 初期 slice AL-G1〜G3 の設計 | approved | [native-desktop-initial-slices.md](./native-desktop-initial-slices.md) |
| DESIGN-KmsgLgUSS9WY4blzhdDRqp | Native GUI drawn parity の Spectrogram・Label・Table 拡張と Job/Run 射影の設計 (第5 slice) | draft | [native-gui-drawn-parity-run-mount.md](./native-gui-drawn-parity-run-mount.md) |
| DESIGN-56ekRrs3U0gtF3sylCGYeE | Native GUI の Jobs 対話選択と Run 再開・取消の設計 (第7 slice) | approved | [native-gui-interactive-job-run.md](./native-gui-interactive-job-run.md) |
| DESIGN-Gtwe8oh10Al6yAAXSmfGkL | Native GUI の job 終端 receipt の消費と再射影検証の設計 (第8 slice) | approved | [native-gui-job-receipt-consumption.md](./native-gui-job-receipt-consumption.md) |
| DESIGN-WRNx7ppADH8dxL2tG2HjFK | Native GUI の Job/Run tab mount と Tile::columns 消費の設計 (第6 slice) | draft | [native-gui-job-run-mount.md](./native-gui-job-run-mount.md) |
| DESIGN-rmQlUpDmFZLlonLOUfAGa4 | Native GUI Source 画面の ViewSpec 射影・al-desktop mount・GPUI parity の設計 | draft | [native-gui-product-screen-mount.md](./native-gui-product-screen-mount.md) |
| DESIGN-Hr5Mn9Yw3StP7qXzL2jBdE | Native GUI Spectrogram・Job/Run/Analysis の ViewSpec 射影と drawn parity の設計 | draft | [native-gui-spectrogram-analysis-projection.md](./native-gui-spectrogram-analysis-projection.md) |
| DESIGN-g3Nc8glJv8VXvtDiOoZzpP | Native GUI の Tabs 選択契約の採用と pointer による画面 tab 切替の設計 (第9 slice) | approved | [native-gui-tabs-selection.md](./native-gui-tabs-selection.md) |
| DESIGN-hZqY4x9NK3Xg22TQK3K8mB | Native GUI Waveform/Seek 画面の ViewSpec 射影と Fake 層評価の設計 | draft | [native-gui-waveform-seek-screen.md](./native-gui-waveform-seek-screen.md) |
| DESIGN-034TTalkyZgGfHde64ztJY | Native GUI の Dataset workspace Tabs 選択契約の採用と pointer による tab 切替の設計 (第10 slice) | approved | [native-gui-workspace-tabs-selection.md](./native-gui-workspace-tabs-selection.md) |
| DESIGN-034FPWNHT7czdzzGSNBU2Q | NCC plan/workspace 分離と2サイト再利用設計 | approved | [ncc-plan-workspace-reuse.md](./ncc-plan-workspace-reuse.md) |
| DESIGN-034EdQdU4O8Ml4IzUkoFFY | observability foundation: sink・activation・計時の配線設計 | approved | [observability-foundation.md](./observability-foundation.md) |
| DESIGN-034P4pSHzTNg4S497UokuT | pre-migration baseline の実装設計 | approved | [pre-migration-baseline.md](./pre-migration-baseline.md) |
| DESIGN-034DlyKvtWhb8oy1WLAsPV | 残存課題の責務境界と検証設計 | approved | [residual-fixes.md](./residual-fixes.md) |
| DESIGN-034VbY8pKJZK6aSmZ7mmHA | ruviform 271e2d41 pin 追従の設計と Legacy WebView 棚卸し分類 | approved | [ruviform-271e2d41-rev-watch.md](./ruviform-271e2d41-rev-watch.md) |
| DESIGN-034U1jgDOqHhX2wdbK7ElK | ruviform pin 54439fdc 追従と Spectrogram intensity 実窓検証の設計 | approved | [ruviform-463-spectrogram-intensity.md](./ruviform-463-spectrogram-intensity.md) |
| DESIGN-6qppTQVCUfffmR60C18lKG | ruviform#464 Step C / AL#127 SemanticGuiTest scenario 採用の設計 | approved | [ruviform-464-scenario-adoption.md](./ruviform-464-scenario-adoption.md) |
| DESIGN-034VnUBxnA88YiQguHXjSc | ruviform b02942c2 pin 追従の設計 | approved | [ruviform-b02942c2-rev-watch.md](./ruviform-b02942c2-rev-watch.md) |
| DESIGN-034P5eOr1bEaMgiImL3DHd | Ruviform Git 依存・SHA 固定・Private repo CI アクセス設計 | approved | [ruviform-dependency-ci.md](./ruviform-dependency-ci.md) |
| DESIGN-034VFT8jbz528XiW8ZauIH | ruviform fd1d4d5f pin 追従の設計 | approved | [ruviform-fd1d4d5f-rev-watch.md](./ruviform-fd1d4d5f-rev-watch.md) |
| DESIGN-034PdHI4tDTk3iOLaSJO54 | Ruviform media I/O ingest 移行(Issue #100) | approved | [ruviform-ingest-migration.md](./ruviform-ingest-migration.md) |
| DESIGN-034RN2eWIExEejH7mYYOyj | Issue #139 Ruviform 共通 I/O 移行の残存分類 — StagedFile 移行記録・回帰契約・Structured/Archive 採否 | draft | [ruviform-io-migration-issue-139.md](./ruviform-io-migration-issue-139.md) |
| DESIGN-6TbEQKqXEHQV8F3VPpYZej | Ruviform 依存の Cargo.lock 正本化設計 | approved | [ruviform-lockfile-source-of-truth.md](./ruviform-lockfile-source-of-truth.md) |
| DESIGN-17NiTSZp7kRH6pAW7EFwiO | SinkConfig::span_events 追従設計 — stderr span close 復旧 (AL#127) | approved | [ruviform-logging-span-events-compat.md](./ruviform-logging-span-events-compat.md) |
| DESIGN-034P3nlejLy074yTmYSAcJ | Ruviform 移行の処理単位責務マップ | approved | [ruviform-migration-responsibility-map.md](./ruviform-migration-responsibility-map.md) |
| DESIGN-uCvLq1YDJau0qmwrQqB52k | al-observability logging の ruviform-observability 委譲設計 (AL-OBS-01) | approved | [ruviform-observability-logging-migration.md](./ruviform-observability-logging-migration.md) |
| DESIGN-034RIK5GkdCeUVI4sbrxPb | Issue #148 sccache コンパイルキャッシュ採用設計 — lane 選定・namespace・kill switch・計測 | approved | [sccache-compile-cache-issue-148.md](./sccache-compile-cache-issue-148.md) |
| DESIGN-034Dlsvf9IuZSuybyJHn8w | SIMD 算術クレート基盤と DSP カーネル高速化 | approved | [simd-dsp-foundation.md](./simd-dsp-foundation.md) |
| DESIGN-034FPeXgGtwiUXHI98tTsU | Issue 33 確定仕様の文書適用設計 | approved | [spec-residual-contract-finalization.md](./spec-residual-contract-finalization.md) |
| DESIGN-UuNFjPBJ3weQkbJMWWpkh0 | spectral_envelope の fft_size 入口 assert と底上げ契約の明文化 | approved | [spectral-envelope-fft-size.md](./spectral-envelope-fft-size.md) |
| DESIGN-034TX8udVdtrH0SBVRBMly | Spectrogram tab hang 解消の設計（ruviform pin 追従と回帰検証） | approved | [spectrogram-tab-hang.md](./spectrogram-tab-hang.md) |
| DESIGN-034H77LaRZ9cyfPIcdYe0w | Streaming Bandpass 設計 (issue #71 P2) | draft | [streaming-bandpass-issue-71-p2.md](./streaming-bandpass-issue-71-p2.md) |
| DESIGN-034Vioow1Mm5KhHailH7rG | time_map materialize と SyncTimeline wire 経路の設計 | approved | [timemap-locate-materialize.md](./timemap-locate-materialize.md) |
| DESIGN-034H5ozhtFnMNl0ZABihBS | Valid FFT NCC plan 設計 (issue #71 P1) | draft | [valid-fft-ncc-plan-issue-71-p1.md](./valid-fft-ncc-plan-issue-71-p1.md) |
| DESIGN-034AasyGL3izivzyzZjS0j | 映像フレーム特徴と A/V 同期設計 | approved | [video-features.md](./video-features.md) |
| DESIGN-HzMSjoqoRH2ibQWn8inlUp | al-gui webview host の driver-neutral API 移行の設計 | approved | [webview-host-api-migration.md](./webview-host-api-migration.md) |

## Settled <!-- doc-suite:settled-index -->

| ID | Title | Status | File |
| --- | --- | --- | --- |
| DESIGN-034RNl7ph9ojvXsoInYjLx | Issue #149 変更影響ベース CI — AudioLens 分類規約・期待集合 Fail-Closed Gate・Shadow Mode 移行 | superseded | [impact-based-ci-issue-149.md](./impact-based-ci-issue-149.md) |
| DESIGN-034yYLceYoIOUbV56hSfsg | 可視化・エクスポート設計 | superseded | [visualization-export.md](./visualization-export.md) |

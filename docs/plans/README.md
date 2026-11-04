# PLAN Documents

<!-- doc-suite:generated-index -->

Directory: `docs/plans`

| ID | Title | Status | File |
| --- | --- | --- | --- |
| PLAN-034DXViZtHIbCiHSxsn1O0 | al-math 公開 API 中立化と split re/im FFT 境界 | approved | [al-math-api-boundary.md](./al-math-api-boundary.md) |
| PLAN-034EpNVDehwMClwkJioA7x | al-math vec プリミティブ集約と dsp ファサード移行 | approved | [al-math-vec-primitives.md](./al-math-vec-primitives.md) |
| PLAN-034GsQ4Xf3tqGpUXx6bUK2 | Implement artifact durability tiers and resume verification | approved | [implement-artifact-durability-tiers.md](./implement-artifact-durability-tiers.md) |
| PLAN-dmszyxRb1oaST3thMlruEm | AudioLens 独立 CI(#142)の実装計画 | approved | [implement-audiolens-independent-ci.md](./implement-audiolens-independent-ci.md) |
| PLAN-034G5TxZGhHh39SQ8S5su4 | decoded/normalized PCM read-only view の実装 | approved | [implement-decoded-pcm-artifact-view.md](./implement-decoded-pcm-artifact-view.md) |
| PLAN-034P4YN0NpG6X9OObVjUYA | pre-migration baseline の実装計画 | approved | [implement-pre-migration-baseline.md](./implement-pre-migration-baseline.md) |
| PLAN-034Gs73yLxitjnNfDd2Cz4 | implement symphonia AAC-LC audio decode fallback in MP4 | approved | [implement-symphonia-aac-lc-audio-decode-fallback-in-mp4.md](./implement-symphonia-aac-lc-audio-decode-fallback-in-mp4.md) |
| PLAN-035bGhJkLmNpQrStUvWxYz | メディア形式拡大・正規化アーキテクチャと解析精度拡張 | approved | [media-format-analysis-extensions.md](./media-format-analysis-extensions.md) |
| PLAN-034Eesy8DzTxxROXy4lSFg | observability foundation 導入 | approved | [observability-foundation.md](./observability-foundation.md) |
| PLAN-034dLdqZ28TeDZBM9XH3rg | 残存 Issue #28〜#33 の修正・仕様判断計画 | approved | [residual-fixes.md](./residual-fixes.md) |
| PLAN-034P3zvTd9Bg7Ps3kxAKOd | Ruviform 基盤移行の段階実施計画 | approved | [ruviform-foundation-migration.md](./ruviform-foundation-migration.md) |
| PLAN-4ifedHAkxHtbglJWu8ESIL | Ruviform Git 依存を Cargo.lock 正本へ移行する実装計画 | approved | [ruviform-lockfile-source-of-truth.md](./ruviform-lockfile-source-of-truth.md) |
| PLAN-035aRtYvBnMxQwErTyUiOp | SIMD 算術クレート基盤と DSP カーネル高速化 | approved | [simd-dsp-foundation.md](./simd-dsp-foundation.md) |
| PLAN-NvzYVuupuqGBJmuEuJJPjA | spectral_envelope の fft_size 入口 assert 実装 | approved | [spectral-envelope-fft-size.md](./spectral-envelope-fft-size.md) |

## Settled <!-- doc-suite:settled-index -->

| ID | Title | Status | File |
| --- | --- | --- | --- |
| PLAN-034z7Go0WvShlVKkU7NCCH | Analysis Dataset ストアの実装 | completed | [analysis-dataset.md](./analysis-dataset.md) |
| PLAN-034z8FkpJb4ejhkehZ3pgc | analysis-stages | completed | [analysis-stages.md](./analysis-stages.md) |
| PLAN-034EJg701ux76Ga7UWPP0j | caller 側最適化チェーン: 畳み込み FFT 化・STFT plan 再利用・LeastSquares 列指向 (A1/A3/A4) | completed | [caller-side-optimizations.md](./caller-side-optimizations.md) |
| PLAN-034AaBrYBgbeSTDlZzbaGl | Expensive 追加解析ファミリの実装 | completed | [expensive-families.md](./expensive-families.md) |
| PLAN-034Rp5hEUjbyXqjbKwlxDA | Export 可搬投影の実装 | completed | [export-portable-projection.md](./export-portable-projection.md) |
| PLAN-034AaBtyLhpTxk9Vb4eftR | ゴールデン fixture マトリクスの実装 | completed | [golden-validation.md](./golden-validation.md) |
| PLAN-034z0yzgmf43IGEQwzG9gE | インジェストとAVタイミングの実装 | completed | [implement-ingest-av.md](./implement-ingest-av.md) |
| PLAN-034yZp5w0ziN8jQO61t3nW | ジョブオーケストレーション基盤の実装 | completed | [implement-job-orchestration-foundation.md](./implement-job-orchestration-foundation.md) |
| PLAN-034Fo8JsxsjegvrZWwahTO | locate 合議 argmax 曖昧性検出の実装 | completed | [implement-locate-argmax-ambiguity.md](./implement-locate-argmax-ambiguity.md) |
| PLAN-034FQfhDbOOCKmh4QhnP51 | NCC plan/workspaceを分離し2サイトを再利用へ移す | completed | [implement-ncc-plan-workspace-reuse.md](./implement-ncc-plan-workspace-reuse.md) |
| PLAN-034FPmSqSPhY2WlFZJB0VY | Issue 33 確定仕様を正式文書へ適用する | completed | [implement-spec-residual-contract-finalization.md](./implement-spec-residual-contract-finalization.md) |
| PLAN-034GNwfwp4fXb0aKjI0LOC | local_lag FFT 厳密再現化の実装計画 | completed | [local-lag-fft.md](./local-lag-fft.md) |
| PLAN-034z7HA18FxYMCQPBZ6Gg2 | ソース定位・トラッキングの実装 | completed | [locate-and-tracking.md](./locate-and-tracking.md) |
| PLAN-034GMK73n1nSCbsGEfC6Np | locate 相関 plan ワーカープール導入 | completed | [locate-plan.md](./locate-plan.md) |
| PLAN-034EO41zt4R9uACMi7eaFH | 機械的 caller クリーンアップ: bandpass バッチ化・csd インライン化・pass 集約 (A6-A16) | completed | [mechanical-caller-cleanup.md](./mechanical-caller-cleanup.md) |
| PLAN-034AaBsJ00thIonv8bp54N | MP4 demux とタイミング観測の実装 | completed | [mp4-ingest.md](./mp4-ingest.md) |
| PLAN-034AaBrlpxZEKpyEGpUmmk | 映像フレーム特徴と A/V 同期の実装 | completed | [video-av-features.md](./video-av-features.md) |
| PLAN-034z8FkQ8VWjuojrvS0HhT | visualization-export | completed | [visualization-export.md](./visualization-export.md) |

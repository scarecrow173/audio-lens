# Architecture Decision Records

<!-- doc-suite:generated-index -->

Directory: `docs/adr`

| ID | Title | Status | File |
| --- | --- | --- | --- |
| ADR-034DXb88VbwPFg3jZqRxkv | al-math 公開 API からバックエンド型を排除し FFT 境界を split re/im 表現にする | accepted | [al-math-api-fft-split-re-im.md](./al-math-api-fft-split-re-im.md) |
| ADR-034EoHr8hchpVwmk758yL4 | al-math vec プリミティブ層のスライス公開境界と dsp ファサード消費経路 | accepted | [al-math-vec-primitives-slice-boundary.md](./al-math-vec-primitives-slice-boundary.md) |
| ADR-034yYy5Xx8zznKyVgAgdtP | Analysis Dataset の保存形式を分割する | proposed | [analysis-dataset-storage.md](./analysis-dataset-storage.md) |
| ADR-034AapHIAOqZfyrLPIa14w | 映像フレームデコードは非圧縮 AVI に限定する | accepted | [avi.md](./avi.md) |
| ADR-034yYyqyqy2CzywC1zpHME | 可視化を派生データセットとして分離する | proposed | [derived-visualization-dataset.md](./derived-visualization-dataset.md) |
| ADR-034yYyq7wrVYD7X3NvjeX0 | 不変ジョブと再開可能なステージ実行を採用する | proposed | [immutable-resumable-job.md](./immutable-resumable-job.md) |
| ADR-034yZb7m1ru4R4k7m8jLme | 実装言語に Rust を採用しコア処理を自前モジュールで実装する | accepted | [implementation-stack.md](./implementation-stack.md) |
| ADR-034EdkLsGVFb43OUttkjdR | ログ sink・format・activation の契約 | accepted | [logging-sink-format-activation-contract.md](./logging-sink-format-activation-contract.md) |
| ADR-034yYy5vgxc67xWxy1A4kH | 観測出力を評決より優先する | proposed | [measurement-over-verdict.md](./measurement-over-verdict.md) |
| ADR-034DxdUT164IPZdcbKOLQo | MediaDecoder は native 優先の registry と段階的 backend 導入を採用する | accepted | [media-decoder-backend.md](./media-decoder-backend.md) |
| ADR-034AapIjxqSGmopfqOgcJ5 | MP4 は demux とタイミング観測のみ対応しコーデック decode は行わない | accepted | [mp4-demux-decode.md](./mp4-demux-decode.md) |
| ADR-034yYy6pBGTFj1fMRdqJdg | locate 受理に複数手法の合議を要求する | proposed | [multi-method-locate-consensus.md](./multi-method-locate-consensus.md) |
| ADR-034EdkF9d9cVHx4NSumGSL | observability を専用 crate に集約する | accepted | [observability-dedicated-crate.md](./observability-dedicated-crate.md) |
| ADR-034DlyK8LKvSfTztL5Bfu5 | 残存課題を責務境界で修正し仕様判断を分離する | proposed | [residual-issue-boundaries.md](./residual-issue-boundaries.md) |
| ADR-035pmRjqxvweesfcw4L90A | 独自 SIMD 算術クレートの基盤に pulp を採用する | accepted | [simd-arithmetic-pulp.md](./simd-arithmetic-pulp.md) |
| ADR-034EoHxzsvsHp9BOdCK2Cn | SIMD reduction 順序は lane アキュムレートと FMA を許容し等価性を許容誤差で定義する | accepted | [simd-reduction-lane-fma.md](./simd-reduction-lane-fma.md) |
| ADR-z2U7XfOoFCqimI9EpcPc8B | spectral_envelope の fft_size を入口 assert で非ゼロ 2 冪必須とし底上げ規則を維持する | accepted | [spectral-envelope-fft-size-assert.md](./spectral-envelope-fft-size-assert.md) |
| ADR-034EdkLSef7miMOT11Ca50 | stage 計時を manifest に永続化する | accepted | [stage-timing-manifest-persistence.md](./stage-timing-manifest-persistence.md) |

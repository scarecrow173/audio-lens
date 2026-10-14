# Implementation Record Documents

Directory: `docs/impl/ir`

実施済みタスクの実装記録。タスクとの対応は各ファイルの
`implements` リレーションを参照。

## Records

| Task | Record |
| --- | --- |
| analysis-candidate-events | [analysis-candidate-events.md](./analysis-candidate-events.md) |
| analysis-cheap-stage | [analysis-cheap-stage.md](./analysis-cheap-stage.md) |
| analysis-cli-wiring | [analysis-cli-wiring.md](./analysis-cli-wiring.md) |
| analysis-dataset-foundation | [analysis-dataset-foundation.md](./analysis-dataset-foundation.md) |
| analysis-dsp-intermediates | [analysis-dsp-intermediates.md](./analysis-dsp-intermediates.md) |
| analysis-expensive-stage | [analysis-expensive-stage.md](./analysis-expensive-stage.md) |
| analysis-integration-tests | [analysis-integration-tests.md](./analysis-integration-tests.md) |
| analysis-reference-stage | [analysis-reference-stage.md](./analysis-reference-stage.md) |
| al-analysis-vec-migration | [al-analysis-dsp-facade-vec-migration.md](./al-analysis-dsp-facade-vec-migration.md) |
| al-locate-vec-migration | [al-locate-dsp-re-export-vec-migration.md](./al-locate-dsp-re-export-vec-migration.md) |
| al-math-vec-primitives | [al-math-vec-primitives.md](./al-math-vec-primitives.md) |
| al-media-skeleton | [al-media-skeleton.md](./al-media-skeleton.md) |
| artifact-dictionary | [artifact-dictionary.md](./artifact-dictionary.md) |
| avi-container-ingest-and-video-fixtures | [avi-container-ingest-and-video-fixtures.md](./avi-container-ingest-and-video-fixtures.md) |
| caller-opt-bench-baseline-characterization | [caller-side-optimizations-bench-baseline-and-characterization.md](./caller-side-optimizations-bench-baseline-and-characterization.md) |
| caller-opt-dsp-conv-overlap-save | [caller-side-optimizations-dsp-conv-overlap-save-and-fir-model-migration.md](./caller-side-optimizations-dsp-conv-overlap-save-and-fir-model-migration.md) |
| caller-opt-least-squares-column-api | [caller-side-optimizations-least-squares-column-api.md](./caller-side-optimizations-least-squares-column-api.md) |
| caller-opt-measurement-ir-closeout | [caller-side-optimizations-post-measurement-closeout.md](./caller-side-optimizations-post-measurement-closeout.md) |
| caller-opt-stft-fft-plan-hoist | [caller-side-optimizations-stft-fft-plan-hoist.md](./caller-side-optimizations-stft-fft-plan-hoist.md) |
| chunked-tensor-store | [chunked-tensor-store.md](./chunked-tensor-store.md) |
| columnar-table-store | [columnar-table-store.md](./columnar-table-store.md) |
| core-domain-types | [core-domain-types.md](./core-domain-types.md) |
| dataset-integration-tests | [dataset-integration-tests.md](./dataset-integration-tests.md) |
| dataset-reader | [dataset-reader.md](./dataset-reader.md) |
| dataset-skeleton | [dataset-skeleton.md](./dataset-skeleton.md) |
| dataset-stage | [dataset-stage.md](./dataset-stage.md) |
| dataset-writer-provenance | [dataset-writer-provenance.md](./dataset-writer-provenance.md) |
| export-package | [export-package.md](./export-package.md) |
| ingest-integration-tests | [ingest-integration-tests.md](./ingest-integration-tests.md) |
| ingest-stage-and-cli-wiring | [ingest-stage-and-cli-wiring.md](./ingest-stage-and-cli-wiring.md) |
| init-rust-workspace | [init-rust-workspace.md](./init-rust-workspace.md) |
| job-manifest-registry | [job-manifest-registry.md](./job-manifest-registry.md) |
| locate-argmax-ambiguity-closeout | [locate-argmax-ambiguity-closeout.md](./locate-argmax-ambiguity-closeout.md) |
| locate-argmax-tie-eps-characterization | [locate-argmax-tie-eps-calibration.md](./locate-argmax-tie-eps-calibration.md) |
| locate-consensus | [locate-consensus.md](./locate-consensus.md) |
| locate-integration-tests | [locate-integration-tests.md](./locate-integration-tests.md) |
| locate-methods | [locate-methods.md](./locate-methods.md) |
| locate-mixer | [locate-mixer.md](./locate-mixer.md) |
| locate-residual | [locate-residual.md](./locate-residual.md) |
| locate-result-schema | [locate-result-schema.md](./locate-result-schema.md) |
| locate-skeleton-dsp | [locate-skeleton-dsp.md](./locate-skeleton-dsp.md) |
| locate-stage-cli | [locate-stage-cli.md](./locate-stage-cli.md) |
| locate-tracking | [locate-tracking.md](./locate-tracking.md) |
| mech-cleanup-a15-interleave-block-writes | [mech-cleanup-a15-interleave-block-writes.md](./mech-cleanup-a15-interleave-block-writes.md) |
| mech-cleanup-a16-row-major-fast-paths | [mech-cleanup-a16-row-major-fast-paths.md](./mech-cleanup-a16-row-major-fast-paths.md) |
| mech-cleanup-a6-bandpass-batching | [mech-cleanup-a6-bandpass-batching.md](./mech-cleanup-a6-bandpass-batching.md) |
| mech-cleanup-a7-a8-csd-inlining | [mech-cleanup-a7-a8-csd-inlining.md](./mech-cleanup-a7-a8-csd-inlining.md) |
| mech-cleanup-a9-a12-cheap-pass-consolidation | [mech-cleanup-a9-a12-cheap-pass-consolidation.md](./mech-cleanup-a9-a12-cheap-pass-consolidation.md) |
| mech-cleanup-bench-baseline-characterization | [mech-cleanup-bench-baseline-characterization.md](./mech-cleanup-bench-baseline-characterization.md) |
| mech-cleanup-measurement-ir-closeout | [mech-cleanup-measurement-ir-closeout.md](./mech-cleanup-measurement-ir-closeout.md) |
| minimal-cli | [minimal-cli.md](./minimal-cli.md) |
| observability-cli-wiring | [observability-cli-wiring.md](./observability-cli-wiring.md) |
| observability-crate | [observability-crate.md](./observability-crate.md) |
| observability-criterion-samply | [observability-criterion-samply.md](./observability-criterion-samply.md) |
| observability-leaf-instrumentation | [observability-leaf-instrumentation.md](./observability-leaf-instrumentation.md) |
| observability-stage-timing | [observability-stage-timing.md](./observability-stage-timing.md) |
| resume-reproduction-tests | [resume-reproduction-tests.md](./resume-reproduction-tests.md) |
| stage-runner | [stage-runner.md](./stage-runner.md) |
| al-math-envelope-fft-size-assert | [spectral-envelope-fft-size-entry-assert.md](./spectral-envelope-fft-size-entry-assert.md) |
| timing-observation | [timing-observation.md](./timing-observation.md) |
| vec-primitives-verification-bench | [vec-primitives-verification-and-bench.md](./vec-primitives-verification-and-bench.md) |
| viz-dataset-reads | [viz-dataset-reads.md](./viz-dataset-reads.md) |
| viz-events-summary | [viz-events-summary.md](./viz-events-summary.md) |
| viz-export-integration-tests | [viz-export-integration-tests.md](./viz-export-integration-tests.md) |
| viz-spectrogram-tiles | [viz-spectrogram-tiles.md](./viz-spectrogram-tiles.md) |
| viz-stage-cli | [viz-stage-cli.md](./viz-stage-cli.md) |
| viz-views | [viz-views.md](./viz-views.md) |
| viz-waveform-lod | [viz-waveform-lod.md](./viz-waveform-lod.md) |
| wav-audio-store | [wav-audio-store.md](./wav-audio-store.md) |
| wav-decode-and-hash | [wav-decode-and-hash.md](./wav-decode-and-hash.md) |

| spec-residual-contract-finalization | [spec-residual-contract-finalization.md](./spec-residual-contract-finalization.md) |

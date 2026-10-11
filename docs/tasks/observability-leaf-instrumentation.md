---
id: "TASK-034EevjKiDgtt3CwF4qTtW"
type: "task"
status: "todo"
title: "leaf crate への span 計装"
created: "2026-10-11"
updated: "2026-10-11"
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
  implements:
    - "docs/plans/observability-foundation.md"
  implemented-by: []
  depends-on:
    - "docs/tasks/observability-crate.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related: []
  refines: []
  refined-by: []
  derives-from:
    - "docs/designs/observability-foundation.md"
    - "docs/adr/observability-dedicated-crate.md"
  derived-by: []
  verifies: []
  verified-by:
    - "docs/test-specs/observability-foundation.md"
  references: []
  defers: []
  deferred-by: []
---

# leaf crate への span 計装

## Work

計装対象 leaf crate に design の span 配置マップどおり span を
置く(plan T4)。leaf crate は `tracing` 標準 API
(`info_span!`/`instrument`/`info!`/`debug!` 等)で emit のみを
行い、`al-observability`・subscriber の存在を知らない。
span 名は `&'static str`、field は共通語彙(`job_id`・
`stage`・`duration_ms`・`len`・`sha256`・`input_id`・
`method`)の metadata-only とする。

- al-locate(`methods.rs`): `locate::full_ncc`・
  `locate::band_ncc`・`locate::gcc_phat`・
  `locate::spectral_envelope`(field: `method`, `len`)
- al-analysis(`stage.rs`/`cheap.rs`/`reference.rs`/`expensive/`):
  `analysis::cheap`・`analysis::reference`・`analysis::region`・
  `analysis::<family>`(field: `input_id`, `method`, `len`,
  `duration_ms`)
- al-media(`stage.rs`/`decoder.rs`): `media::ingest`
  (IngestStage::ingest_input)・`media::decode`
  (DecoderRegistry::decode)・`media::probe`
  (MediaDecoder::probe)(field: `input_id`, `sha256`, `len`,
  `method`)
- al-viz(`stage.rs`/`timeline.rs`/`views.rs`/`report.rs`/
  `summary.rs`/`tiles.rs`/`lod.rs`/`package.rs`/`golden.rs`):
  `viz::<product>`(timeline・views・report・summary・
  tiles・lod)・`viz::package`・`viz::golden`(field: `len`,
  `duration_ms`)
- al-dataset(`stage.rs`): `dataset::write`
  (DatasetStage::run)(field: `input_id`, `len`,
  `duration_ms`)
- 各 `Cargo.toml` に `tracing.workspace = true` を追加する
  (al-job は T3 で実施済み。al-core・al-math・al-testkit には
  tracing 依存を追加しない)

## Classification

- `normal-plan-task`

## Upstream Documents

- `docs/plans/observability-foundation.md`
- `docs/designs/observability-foundation.md`(span 配置マップ)
- `docs/adr/observability-dedicated-crate.md`(leaf crate →
  `tracing` のみの依存方向)

## Dependencies

- `docs/tasks/observability-crate.md`(T1): `tracing`
  workspace dep が前提

## Affected Files

- `crates/al-locate/src/methods.rs` 他(上記 span 配置)
- `crates/al-analysis/src/{stage,cheap,reference}.rs` 等
- `crates/al-media/src/{stage,decoder}.rs`
- `crates/al-viz/src/{stage,timeline,views,report,summary,
  tiles,lod,package,golden}.rs`
- `crates/al-dataset/src/stage.rs`
- `crates/al-locate/Cargo.toml`・`crates/al-analysis/Cargo.toml`・
  `crates/al-media/Cargo.toml`・`crates/al-viz/Cargo.toml`・
  `crates/al-dataset/Cargo.toml`: `tracing.workspace = true`

## Done When

- [ ] 5 leaf crate に design の span map どおりの span が置か
  れ、span 名が `&'static str` の `<crate>::<unit>` 形である
- [ ] field が metadata-only 語彙のみで、raw PCM・巨大
  バッファを載せない
- [ ] leaf crate が `tracing` のみに直接依存し、
  `tracing-subscriber`/`tracing-appender`/`al-observability`
  への直接依存がない
- [ ] `AL_LOG=al_locate=debug` で al-locate の内部 span event
  が観測できる(T2 完了後の統合確認)

## Verification

- [ ] `mise exec -- cargo test --workspace` が green
- [ ] `mise exec -- cargo tree --workspace --edges normal` で
  leaf crate の normal 依存に `tracing` 以外の observability
  系がない
- [ ] `mise exec -- cargo clippy`・`cargo fmt --check` が green

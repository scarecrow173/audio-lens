---
id: "IMPL-01M4PAYJMMJP9RCPBAT3ZX1X29"
type: "impl"
status: "completed"
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
    - "docs/tasks/observability-leaf-instrumentation.md"
  implemented-by: []
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related: []
  refines: []
  refined-by: []
  derives-from:
    - "docs/designs/observability-foundation.md"
  derived-by: []
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
metadata:
  experiments:
    adopted: []
    rejected: []
---

# leaf crate への span 計装

## Summary

design の span 配置マップどおり、5 leaf crate(al-locate・
al-analysis・al-media・al-viz・al-dataset)に `tracing` の
`info_span!` を配置した。全て同期 fn のため
`let span = info_span!(...); let _enter = span.enter();` または
`span.in_scope(..)` 形。al-core・al-math・al-testkit は計装対象外。

## Context

対応 task は `docs/tasks/observability-leaf-instrumentation.md`、
plan は `docs/plans/observability-foundation.md`、上位設計は
`docs/designs/observability-foundation.md`(span 配置マップ)と
ADR `docs/adr/observability-dedicated-crate.md`、検証仕様は
`docs/test-specs/observability-foundation.md`。前提は T1/T2
(`docs/impl/ir/observability-crate.md`・
`docs/impl/ir/observability-cli-wiring.md`)の logging 基盤と
配線。

## Implementation

### Changes

- 各 leaf crate の `Cargo.toml` に `tracing.workspace = true` を
  追加(al-cli・al-job は T2/T3 で済み)
- al-locate `methods.rs`: `FullNcc::locate`・
  `FullNcc::locate_coarse_to_fine`・`BandNcc::locate`・
  `BandNcc::locate_prepared`・`GccPhat::locate`・
  `SpectralEnvelope::locate` の入口に
  `locate::{full_ncc,band_ncc,gcc_phat,spectral_envelope}` span
  (field: `method`, `len_a` = template 長, `len_b` = signal 長)
- al-analysis `stage.rs`: `cheap::analyze_input` 呼出しを
  `analysis::cheap`、`reference::analyze_pair` 呼出しを
  `analysis::reference` の `in_scope` で包む
  (field: `input_id`, `len`)
- al-analysis `expensive/mod.rs`: `analyze_region` 入口に
  `analysis::region`(field: `input_id`, `len` = region サンプル
  概算)、各 family `run` 呼出しを `analysis::<family>`(channel・
  codec・dynamics・ir・noise・timing・transfer)で包む
  (field: `input_id`, `method`, `len` = region サンプル数)
- al-media `stage.rs`: `IngestStage::ingest_input` 入口に
  `media::ingest`(field: `input_id`、`len`/`sha256` は読込・hash
  後に `span.record` で追記)
- al-media `decoder.rs`: `DecoderRegistry::decode` 入口に
  `media::decode`(`method` は backend resolve 後に `span.record`)、
  `NativeDecoder::probe` 入口に `media::probe`(field: `method`,
  `len` = 入力バイト数)
- al-viz: product 関数の入口に `viz::<product>` span を配置 —
  `timeline::build`(`viz::timeline`, `input_id` + `len` =
  capture frames を record)、`lod::build`(`viz::lod`, `input_id`
  + `len` = frames を record)、`tiles::build` /
  `tiles::validity_masks`(`viz::tiles`, `method` で区別)、
  `views::*_views`/`events_view` 6 関数(`viz::views`, `method` =
  関数名)、`summary::summary_body`(`viz::summary`, `len` =
  produced 数)、`report::generate`(`viz::report`)、
  `package::build`(`viz::package`, `len` = 収録 file 数を
  record)、`golden::snapshot`(`viz::golden`, `len` = 対象 prefix
  数)
- al-dataset `stage.rs`: `materialize_pcm` の入力単位呼出しを
  `dataset::write`(`input_id`、`len` = 生成 record 数を record)
  で包む
- al-observability `lib.rs`: stderr/file 両 fmt layer に
  `with_span_events(FmtSpan::CLOSE)` を追加(span に内側 event が
  ないと sink に何も残らないため、design/test-spec が要求する
  「内部 span event が観測できる」を成立させる close event を
  出す。T1 の layer 構築で未設定だった不備を修正)

### Decisions During Implementation

- span name は `&'static str` リテラル必須のため、family 呼出しは
  汎用ヘルパではなく個別 `info_span!` で包んだ
- locate の入力長は design の `len`(template/signal 長)を
  `len_a`(template)・`len_b`(signal)に分けて記録。1 field に
  畳むと片方を失うため
- `job_id` は stage span(runner)から親経由で伝播するため leaf
  側では重複設定していない

## Related Experiments

### Adopted

### Rejected

## Validation

- `cargo build --workspace`・`cargo test --workspace`・
  `cargo fmt --all -- --check`・`cargo clippy --workspace
  --all-targets -- -D warnings`: 全て green
- `AL_LOG=al_locate=debug` + `al run --stages ingest,locate` で
  stderr に `locate::{full_ncc,band_ncc,gcc_phat,spectral_envelope}`
  の close event(fields: `method`,`len_a`,`len_b`)を確認。
  file sink の JSON-lines にも同名 span が `spans`/`span`
  context 付きで記録される
- `AL_LOG=al_media=debug` + ingest で `media::ingest`(`input_id`・
  `len`・`sha256` を record 済み)→ `media::decode` →
  `media::probe` のネストを JSON-lines で確認

## Risks

- `FmtSpan::CLOSE` により logging 有効時の sink に span close
  event 行が増える(--log info でも `stage`・`cli::*` の close が
  出る)。イベント量増加は許容範囲と判断

## Follow-ups

- なし

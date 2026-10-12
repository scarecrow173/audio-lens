---
id: "IMPL-034Eg35VgnKI66R9G4k9he"
type: "impl"
status: "completed"
title: "stage 計時の manifest 永続化と runner 計装"
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
    - "docs/tasks/observability-stage-timing.md"
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
    - "docs/adr/stage-timing-manifest-persistence.md"
  derived-by: []
  verifies: []
  verified-by:
    - "docs/test-specs/observability-foundation.md"
  references:
    - "docs/plans/observability-foundation.md"
  defers: []
  deferred-by: []
metadata:
  experiments:
    adopted: []
    rejected: []
---

# stage 計時の manifest 永続化と runner 計装

## Summary

`StageRecord` に `duration_ms: Option<u64>` を追加し、
`StageRunner::execute` 1箇所に計時・stage span・完了 event を
注入した(plan T3)。計時は log 有効性と無関係に常時 manifest
へ永続化される。

## Context

本 IR は実装完了後に作成した(本来の運用はタスク開始時・最初の
コード変更前に作成する)。対応 task は
`docs/tasks/observability-stage-timing.md`、plan は
`docs/plans/observability-foundation.md`、上位設計は
`docs/designs/observability-foundation.md` と ADR
`docs/adr/stage-timing-manifest-persistence.md`、検証仕様は
`docs/test-specs/observability-foundation.md`。

## Implementation

### Changes

- `crates/al-job/Cargo.toml`: `tracing.workspace = true` を追加
- `crates/al-job/src/manifest.rs`: `StageRecord` に
  `#[serde(default)] pub duration_ms: Option<u64>` を追加
  (missing→None の既定で十分だが互換意図の明示として付与)。
  `Manifest::new` の初期化を `None` で追従。旧形式 manifest
  (field なし)が `Manifest::load` で読めることを確認する
  `manifest_without_duration_ms_loads_as_none` test を追加
- `crates/al-job/src/runner.rs`: `execute` 内の `stage.run(&ctx)`
  を `info_span!("stage", stage = %stage_name, job_id = %manifest
  .job_id)` で囲み(enter guard を match を通して保持し完了
  event を span にネスト)、`Instant::now()` で計時。
  成功分岐は `info!("stage completed")`、失敗分岐は
  `error!("stage failed")`(`error` field 付き)で、両方
  `stage`・`job_id`・`duration_ms` field を付けて emit。
  成否両分岐で `record.duration_ms = Some(elapsed)` を設定
  (`stage.run` の panic は unwind で適用範囲外)
- `StageRecord` 構造体リテラル 3 箇所に `duration_ms: None`
  を追従: manifest.rs `Manifest::new`、runner.rs の
  `unwrap_or_else` fallback、registry.rs test
- `crates/al-job/tests/resume.rs`:
  `duration_ms_is_recorded_and_resume_overwrites_only_rerun_
  stages` — Failed 後の resume で再実行 stage の `duration_ms`
  が新計測値に上書きされ、Completed 維持の stage の値が保持
  されることを sentinel 値で検証
- 意味付けは ADR どおり: 「その stage の最新実行の経過時間
  (ミリ秒)」。resume 再実行は新計測値で上書き(合算しない)、
  未実行 stage は既存値保持、旧 manifest 由来は `None`

### Decisions During Implementation

- stage 名は `serde_json::to_value(StageKind)` で serde の
  snake_case 文字列を取得し、得られない場合のみ `{:?}` に
  fallback(design の `<serde snake_case kind>` 規約)
- `Err` 戻りの失敗 stage も計時・記録対象とし、panic unwind
  のみ適用範囲外(design/ADR 規定どおり)
- manifest 書込みは既存の `manifest.save` タイミングに乗せ、
  新しい永続化経路は増やさなかった
- task が想定した「T1 未マージ時の `tracing` workspace dep
  肩代わり」は、実際には T1 と同一ブランチで進めたため不要
  だった

## Related Experiments

### Adopted

### Rejected

## Validation

- `cargo test -p al-job`: 旧 manifest 読込 test・resume の
  上書き/保持 test を含め green
- `al run` 完了後の `manifest.json` で実行 stage の
  `duration_ms` が `Some`(1ms 未満は 0)、未実行 stage が
  `None` であることを確認
- `al run --log info` で stderr/file 両 sink に
  `stage completed`(success)event が `duration_ms` field
  付きで観測できることを確認(T2 統合 test でも検証済み)
- `cargo build --workspace`・`cargo clippy`・
  `cargo fmt --check`: green

## Risks

- 1ms 未満の stage は `duration_ms = 0` になり得る(spec
  許容。正の値は要求しない)
- `stage.run` が panic した場合は unwind で計時・event の
  適用範囲外(設計上の既知の限界)

## Follow-ups

- なし(export package への stage duration 同梱は spec の
  Deferred Design Concerns)

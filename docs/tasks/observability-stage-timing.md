---
id: "TASK-034EevjbT8l6ojq2mAPkn0"
type: "task"
status: "todo"
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
    - "docs/plans/observability-foundation.md"
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
  references: []
  defers: []
  deferred-by: []
---

# stage 計時の manifest 永続化と runner 計装

## Work

`StageRecord` に `duration_ms` を追加し、`StageRunner::execute`
1箇所に計時・stage span・完了 event を注入する(plan T3)。
T1 と並行実施可能 — `tracing.workspace = true` が root
`Cargo.toml` の `[workspace.dependencies]` を前提とするため、
T1 未マージで本タスクを進める場合は `tracing` の workspace
dep 追加を本タスク側で肩代わりする(同一ファイル差分の衝突に
注意)。

- `crates/al-job/src/manifest.rs`: `StageRecord` に
  `#[serde(default)] pub duration_ms: Option<u64>` を追加
  (missing→None の既定で十分だが互換意図の明示として付ける)。
  `Manifest::new` の初期化を `None` で追従
- `crates/al-job/src/runner.rs`: `execute` 内の `stage.run(&ctx)`
  を `info_span!("stage", stage = "<serde snake_case kind>",
  job_id = ...)` で囲み(enter guard を match を通して保持)、
  `Instant::now()` で計時する。成功分岐は `tracing::info!`、
  失敗分岐は `tracing::error!` で stage 完了 event を emit
  (両方 `stage`・`job_id`・`duration_ms` field 付き)し、
  `record.duration_ms = Some(elapsed)` を成否両分岐で設定する
  (`stage.run` が panic した場合は unwind で適用範囲外)
- `StageRecord` 構造体リテラル 3 箇所(manifest.rs の
  `Manifest::new`、runner.rs の `unwrap_or_else` fallback、
  registry.rs test)に `duration_ms: None` を追加
- 意味付け: 「その stage の最新実行の経過時間(ミリ秒)」。
  resume で再実行された stage は新計測値で上書き(合算
  しない)、実行されなかった stage は既存値を保持、未実行・
  旧 manifest 由来は `None`。log 有効性と無関係に常時記録

## Classification

- `normal-plan-task`

## Upstream Documents

- `docs/plans/observability-foundation.md`
- `docs/designs/observability-foundation.md`
- `docs/adr/stage-timing-manifest-persistence.md`

## Dependencies

- なし(T1 と並行可 — `tracing` workspace dep の有無のみ前提
  確認。上記 Work の注記を参照)

## Affected Files

- `crates/al-job/Cargo.toml`: `tracing.workspace = true`
- `crates/al-job/src/manifest.rs`: `duration_ms` field 追加・
  `Manifest::new` 追従
- `crates/al-job/src/runner.rs`: stage span + 計時 + 成否
  event + `StageRecord` リテラル追従
- `crates/al-job/src/registry.rs`: test 内 `StageRecord`
  リテラル追従

## Done When

- [ ] 実行した stage の `duration_ms` が `Some` で manifest に
  記録される(log OFF でも。1ms 未満は 0 になり得る)
- [ ] `Err` 戻りの失敗 stage でも計時・記録され、event が
  `error!` で emit される
- [ ] resume で再実行 stage が新計測値に上書きされ、未実行
  stage の値が保持される
- [ ] `duration_ms` を持たない旧形式 manifest が
  `Manifest::load` で読める
- [ ] manifest 書込みが既存の `manifest.save` タイミングに
  乗り、新しい永続化経路を増やさない

## Verification

- [ ] `mise exec -- cargo test -p al-job` が green(旧 manifest
  読込 test・Failed→resume の上書き/保持 test を含む)
- [ ] `run` 実行後の `manifest.json` で完了 stage の
  `duration_ms` に値が存在する
- [ ] `mise exec -- cargo clippy`・`cargo fmt --check` が green

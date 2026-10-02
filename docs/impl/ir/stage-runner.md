---
id: "IMPL-034yZNZCVzaiiGljZdAb8Z"
type: "impl"
status: "completed"
title: "stage runner"
created: "2026-10-02"
updated: "2026-10-02"
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
    - "docs/tasks/stage-runner.md"
  implemented-by: []
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related: []
  refines: []
  refined-by: []
  derives-from: []
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

# stage runner

## Summary

Stage trait(読み取り専用 StageContext)と依存順実行・部分再開を行う StageRunner を実装した。

## Context

task `docs/tasks/stage-runner.md`、test-spec `docs/test-specs/stage-runner-resume.md` に従う。

## Implementation

### Changes

- `stage.rs`: Stage trait と StageContext(読み取り専用 manifest + registry + config)
- `runner.rs`: run/resume。ステージ遷移ごとに manifest を保存し、失敗時は Failed を記録して中断
- al-core に StageKind::PIPELINE_ORDER と dependencies() を追加

### Decisions During Implementation

- ステージは manifest を直接変更せず成果物レコードを返し、runner が登録する
- デフォルト依存は canonical pipeline order の先行ステージ全て
- `run` は `create_dir` で job_dir を原子的に予約し、既存ディレクトリ
  (manifest 有無・部分成果物を問わず)は明示的に拒否する
  (同一 Job ID の上書きは不可。再実行は新しい `JobDefinition::create`)
- `run` は最初のステージ実行前に初期 manifest を永続化する。
  プロセスが stage 途中で kill/panic しても manifest が残り、
  `resume` で Pending 状態から再開できる
- `resume` は manifest に記録された `inputs`/`config`/`stages` と
  渡された JobDefinition の完全一致を検証する。struct literal で
  同 Job ID の改変 def を渡しても実行前に拒否される
- run/resume 共通で全入力の `source_sha256` 必須性を検証し、
  unpinned input の struct-literal 迂回を拒否する
- manifest の保存は `al_job::fsutil::write_durable`(temp file +
  `sync_all` + rename + ディレクトリ fsync[unix])の atomic &
  durable write。stage 成果物のファイル書き込みも同ヘルパ経由に
  統一し、「manifest は Completed だが artifact が欠損」という
  電源断後の不整合を防ぐ
- run/resume は `<base>/<job>.lock`(job_dir の兄弟)に `try_lock`
  で実行中排他ロックを取る。ロックは job_dir 生成「前」に取得する
  ため、create_dir と lock の隙間に resume が割り込んで manifest 無し
  の dir を残す競合窓がない。同一 Job の並行 resume/run による
  manifest・成果物の競合更新を防ぐ。プロセス死亡時は OS がロックを
  解放するため lock ファイル残骸は無害(存在ではなく保持が判定)
- `validate_plan` は stage plan の重複・依存の順序(依存が後続に
  あると永遠に完了しない)を事前検証し、未登録実装も run/resume
  入口で拒否する。依存の「存在」だけでなく配置まで確認する

## Related Experiments

### Adopted

<!-- Record adopted experiment logs and why they were adopted. -->

### Rejected

<!-- Record rejected experiment logs and why they were rejected. -->

## Validation

<!-- Record the checks that were run. -->

## Risks

<!-- Record remaining risks. -->

## Follow-ups

<!-- Record follow-up tasks or next actions. -->

---
id: "PLAN-034yZp5w0ziN8jQO61t3nW"
type: "plan"
status: "completed"
title: "ジョブオーケストレーション基盤の実装"
created: "2026-10-02"
updated: "2026-10-02"
owners: ["scarecrow173"]
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
    - "docs/specs/analysis-job-model.md"
  implemented-by: []
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related: []
  refines: []
  refined-by: []
  derives-from:
    - "docs/specs/analysis-job-model.md"
    - "docs/designs/job-orchestration.md"
    - "docs/adr/immutable-resumable-job.md"
    - "docs/adr/implementation-stack.md"
  derived-by: []
  verifies: []
  verified-by:
    - "docs/test-specs/job-immutability-provenance.md"
    - "docs/test-specs/stage-runner-resume.md"
  references: []
  defers: []
  deferred-by: []
---

# ジョブオーケストレーション基盤の実装

## Goal

Rust ワークスペース上に、不変ジョブ・安定 id・manifest・
ステージ実行 / 部分再開を担うオーケストレーション基盤を実装し、
後続の ingest / locate / 解析ステージが乗る土台を作る。

## Scope

本プランは 1 つの実装ストリーム。対象はジョブモデル・成果物
レジストリ・manifest・ステージランナーの骨格であり、各ステージの
中身(デコード・定位・計測)は別プランで実装する。ステージは
trait / インターフェースとスタブ実装までを本プランの範囲とし、
具体的な解析処理は含めない。

## Upstream Documents

- `docs/specs/analysis-job-model.md`
- `docs/adr/immutable-resumable-job.md`
- `docs/adr/implementation-stack.md`

## Design Inputs

- `docs/designs/job-orchestration.md`
- `docs/designs/overview.md`

## File Map

- `Cargo.toml` - ワークスペース定義(members / 共通依存)
- `crates/al-core/src/lib.rs` - 共有ドメイン型(JobId, ArtifactId,
  InputRole, StageKind, Validity, Confidence 等)
- `crates/al-core/src/ids.rs` - 安定 id の生成・検証
- `crates/al-job/src/job.rs` - 不変ジョブ定義(入力ロール・設定・
  ステージ計画)
- `crates/al-job/src/manifest.rs` - manifest の読み書きと系譜記録
- `crates/al-job/src/registry.rs` - artifact-registry(成果物 id の
  発行・解決)
- `crates/al-job/src/runner.rs` - stage-runner(依存順実行・
  成果物再利用・部分再開)
- `crates/al-job/src/stage.rs` - ステージ trait と入出力契約
- `crates/al-cli/src/main.rs` - 最小 CLI(run / resume / inspect)
- `crates/*/tests/` - 各 crate の単体・結合テスト

## Implementation Sequence

- [ ] Rust workspace を初期化し、mise task(build/test/lint)を整備
- [ ] `al-core` にドメイン型と id 生成を実装
- [ ] `al-job` の不変ジョブ定義と設定のシリアライズを実装
- [ ] artifact-registry(成果物 id 発行・解決)を実装
- [ ] manifest の読み書きと系譜記録を実装
- [ ] stage trait とステージ間の id 参照契約を定義
- [ ] stage-runner に依存順実行と部分再開を実装
- [ ] スタブステージで run → 失敗 → resume の再現テストを作成
- [ ] `al-cli` の最小コマンド(run / resume / inspect)を実装

## Dependencies and Risks

- 依存: Rust toolchain と mise task の整備(リポジトリの
  `mise install` / `mise run setup` に従う)
- リスク: ステージ間インターフェースを早く固定しすぎると後続解析
  設計と不整合になる → ステージ契約は最小(成果物 id + dataset
  参照)に留め、解析固有の型は後続プランで追加する
- リスク: 部分再開の判定条件が複雑化する → 完了済み成果物の再利用は
  manifest の記録のみを根拠にし、暗黙の推測をしない

## Task Breakdown

- workspace 初期化と検証 task 整備
- `al-core` ドメイン型・id
- `al-job` ジョブ定義 / manifest / registry
- stage trait と stage-runner
- run / resume 再現テスト
- `al-cli` 最小コマンド

## Verification

- [ ] `mise run test`(または同等の cargo test task)が全 crate で通る
- [ ] 同一設定で run を再実行すると新ジョブ id が発行される
- [ ] 途中ステージを失敗させても resume が完了済み成果物を再利用する
- [ ] manifest だけで成果物の生成経緯を再構成できるテストが通る

## Open Questions / Gaps

- manifest / 設定の具体的フォーマット(JSON / TOML 等)は実装時に
  確定する
- dataset の物理保存実装は別プラン(`dataset-schema` 設計参照)であり、
  本プランでは trait とスタブで代替する

## Self-Review

- [ ] Every spec requirement maps to a task.
- [ ] No placeholders remain.
- [ ] File paths, relations, status values, and terminology are consistent.

## Implementation Handoff

- Recommended mode: inline execution(依存順が明確な単一ストリーム)
- Independent work streams: なし(workspace 初期化以降は順序依存)
- User approval required before dispatch: yes
- If approved: discover available implementation and delegation
  capabilities in the current environment, then execute task-by-task
  with review checkpoints.
- If declined or unavailable: execute inline using the same dependency
  graph and verification matrix.

## Review Handoff

- ステージ trait の範囲(解析固有型を含めない)を重点確認
- 部分再開の再現条件が `immutable-resumable-job` ADR と一致するか確認

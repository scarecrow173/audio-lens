---
id: "KNOW-qT4vLx9mZnKpRsFhJbDy"
type: "knowledge"
status: "approved"
title: "ジョブオーケストレーション: 不変ジョブ・manifest・ステージランナー"
created: "2026-11-19"
updated: "2026-11-19"
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
  implements: []
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
---

# ジョブオーケストレーション: 不変ジョブ・manifest・ステージランナー

`al-job` クレートは、不変ジョブ定義・manifest 永続化・ステージ依存順実行・部分再開の基盤を提供する。スコープはオーケストレーション骨格のみで、ステージ中身(デコード・定位・計測)は各 stage 実装に委ねる。本書は不変性の防御設計、manifest 契約、runner の resume セマンティクスをまとめる。

## 不変ジョブ設計

不変性は 3 層で防御する。(1) `JobDefinition::create` で入力パスを絶対化し内容ハッシュ `source_sha256` を作成時に必ず固定(読めない入力は Job 作成自体が失敗)。(2) `JobDefinition`/`JobInput` は serde round-trip のため public フィールドを維持するが、`StageRunner::run`/`resume` が実行前に全入力の `source_sha256` 必須性を検証し、struct literal 構築による `create` 迂回でも runner 入口で拒否する。(3) 再実行は常に新 `JobId` を発行し、`create_dir` で job_dir を原子的に予約して既存ディレクトリ(manifest 有無・部分成果物を問わず)を明示拒否する —— 同一 Job ID の上書きは許可しない。

manifest は「正本」に一元化し、`ArtifactRegistry` は状態を持たず命名規則のみ担う。宣言入力は `produced_by: None` の `declared_input` artifact として登録し、下流成果物の `inputs` が元入力を指す lineage 再構成を可能にする。

## Stage 契約と runner

`Stage` trait は読み取り専用 `StageContext`(manifest + registry + config)を受け、manifest を直接変更せず成果物レコードを返す —— runner が登録を担う。`StageKind::PIPELINE_ORDER` と `dependencies()` は canonical pipeline order の先行ステージ全てを既定依存とする。ステージ間受け渡しは成果物 id/dataset 参照に限定し、解析固有型は後付けとした(早期固定による不整合回避)。

`validate_plan` は run/resume 入口で計画を事前検証する: stage 重複・依存が後続に来る順序違反(永遠に完了しない)・未登録実装を拒否。`run` は最初のステージ実行前に初期 manifest を永続化し、途中 kill/panic でも manifest が残り `resume` で Pending から再開できる。ステージ遷移ごとに manifest を保存し、失敗時は Failed を記録して中断する。

## resume セマンティクスと排他制御

`resume` は manifest に記録された `inputs`/`config`/`stages` と渡された JobDefinition の完全一致を検証し、同 Job ID の改変定義を実行前に拒否する。manifest の `inputs`/`config`/`stages` から JobDefinition を再構成するため、ジョブ定義ファイルの別途保存は不要。

run/resume は `<base>/<job>.lock`(job_dir の兄弟)に `try_lock` で排他ロックする。重要なのはロックを job_dir 生成「前」に取得することで、create_dir と lock の隙間に resume が割り込む競合窓を排除した点である。プロセス死亡時は OS がロックを解放し、残骸の存在ではなく保持が判定基準となる。

manifest 書き込みは `write_durable`(temp file + `sync_all` + rename + ディレクトリ fsync)の atomic & durable write に一本化し、stage 成果物も同ヘルパを使う。

## CLI 設計

`al` バイナリは run/resume/inspect の 3 サブコマンドで、引数は手書きパース(clap 等の外部クレートなし、依存は `al-core`/`al-job`/`serde_json` のみ)。`run` は `--base-dir`/`--input role:path,...`/`--stages`/`--config <file>` を取る。`StageKind::Visualization` は後の Ruviform 移行で撤去され、pipeline は `Ingest → Locate → CheapAnalysis → ReferenceAnalysis → ExpensiveAnalysis → Dataset → Export` となった。後からの単一 stage 実行は `StageRunner::run_stage`/`run_stage_with_hooks` が担い、`al export` はこの経路の薄い入口。

## resume 検証の実証テスト

`crates/al-job/tests/resume.rs` はスタブ stage に実行カウンタを持たせ副作用の有無を直接観測する方式で、部分失敗→resume の再利用/未完了のみ再実行、全成功後の no-op、再実行での新 job id・別 manifest、依存未充足計画の拒否、lineage 再構成を検証した。

## 設計上の教訓

- 「resume 判定は manifest のみを正本とする」— 暗黙の推測をしないことで判定条件の複雑化を避ける。
- ロックはリソース生成より先に取得する(競合窓の構造的排除)。
- durable write は 1 ヘルパに集約し、manifest・成果物の両方で使い回す。
- ステージ契約を最小に留め、解析固有型を runner 層に持ち込まない。

関連 Issue/PR: 基盤フェーズのため番号なし(#95 系移行後の `run_stage` 追加を含む)。

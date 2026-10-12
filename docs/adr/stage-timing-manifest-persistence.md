---
id: "ADR-034EdkLSef7miMOT11Ca50"
type: "adr"
status: "accepted"
title: "stage 計時を manifest に永続化する"
created: "2026-10-11"
updated: "2026-10-11"
owners:
  - "scarecrow173"
relations:
  source:
    - "../discovery/observability-foundation-logging-and-profiling.md"
  changes:
    added: []
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements: []
  implemented-by:
    - "../plans/observability-foundation.md"
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "../specs/observability-foundation.md"
    - "observability-dedicated-crate.md"
    - "logging-sink-format-activation-contract.md"
  refines: []
  refined-by: []
  derives-from:
    - "../discovery/observability-foundation-logging-and-profiling.md"
  derived-by:
    - "../designs/observability-foundation.md"
    - "../tasks/observability-stage-timing.md"
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
metadata:
  adr:
    decision-makers:
      - "scarecrow173"
    consulted: []
    informed: []
---

# stage 計時を manifest に永続化する

## Context and Problem Statement

AudioLens にはステージ単位の時間配分を計測する経路がなく、全体
俯瞰からボトルネックを追う証拠チャネルが存在しない。計時の注入点
として `StageRunner::execute`(`crates/al-job/src/runner.rs`)が
全ステージ実行の唯一の通過点であり、永続先として manifest の
`StageRecord`(`crates/al-job/src/manifest.rs`)が既存する。
計時を構造化 record として manifest に永続化するか、log event
として流すか、両方かが分岐点となる(discovery D7)。cheaper は
相当する構造化 record を持たず log stream に流したが、AudioLens
は「計時=manifest、診断=log」の分離が可能。

## Decision Drivers

* 計時は job 横断の俯瞰分析の対象であり、job 成果物として構造化
  して残る形(= manifest)が機械処理に適する
* 計時記録は log の有効性と無関係に常時行う — logging が既定 OFF
  でも俯瞰データが欠損しない(discovery D5 と整合)
* 実行中のリアルタイム確認(stderr の text log)でも stage 所要
  時間を見たい — 両経路が必要
* 既存 manifest を読み込む経路(resume、過去 job の inspect)を
  壊さない — pre-release でも読込互換は `#[serde(default)]` で
  無難に維持する

## Considered Options

* manifest 永続化 + tracing event 両方(`StageRecord.duration_ms`
  に常時記録し、runner が stage 完了を duration field 付き event
  でも emit)
* manifest 永続化のみ(log event なし)
* log event のみ(stderr/file sink に流し、manifest には記録
  しない)

## Decision Outcome

Chosen option: "manifest 永続化 + tracing event 両方", because
構造化 record(manifest)は永続・機械処理用、log event は実行中の
人間向け確認用として両立し、それぞれ単独では片方の用途が欠ける
ため。

確定した意味付け・同梱方針:

* `duration_ms` は「その stage の最新実行の経過時間(ミリ秒)」。
  resume で再実行された stage は新しい計測値で上書きし、複数
  run での合算はしない。Completed/Skipped で実行されなかった
  stage は既存値を保持する
* 計測は stage の `Err` 戻り(失敗)でも記録する — 失敗した
  stage の所要時間もボトルネック調査の対象になる。ただし
  `stage.run` が panic した場合は unwind により計時注入の
  適用範囲外となり、計時・記録されない
* 未実行(Pending)および旧 manifest 由来の record は
  `duration_ms = None`。`Option<u64>` は serde で missing→
  None が既定のため `#[serde(default)]` は厳密には冗長だが、
  field を持たない旧 manifest の読込互換の意図を明示するため
  付ける
* export package への duration 同梱は行わない — 現状 package は
  job manifest を含まず analysis dataset のみをコピーする境界を
  維持する(`crates/al-viz/src/package.rs`)。package 側で実行
  時間が必要になった時点で job manifest コピーの追加を別途検討
  する(spec の Deferred Design Concerns に記録)

### Consequences

* Good, because stage 時間配分が job 成果物に構造化して残り、
  manifest を読むだけで job 横断の時間配分分析ができる
* Good, because log OFF でも `duration_ms` が manifest に残るため
  既定 OFF の activation 方針と矛盾しない
* Good, because `StageRunner::execute` 1箇所の変更で全 stage の
  計時を注入でき、stage 実装側の変更が不要
* Bad, because `StageRecord` の拡張と runner の event 定義で2箇所
  の変更が要る
* Bad, because resume 上書き意味では「初回実行の所要時間」は
  再実行時に失われる — 履歴が要る分析は manifest 履歴ではなく
  log file を使う想定

### Confirmation

* 実行後の `manifest.json` の各 `StageRecord` に `duration_ms`
  が記録されること(log 無効時を含む)
* `duration_ms` field を持たない旧 manifest が `Manifest::load`
  で読めること
* resume で再実行された stage の `duration_ms` が上書きされ、
  実行されなかった stage の値が保持されること
* `--log` 有効時に stderr/file sink へ stage 完了 event
  (`stage`・`job_id`・`duration_ms` field 付き)が成功・失敗の
  両分岐で出力されること

## Implementation Plan

* Affected paths: `crates/al-job/src/manifest.rs`(`StageRecord`
  に `#[serde(default)] pub duration_ms: Option<u64>` を追加し、
  `Manifest::new` の初期化を `None` で追従)、
  `crates/al-job/src/runner.rs`(`execute` 内の `stage.run(&ctx)`
  を stage 境界 span(`info_span!("stage", stage = "<serde
  snake_case kind>", job_id = ...)`)で囲んで
  `std::time::Instant` で計時し、成功・失敗の両分岐で
  `record.duration_ms` を設定。成功分岐は `tracing::info!`、
  失敗分岐は `tracing::error!` で stage 完了 event を emit
  (両方 `stage`・`job_id`・`duration_ms` field 付き)。
  `unwrap_or_else` の fallback `StageRecord` 構造体リテラルに
  `duration_ms: None` を追加)、
  `crates/al-job/src/registry.rs:77`(test 内の `StageRecord`
  構造体リテラルに `duration_ms: None` を追加)、
  `crates/al-job/Cargo.toml`(`tracing.workspace = true` 依存
  追加)
* Patterns to follow: 計測は runner の `execute` 1箇所に集約し、
  `Stage::run` 実装側に計時コードを持ち込まない。event の field
  名は spec の共通語彙(`job_id`・`stage`・`duration_ms`)に
  揃える。manifest 書込みは
  既存の `manifest.save` タイミング(各 stage 遷移後)に乗せ、
  新しい永続化経路を増やさない
* Patterns to avoid: stage 実装ごとの手動計時(注入点の分散)、
  `duration_ms` の合算・累積(意味が「最新実行」でない値になる)、
  log event だけに依存する計時記録(manifest が機械処理用の俯瞰
  データを失う)、旧 manifest 読込を壊す必須 field 化
* Dependencies or configuration: `tracing.workspace = true`
  (al-job)。`[workspace.dependencies]` への `tracing` 追加は
  [observability-dedicated-crate.md](observability-dedicated-crate.md)
  の plan で行う。

## Verification

* [ ] `run` 実行後の `manifest.json` で完了 stage の
  `duration_ms` に値が存在する(`Some`。1ms 未満の stage では
  0 になり得るため正の値は要求しない)こと、未実行 stage が
  `None` または key 省略で記録されることを確認する
* [ ] `duration_ms` を含まない JSON を `Manifest::load` に通す
  test で旧 manifest 読込が成功することを確認する
* [ ] Failed → resume の test で、再実行 stage の `duration_ms`
  が上書きされ、Completed 維持の stage の値が変わらないことを
  確認する
* [ ] `--log info` で stage 完了 event(`stage`・`job_id`・
  `duration_ms` field 付き)が stderr・file 両 sink に成功・
  失敗(失敗は `error!`)の両分岐で出力されることを確認する
* [ ] `mise exec -- cargo test -p al-job` が green である

## Pros and Cons of the Options

### manifest 永続化 + tracing event 両方

* Good, because 永続・機械処理(manifest)と実行中のリアルタイム
  確認(log)が両立し、「計時=manifest、診断=log」の責務分離に
  なる
* Bad, because record 拡張と event 定義の2箇所の変更になる

### manifest 永続化のみ

* Good, because 変更が最小
* Bad, because 実行中に stage 所要時間をリアルタイムで確認する
  手段がなく、長時間 stage の進行把握が job 完了待ちになる

### log event のみ

* Good, because manifest スキーマ変更が不要
* Bad, because 計時が文字列パース頼みになり job 横断の俯瞰分析が
  困難で、log OFF 時は計時そのものが欠損する

## More Information

* 確定経緯: `docs/discovery/observability-foundation-logging-and-profiling.md`
  の D7(L2 計時の出力先)と「確認済み(構造化永続先)」
* 計時注入点: `crates/al-job/src/runner.rs` の
  `StageRunner::execute`
* manifest スキーマ: `crates/al-job/src/manifest.rs` の
  `StageRecord`
* export package の同梱境界: `crates/al-viz/src/package.rs`
* 実施スコープ: `docs/specs/observability-foundation.md`

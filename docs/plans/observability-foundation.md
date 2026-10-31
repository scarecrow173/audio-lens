---
id: "PLAN-034Eesy8DzTxxROXy4lSFg"
type: "plan"
status: "approved"
title: "observability foundation 導入"
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
    - "docs/specs/observability-foundation.md"
    - "docs/adr/observability-dedicated-crate.md"
    - "docs/adr/logging-sink-format-activation-contract.md"
    - "docs/adr/stage-timing-manifest-persistence.md"
  implemented-by:
    - "docs/tasks/observability-crate.md"
    - "docs/tasks/observability-cli-wiring.md"
    - "docs/tasks/observability-stage-timing.md"
    - "docs/tasks/observability-leaf-instrumentation.md"
    - "docs/tasks/observability-criterion-samply.md"
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
  verified-by:
    - "docs/test-specs/observability-foundation.md"
  references: []
  defers: []
  deferred-by: []
---

# observability foundation 導入 Implementation Plan

## Goal

approved design `docs/designs/observability-foundation.md`
(DESIGN-034EdQdU4O8Ml4IzUkoFFY) に基づき、専用 crate
`al-observability` による logging 基盤(stderr+file dual sink、
per-sink format、既定 OFF の activation)、stage 計時の manifest
永続化、計装対象 leaf crate への span 配置、および L1/L3 計測手段
(criterion / samply)を導入する。

## Scope

単一の実装ストリームとして扱い、T1–T5 の 5 タスクに分割する。
`al-observability` の公開 API(resolve_activation / init)が T2・
T4 の前提となるため、タスク分割の境界は crate 境界と init/emit の
依存方向に沿う。

- In scope: `crates/al-observability` 新設、al-cli の
  `--log`/`--log-format` 配線と dispatch span、`StageRecord.
  duration_ms` の manifest 永続化と runner の stage span + 計時
  event、al-locate/al-analysis/al-media/al-dataset の span
  配置、al-math の criterion smoke bench、samply 手順書
- Out of scope(design/spec の Out of Scope を継承): raw payload
  capture、常時計装プロファイラ(puffin/tracy)、CI ベンチ・回帰
  ゲート、`--quiet` flag、stdout への log、export package への
  duration 同梱、既存 examples ベンチの criterion 移行、
  al-core/al-math/al-testkit への tracing 依存

## Upstream Documents

- `docs/specs/observability-foundation.md`(approved): 受入基準 —
  activation 契約、sink/format 契約、計装対象、duration_ms 意味、
  依存方向、metadata-only 規約
- `docs/adr/observability-dedicated-crate.md`(accepted): 専用 crate
  集約(T-B 型)と依存方向の決定
- `docs/adr/logging-sink-format-activation-contract.md`(accepted):
  sink/format/activation 契約の決定
- `docs/adr/stage-timing-manifest-persistence.md`(accepted):
  `duration_ms` の manifest 永続化 + tracing event の決定
- `docs/discovery/observability-foundation-logging-and-profiling.md`:
  D1–D11 の確定根拠

## Design Inputs

- `docs/designs/observability-foundation.md`(approved): 本計画の
  正本。`al-observability` の API 形態(`Activation`/`LogConfig`/
  `SinkKind`/`LogFormat`/`LogConfigError`/`InitError`/`LogGuard`/
  `resolve_activation`/`init`)、sink 構築(Vec boxed layers +
  EnvFilter + rolling file + degrade)、al-cli 配線
  (`init_logging` ヘルパ・dispatch span 生成順序・env scrub)、
  span 配置マップ、`StageRecord.duration_ms` 変更、依存配線を
  規定

## File Map

| Path | Responsibility | Task |
|------|----------------|------|
| `Cargo.toml`(root) | `workspace.members` に `crates/al-observability` 追加、`[workspace.dependencies]` に `al-observability`/`tracing`/`tracing-subscriber`(features: fmt, env-filter, json, ansi)/`tracing-appender` | T1 |
| `crates/al-observability/Cargo.toml` | 新設 crate マニフェスト(tracing/tracing-subscriber/tracing-appender を `.workspace = true`) | T1 |
| `crates/al-observability/src/lib.rs` | 型定義 + `resolve_activation`(純粋関数)+ `init`(boxed layers・EnvFilter・rolling file・degrade)+ `LogGuard` | T1 |
| `crates/al-cli/Cargo.toml` | `al-observability` + `tracing` 依存、`tracing-subscriber` dev-dep(capture test 用) | T2 |
| `crates/al-cli/src/main.rs` | `--log`/`--log-format` parse、`init_logging` ヘルパ、各 `cmd_*` の `--base-dir` 後 init + dispatch span、USAGE 追記 | T2 |
| `crates/al-cli/tests/cli.rs` | `al_command()` ヘルパによる `AL_LOG`/`AL_LOG_FORMAT` env scrub、logging 系統合 test、metadata-only capture test | T2 |
| `crates/al-job/Cargo.toml` | `tracing.workspace = true` | T3 |
| `crates/al-job/src/manifest.rs` | `StageRecord.duration_ms: Option<u64>` + `#[serde(default)]`、`Manifest::new` 初期化 | T3 |
| `crates/al-job/src/runner.rs` | `execute` の stage span + `Instant` 計時 + 成否両分岐 event、`StageRecord` リテラル追従 | T3 |
| `crates/al-job/src/registry.rs` | test 内 `StageRecord` リテラルに `duration_ms: None` 追加 | T3 |
| `crates/al-locate/src/methods.rs` 他 | `locate::full_ncc`/`locate::band_ncc`/`locate::gcc_phat`/`locate::spectral_envelope` span | T4 |
| `crates/al-analysis/src/*.rs` | `analysis::cheap`/`analysis::reference`/`analysis::region`/`analysis::<family>` span | T4 |
| `crates/al-media/src/stage.rs`/`decoder.rs` | `media::ingest`/`media::decode`/`media::probe` span | T4 |
| `crates/al-dataset/src/export*.rs` | `dataset::export`/`dataset::export::golden` span | T4 |
| `crates/al-dataset/src/stage.rs` | `dataset::write` span | T4 |
| 各 leaf crate `Cargo.toml` | `tracing.workspace = true`(al-job は T3 で実施) | T4 |
| `crates/al-math/Cargo.toml` | criterion dev-dep(≥7 日経過版 pin)+ `[[bench]] harness = false` | T5 |
| `crates/al-math/benches/dsp_smoke.rs` | 既存公開カーネルの最小 criterion bench | T5 |
| `docs/guides/profiling-samply.md` | samply による CPU sampling 手順書(新設) | T5 |

## Implementation Sequence

| Task | 内容 | Depends on | Blocks |
|------|------|------------|--------|
| T1 | `al-observability` crate 新設(型・resolve_activation・init・unit test) | — | T2, T4 |
| T2 | al-cli 配線(flag parse・init_logging・dispatch span・env scrub) | T1 | — |
| T3 | manifest `duration_ms` + runner 計時/span/event | —(T1 と並行可) | — |
| T4 | leaf crate instrumentation(span 配置 + tracing 依存) | T1 | — |
| T5 | criterion smoke bench + samply 手順書 | —(独立) | — |

T2・T4 は `al-observability`/workspace deps(T1 の成果物)に依存
する。T3 は T1 と並行可能 — `al-job` の `tracing.workspace = true`
が root `Cargo.toml` の `[workspace.dependencies]` 追加を前提と
するため、T3 が T1 より先にマージされる場合のみ T3 側で
`tracing` の workspace dep 追加を肩代わりする(同一ファイル差分
の衝突に注意)。T5 は他タスクと共有ファイルを持たず完全に独立。

## Dependencies and Risks

外部依存の追加: `tracing`・`tracing-subscriber`(features:
`fmt`, `env-filter`, `json`, `ansi`)・`tracing-appender`・
`criterion`(al-math dev-dep)。すべて crates.io 公開から
≥7 日経過した版を pin し、選定版と公開日を Implementation
Record に記録する。

| # | Risk | Likelihood | Impact | Mitigation |
|---|------|-----------|--------|------------|
| 1 | `non_blocking` file writer の未 flush — `WorkerGuard` を早期 drop・喪失すると file event が欠落する | med | med | `init` が `LogGuard` を返す型で drop 忘れを防ぎ、`cmd_*` スコープ全体で保持する(早期 `fail()` でも Drop で flush)。design の規約を T2 の Done When に組み込む |
| 2 | subscriber init 前に生成した span が inert のまま残り `span.record` が no-op になる | med | med | design の「span 生成順序」規約 — 各 `cmd_*` で `init_logging` を dispatch span 生成より必ず先に呼ぶ。統合 test で event 観測を確認 |
| 3 | `EnvFilter` parse 失敗時に生の directive が欠落し fail-fast の診断が不明瞭になる | med | low | `resolve_activation` が error メッセージに生の値を埋め込む(LogConfigError の各 variant が `String` を保持)。unit test で分岐を網羅 |
| 4 | 同一 `<base_dir>` で複数 `al` プロセスが同日同 log file に append する既知制約(retention 削除が Windows で失敗し得る) | — | low | design で既知制約として受容済み。行原子性は tracing-appender 依存。対策は scope 外 |
| 5 | `stderr=json` 指定時に `fail()` の plaintext 行が混ざり stderr が機械処理不能になる | — | low | design で受容済み — JSON 機械処理は file sink の責務と明示 |
| 6 | boxed layer(`Vec<Box<dyn Layer<Registry>>>`)の動的 dispatch コスト | low | low | per-sink format の表現力を優先する design 判断。event 単位の微小コストに留まる |
| 7 | T1・T3・T4 が root `Cargo.toml` / leaf `Cargo.toml` に並行して差分を出しマージ衝突する | med | low | workspace dep 追加は T1 が担う。T3 の並行実施時は `tracing` workspace dep の有無を前提確認として明記 |

### Rollback Strategy

- 対象: 全タスクがコード・文書の追加変更のみで、不可逆なデータ
  変換はない。manifest schema は `Option<u64>` の追加のみで
  旧形式の読込互換を維持する
- Trigger: 検証マトリクスの失敗(既存 test 非回帰・stdout 契約
  破壊・依存方向違反)で原因切り分けに時間を要する場合
- Procedure: 当該タスクのコミットを `git revert` する。タスク
  単位で独立に revert 可能(T2/T4 を revert しても T1 の crate
  自体は残せる)
- Verification: revert 後に `mise exec -- cargo test --workspace`
  が合格し、stdout 出力形式が変更前と一致すること

## Task Breakdown

### T1: `al-observability` crate 新設 — `docs/tasks/observability-crate.md`

root `Cargo.toml` への member/workspace dep 追加、crate
マニフェスト、型(`Activation`/`LogConfig`/`SinkKind`/
`LogFormat`/`LogConfigError`/`InitError`/`LogGuard`)、
`resolve_activation`(env/flag の生値を引数で受ける純粋関数)、
`init`(per-sink boxed layer + EnvFilter + rolling file +
degrade)。unit test で resolve_activation 全分岐・format
directive parse・未知 sink/format・空文字 flag の fail-fast を
網羅する。

### T2: al-cli 配線 — `docs/tasks/observability-cli-wiring.md`

`--log`/`--log-format` の flag parse(値必須チェック)、
`init_logging` ヘルパ、各 `cmd_*` の `--base-dir` 検証後 init
(run/resume=file+stderr、inspect=stderr のみ)、dispatch span
(init 後に生成)、USAGE 追記、`al_command()` ヘルパによる
`Command::env_remove` env scrub、logging 統合 test・
metadata-only capture test。

### T3: manifest + runner — `docs/tasks/observability-stage-timing.md`

`StageRecord.duration_ms: Option<u64>` + `#[serde(default)]`、
runner `execute` の `info_span!("stage", ...)` + `Instant` 計時 +
成否両分岐 emit、3 箇所の `StageRecord` リテラル(manifest.rs/
runner.rs/registry.rs)追従。`cargo test -p al-job` で旧 manifest
読込・resume 上書き/保持を検証。

### T4: leaf crate instrumentation — `docs/tasks/observability-leaf-instrumentation.md`

al-locate/al-analysis/al-media/al-dataset に design の span
map どおり span を配置し、各 `Cargo.toml` に
`tracing.workspace = true` を追加する。

### T5: criterion + docs — `docs/tasks/observability-criterion-samply.md`

`crates/al-math/benches/dsp_smoke.rs`(既存公開カーネルの最小
ベンチ)、`[[bench]] harness = false`、criterion の ≥7 日経過版
pin、`docs/guides/profiling-samply.md` 新設。

## Verification

| spec AC(要旨) | 検証方法 | 担当 |
|---|---|---|
| 既定 OFF で `cargo test --workspace` 全 pass・stdout 不変 | 全タスク完了後の workspace test + 既存統合 test | 全体 |
| run/resume で dual sink、inspect は stderr のみで `logs/` 非生成 | al-cli 統合 test(子プロセス + env scrub) | T2 |
| 空文字 `AL_LOG`=未指定、`--log-format` 単独では有効化されない | `resolve_activation` unit test + 統合 test | T1, T2 |
| 値なし `--log`・未知 sink/format・壊れた directive が `error:` + exit 2 で fail-fast | `resolve_activation` unit test + CLI 統合 test | T1, T2 |
| per-sink format 独立切替・flag が env を全体置換 | `resolve_activation` unit test + 統合 test | T1, T2 |
| `AL_LOG=al_locate=debug` で crate 内部 span が観測できる | `AL_LOG` 付き run の file JSON / stderr 確認 | T4 |
| logging 有効時も `al inspect` stdout が有効 JSON | 統合 test(serde_json で parse) | T2 |
| file sink init 失敗で warn + stderr-only degrade・job 成功 | init の degrade 経路 test(read-only dir) | T1 |
| metadata-only 規約の regression test 存在・pass | capture subscriber test(al-cli dev-dep) | T2 |
| `duration_ms` 記録(log OFF 含む)・旧 manifest 読込 | al-job test | T3 |
| resume で再実行 stage は上書き・未実行は保持 | al-job test | T3 |
| `cargo tree --workspace --edges normal` で tracing-subscriber/appender が al-observability 経由のみ | 出力検査(findstr 抽出) | T1–T4 完了時 |
| 統合 test が `Command::env_remove` で env scrub 済み | `cli.rs` の `al_command()` ヘルパ検査 | T2 |
| `cargo bench -p al-math` で criterion ベンチ実行・版/公開日を IR に記録 | bench 実行 + Implementation Record | T5 |
| samply 手順書が `docs/` に存在 | `docs/guides/profiling-samply.md` | T5 |
| OFF 時 overhead を計測・記録 | examples ベンチ前後比較 or criterion 空転 cost を IR に記録 | T2 完了後 |
| `cargo test --workspace`・`cargo clippy`・`cargo fmt --check` green | 各タスクの Verification + 最終確認 | 全体 |

## Open Questions / Gaps

- 各依存(tracing/tracing-subscriber/tracing-appender/
  criterion)の pin 版は add 時点で決定し、選定版と crates.io
  公開日を Implementation Record に記録する(design は版を
  固定せず ≥7 日経過ルールのみ規定)
- T3 を T1 より先にマージする場合、`tracing` workspace dep の
  追加を T3 が肩代わりするか T1 完了待ちとするかは実施時に判断
- logging OFF 時 overhead の計測は examples ベンチ前後比較か
  criterion 空転 cost のどちらを採るか — T5 の dsp_smoke 整備後に
  判断可能

## Self-Review

- [x] Every spec requirement maps to a task.
- [x] No placeholders remain.
- [x] File paths, relations, status values, and terminology are consistent.

## Implementation Handoff

- Recommended mode: inline execution(T1→T2・T1→T4 の依存順、
  T3・T5 は並行可能)
- Independent work streams: T3・T5(T1 完了後は T2・T4 も並行可)
- User approval required before dispatch: yes
- If approved: discover available implementation and delegation capabilities in the current environment, then execute task-by-task with review checkpoints.
- If declined or unavailable: execute inline using the same dependency graph and verification matrix.

## Review Handoff

- design の span map・API 形態を正本として転記済み。レビュアは
  Verification matrix が spec の Acceptance Criteria を網羅して
  いるか、Task Breakdown の依存(T2←T1、T4←T1、T3 並行・T5 独立)
  が妥当かを確認すること

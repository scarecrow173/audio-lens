---
id: "ADR-034EdkF9d9cVHx4NSumGSL"
type: "adr"
status: "accepted"
title: "observability を専用 crate に集約する"
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
  implemented-by: []
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "../specs/observability-foundation.md"
    - "logging-sink-format-activation-contract.md"
    - "stage-timing-manifest-persistence.md"
  refines: []
  refined-by: []
  derives-from:
    - "../discovery/observability-foundation-logging-and-profiling.md"
  derived-by: []
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

# observability を専用 crate に集約する

## Context and Problem Statement

AudioLens の全 crate に `tracing`/`log` 系依存は 0 件で、
production 経路の stderr 出力は `al-cli/src/main.rs` の `fail()`
が出す `error: {message}` 1系統のみ。計装対象 crate(spec の
Scope で列挙)への logging 基盤導入が確定した以上、sink init・
format 配線・routing・activation
解釈の所有境界をどこに置くかを決める必要がある。al-cli に寄せる
最小差分か、専用 crate に集約するか、facade で wrap するかが分岐点
となる(discovery D1/D10)。

## Decision Drivers

* sink init/format/routing/activation の方針を単一の所有境界に置き、
  肥大・変更時の編集箇所を一箇所に閉じ込める(discovery D1)
* leaf crate は subscriber の存在を知らなくてよい — emit は
  `tracing` 標準 API を直接使い、facade wrap で標準 API を隠す
  実益はない(discovery D10、cheaper で棄却済みの T-C と同じ理由)
* 依存方向を一方向に保ち、循環依存と責務分散を防ぐ。実際の
  依存系統は `al-cli` → `al-observability` →
  `tracing-subscriber`/`tracing-appender`(init 系統)と
  leaf crate → `tracing`(emit 系統)の2系統で、leaf crate は
  `al-observability` に依存しない
* 同一テーマを確定済みの cheaper observability foundation で検証
  済みの構成(T-B 型)を踏襲する
* init は `al-cli` のみが呼ぶ — 各 `cmd_*`
  (`run`/`resume`/`inspect`)で `--base-dir` 検証後に1回行う
  (プロセス境界でライブラリ crate が global subscriber を
  set しない)。`inspect` は読み取り専用のため stderr sink のみで
  file sink は組まない

## Considered Options

* 専用 `al-observability` crate が sink init/format/routing/
  activation 解釈を所有し、leaf crate は `tracing` 直接依存で
  emit のみ行う(cheaper T-B 型)
* `al-cli` が init を所有する縮小構成: crate を増やさず、
  sink/format/activation の解釈コードを `main.rs` に置く
  (cheaper の T-A 相当)
* `al-observability` が span macro・event macro まで wrap する
  facade 構成: leaf crate は `tracing` にすら依存せず
  `al_observability` 経由で emit する(cheaper の T-C 相当)

## Decision Outcome

Chosen option: "専用 `al-observability` crate が sink
init/format/routing/activation 解釈を所有し、leaf crate は
`tracing` 直接依存で emit のみ行う", because sink/format/
activation 方針の単一所有境界ができることで backend 切替・sink
追加を1境界に吸収でき、計装対象 crate への横展開で一貫する
ため。leaf
crate が `tracing` 標準 API を直接使うことで facade が隠すだけの
無益な間接層を避けられる。cheaper で同一構成が検証済み。

### Consequences

* Good, because sink backend の切替・追加(file sink の rolling
  変更、将来の sink 追加)が `al-observability` 内部の変更に閉じる
* Good, because activation 解釈(env/flag の優先順位、既定値、
  EnvFilter directive の解釈)が単一の関数に集約され、leaf crate
  や CLI 側に解釈ロジックが分散しない
* Good, because leaf crate は `tracing::instrument`/`event!` 等の
  標準 API をそのまま使え、subscriber 未設定時は no-op になる
  tracing の既定動作によりテスト・examples でも init 不要
* Bad, because workspace に crate が1つ増え、計装対象 leaf
  crate の Cargo.toml に `tracing` 依存を追加する初期コストが
  かかる
* Bad, because `al-observability` 自体が `tracing-subscriber`/
  `tracing-appender` に依存するため、その version bump は
  専用 crate 経由で管理する必要がある

### Confirmation

* `mise exec -- cargo tree --workspace` で
  `tracing-subscriber`/`tracing-appender` が `al-observability`
  経由のみで依存グラフに現れ、leaf crate が `al-observability`
  に依存しないこと
* subscriber init(`init` 系関数の呼出し)が
  `crates/al-cli/src/main.rs` の各 `cmd_*` 内(`--base-dir`
  検証後)のみから行われること
* `--log` 有効時に `run`/`resume` で stderr と file の両 sink
  へ event が出力され、`inspect` では stderr sink のみになる
  こと(契約の詳細は
  [logging-sink-format-activation-contract.md](logging-sink-format-activation-contract.md))
* `AL_LOG=al_locate=debug` で leaf crate の stage 内部 span が
  観測できること(計装対象の列挙は spec の Scope を参照)

## Implementation Plan

* Affected paths: `crates/al-observability/`(新設:
  `Cargo.toml`、`src/lib.rs` に activation 解釈済み config を受けて
  subscriber を構築し guard を返す薄い init API)、root `Cargo.toml`
  (`workspace.members` に `al-observability` を追加し、
  `[workspace.dependencies]` に `tracing`/`tracing-subscriber`/
  `tracing-appender` を追加)、`crates/al-cli/Cargo.toml`
  (`al-observability` + `tracing.workspace = true` 依存)、
  `crates/al-cli/src/main.rs`(各 `cmd_*` で `--base-dir` 検証後に
  flag/env の生値を `al-observability` の解釈関数へ渡して init、
  返却 guard を `cmd_*` のスコープで保持)、計装対象 leaf crate
  の `Cargo.toml` に `tracing.workspace = true` 依存を追加
  (al-job・al-locate・al-analysis・al-media・al-viz・al-dataset。
  al-core・al-math・al-testkit は計装対象外 — spec の Scope を
  参照)
* Patterns to follow: leaf crate は `tracing` の標準 API
  (`#[tracing::instrument]`、`tracing::info!`/`debug!` 等)を
  直接使用する。init はプロセス入口(各 `cmd_*`、
  `--base-dir` 検証後)で1回。activation 解釈の `resolve` 系
  関数は env を内部で読まず env 値も引数として受ける純粋関数に
  し、unit test 可能にする。`al-observability` の公開 API は
  「解釈済みの設定を受けて subscriber + guard を返す」薄い形に
  留める
* Patterns to avoid: `al-observability` による tracing macro の
  wrap/facade(棄却した T-C)、leaf crate からの
  `tracing-subscriber`/`tracing-appender`/`al-observability`
  直接依存、ライブラリ crate 内での `set_global_default`/`init`
  呼出し、env 変数を複数箇所で読む activation 解釈の分散
* Dependencies or configuration: `[workspace.dependencies]` に
  `tracing`(leaf crate 群 + al-cli + al-observability)、
  `tracing-subscriber`(feature: `fmt`, `env-filter`, `json` —
  `json` は file sink の JSON-lines に必須)、
  `tracing-appender`(al-observability のみ)を追加。leaf crate
  は `tracing.workspace = true` で参照する

## Verification

* [ ] `mise exec -- cargo tree --workspace` の出力を検査し、
  `tracing-subscriber`/`tracing-appender` が `al-observability`
  配下のみに現れることを確認する(出力を目視確認するか
  `findstr` 等で抽出)
* [ ] leaf crate の `Cargo.toml` に `tracing-subscriber`/
  `tracing-appender`/`al-observability` がないことを各
  Cargo.toml の検査で確認する
* [ ] `set_global_default`・subscriber init 呼出しが
  `crates/al-cli/src/main.rs` と `al-observability` 内部に限定
  されることをコード検索で確認する
* [ ] `mise exec -- cargo build --workspace` と
  `mise exec -- cargo test --workspace` が green である

## Pros and Cons of the Options

### 専用 `al-observability` crate(T-B 型)

* Good, because backend 切替・sink 追加・activation 規約の変更を
  1境界に吸収でき、横展開で一貫する。cheaper で検証済みの形
* Bad, because crate +1 と計装対象 leaf crate への `tracing`
  依存追加、init 配線の初期コストがある

### `al-cli` init の縮小構成(T-A 相当)

* Good, because crate が増えず最小差分で済む
* Bad, because sink/format/activation 方針の単一所有境界がなく、
  機能が肥大したとき al-cli に解釈ロジックが集中する。init 方針が
  cli に分散する兆候がある

### facade wrap(T-C 相当)

* Good, because emit 記法を一元化できる
* Bad, because `tracing` の標準 API(`#[instrument]`/`event!`)を
  自前 macro で隠すだけで実益がなく、標準エコシステムの知識が
  通用しなくなる。cheaper で棄却済み

## More Information

* 確定経緯: `docs/discovery/observability-foundation-logging-and-profiling.md`
  の D1(全体構成規模)・D10(crate topology の詳細)
* 契約面の詳細: `docs/adr/logging-sink-format-activation-contract.md`
* 実施スコープ: `docs/specs/observability-foundation.md`
* 参照実装(外部リポジトリ): `F:\repositry\cheaper` の
  `docs/adr/consolidate-observability-behind-a-dedicated-crate.md`

---
id: "IMPL-034RBZk8kiu9P2zd0cGhw5"
type: "impl"
status: "completed"
title: "ruviform-observability logging 委譲 (AL#138)"
created: "2026-11-04"
updated: "2026-11-04"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/audio-lens/issues/138"
  changes:
    added: []
    modified:
      - type: "manifest"
        path: "Cargo.toml"
      - type: "manifest"
        path: "crates/al-observability/Cargo.toml"
      - type: "code"
        path: "crates/al-observability/src/lib.rs"
      - type: "code"
        path: "crates/al-cli/src/main.rs"
      - type: "test"
        path: "crates/al-cli/tests/observability.rs"
      - type: "doc"
        path: "docs/tasks/ruviform-observability-logging-migration.md"
    deleted: []
    renamed: []
    moved: []
    generated:
      - type: "lockfile"
        path: "Cargo.lock"
  implements:
    - "docs/tasks/ruviform-observability-logging-migration.md"
  implemented-by: []
  depends-on:
    - "docs/designs/ruviform-observability-logging-migration.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "https://github.com/scarecrow173/ruviform/issues/334"
    - "https://github.com/scarecrow173/ruviform/issues/335"
  refines: []
  refined-by: []
  derives-from:
    - "docs/plans/ruviform-observability-logging-migration.md"
  derived-by: []
  verifies: []
  verified-by:
    - "docs/test-specs/ruviform-observability-logging-migration.md"
  references:
    - "docs/specs/ruviform-observability-logging-migration.md"
    - "docs/adr/logging-sink-format-activation-contract.md"
    - "docs/impl/ir/observability-cli-wiring.md"
    - "https://github.com/scarecrow173/ruviform/issues/276"
  defers: []
  deferred-by: []
metadata:
  experiments:
    adopted: []
    rejected: []
---

# ruviform-observability logging 委譲 (AL#138)

## Summary

`al-observability` の共通 logging 実装を
`ruviform-observability::logging`(feature `logging`、resolved
SHA `8fb665f…`)へ委譲した。stderr sink は ruviform
`build_logging_layers` の bounded-writer layer、file sink は
`al.YYYY-MM-DD.log` 命名契約のため暫定 local 構築を維持する
(ruviform#334 解消条件付き)。`resolve_activation` をはじめと
する製品固有の有効化ポリシー・sink/format 閉集合語彙・エラー
契約は本 crate に残る薄い Config Adapter として保持した。

## Context

本 IR は実装完了後に作成した(本来の運用はタスク開始時・最初の
コード変更前に作成する)。対応 task は
`docs/tasks/ruviform-observability-logging-migration.md`、plan は
`docs/plans/ruviform-observability-logging-migration.md`、上位設計は
`docs/designs/ruviform-observability-logging-migration.md`、検証仕様は
`docs/test-specs/ruviform-observability-logging-migration.md`。前提は
OBS-01(ruviform #276、merged)の汎用 logging adapter と、先行の
`docs/impl/ir/observability-cli-wiring.md` が確立した CLI 配線。

## Implementation

### Changes

- `Cargo.toml`(`[workspace.dependencies]`):
  `ruviform-observability = { git = …, features = ["logging"] }`
  を追加(manifest に rev 固定なし、Cargo.lock が正本)
- `crates/al-observability/Cargo.toml`:
  `ruviform-observability.workspace = true` を追加
- `crates/al-observability/src/lib.rs`:
  - `init` が `build_logging_layers(&LoggingConfig{ sinks:
    [SinkConfig{Stderr, format, filter, Required}] })` を呼び、
    返却された `LoggingLayers`/`WriterGuard` を host が保持する。
    global 登録は `registry().with(layers).try_init()` で1回のみ
    (ruviform adapter は `set_global_default` を呼ばない契約)
  - file sink は `build_file_sink` として暫定 local 構築を維持:
    `filename_prefix("al")` + `filename_suffix("log")` +
    `Rotation::DAILY` + `max_log_files(7)` + `non_blocking` +
    `FmtSpan::CLOSE` + per-sink `EnvFilter`。ruviform
    `SinkDestination::RollingFile` が suffix を公開しないため
    (ruviform#334)、解消後に `RollingFile` へ移す
  - `InitError` に `Build(LoggingBuildError)` variant を追加
    (`Required` stderr sink 構築失敗の上位伝播。
    `LogConfigError`→`error:` + exit 2 の fail-fast 契約は不変)
  - `Activation::Enabled` を `Box<LogConfig>` 化し、既存の
    `#[allow(clippy::large_enum_variant)]` を撤去(作業規約の
    lint 抑制禁止に従う構造的解消)
  - `map_log_format` で製品語彙 `LogFormat::{Text,Json}` を
    ruviform `LogFormat::{Text,JsonLines}` へ1箇所で写像
  - `LogGuard` は `Vec<ruviform WriterGuard>` +
    `Option<WorkerGuard>` を保持し Drop で drain/flush を保証
- `crates/al-cli/src/main.rs`: `init(*config)` に追随
  (`Enabled(Box<LogConfig>)` 化のみ。env 読み取り・`file_dir`
  設定・fail 経路は不変)
- `crates/al-cli/tests/observability.rs`: golden test を
  6→12 test に拡充(先行コミット済み)

### Decisions During Implementation

- **file sink を local 暫定維持**: ruviform `RollingFile` は
  `filename_suffix` を持たず `al.YYYY-MM-DD.log` の `.log`
  suffix を生成できない(ruviform#334 起票済み)。製品側の恒久
  workaround を禁じる規約に従い、暫定 local 構築として隔離し、
  module doc・design・task に理由・解消条件・追跡 Issue を明記
- **stderr span close は既知 regression として扱う**: ruviform
  `make_layer` は `FmtSpan` を設定しない(ruviform#335 起票
  済み)。`Box<dyn Layer>` は不透明で下流から span event を再
  設定できないため、stderr の `FmtSpan::CLOSE` 出力は委譲と引換
  に一時的に失われる。file JSONL 側は local layer で `CLOSE` を
  維持し、集計用途の leaf span 観測契約は継続
- **per-sink filter 方式へ移行**: 従来は global `EnvFilter` 1本
  + per-sink fmt layer だったが、ruviform の sink モデル
  (per-sink filter 文字列)に合わせ、同一 directive を各 sink
  に適用する形へ変更。観測される振る舞いは等価
- **`init` の global install は host 側に残す**: ruviform
  adapter が dispatch/layer 構築のみを担い、global 登録は host
  契約のため、従来どおり `try_init()` + `AlreadyInitialized`
  を本 crate が担う(2回目呼出の検出経路も不変)
- **Source→Snapshot(実装順序 c)は対象外**: OBS-07(ruviform
  #289)が未マージで、かつ登録対象となる Counter 実体が本
  Repo に存在しない。段階導入の前提条件が揃っていないため、
  本 migration では接続対象なしと判断した

## Related Experiments

### Adopted

### Rejected

## Validation

実行環境: Windows 11 x64(Git Bash)、Rust `1.99.0`(mise pin)。

- `mise exec -- cargo check -p al-observability --all-targets`:
  exit 0
- `mise exec -- cargo clippy -p al-observability -p al-cli
  --all-targets -- -D warnings`: exit 0(`#[allow]` 撤去後も
  warning なし)
- `mise exec -- cargo test -p al-observability --lib`: 15 test
  全 pass(exit 0)
- `mise exec -- cargo test -p al-cli --test observability`:
  golden 12 test 全 pass(exit 0)。migration 前の旧実装でも
  同 12 test が pass 済み(互換 parity)
- `mise exec -- cargo tree`: `al-observability` が
  `ruviform-observability` + tracing stack を直接保持、
  `al-observability` の逆依存は `al-cli` のみ、leaf crate
  (al-core/al-job/al-locate/al-analysis/al-media/al-dataset/
  al-service)はいずれも `al-observability`・
  `tracing-subscriber`・`tracing-appender` 非依存
- `Cargo.lock`: `ruviform-observability v0.1.0` が同一 SHA
  `8fb665f…` で解決(他 ruviform crate と同一 revision)
- logging ON/OFF の解析 artifact 比較(補助検証・canonical
  ではない): 同一 WAV 入力で `al run --stages ingest` を
  ON/OFF 各1回実行。`decoded.f64le` は byte 一致、
  `metadata.json`/`manifest.json` は生成 ID・timestamp 以外
  一致、file 集合差分は `logs/` と lock file のみ
- `mise run fmt` / `check` / `lint` / `test`: いずれも
  exit 0(workspace 全体。golden 12 test・unit 15 test を
  含む全 test green)
- `mise run verify-ci`(fmt-check → check-ci → lint-ci →
  test-ci): exit 0
- `mise run check-ruviform-lock`: `8fb665f…`・16 packages
  (ruviform-observability を含む)で exit 0
- `mise run check-ruviform-rev`: lockfile-primary REV match
  で exit 0
- `mise run check-cli-headless` / `verify-toolchain`: exit 0
- 未実施: 実 Plugin・実 GUI・非 Windows OS での検証(headless
  lane のみ。samply 手順は文書・profile 定義とも変更なしで
  保持)

## Risks

- stderr sink の `FmtSpan::CLOSE` 欠落(ruviform#335)。file
  JSONL sink は維持されているため集計用途への影響はないが、
  stderr へ span close を期待する利用は一時的に欠落する
- file sink が暫定 local 構築のため、ruviform 側の bounded
  queue・health counter の恩恵は file sink に及ばない
  (ruviform#334 解消まで)

## Follow-ups

- ruviform#334(`filename_suffix`)解消後、file sink を
  `SinkDestination::RollingFile` へ移し `build_file_sink` を
  撤去
- ruviform#335(`FmtSpan` 設定面)解消後、stderr sink の
  span close を復旧
- OBS-07(ruviform#289)マージ・Counter 実体の導入後に
  Source→Snapshot 接続を段階導入

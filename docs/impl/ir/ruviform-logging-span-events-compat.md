---
id: "IMPL-Ek8G2pJnU9LJS6U1EFxFDw"
type: "impl"
status: "completed"
title: "al-observability SinkConfig::span_events 追従 (AL#127)"
created: "2026-11-07"
updated: "2026-11-07"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/audio-lens/issues/127"
  changes:
    added:
      - type: "doc"
        path: "docs/specs/ruviform-logging-span-events-compat.md"
      - type: "doc"
        path: "docs/designs/ruviform-logging-span-events-compat.md"
      - type: "doc"
        path: "docs/plans/ruviform-logging-span-events-compat.md"
      - type: "doc"
        path: "docs/test-specs/ruviform-logging-span-events-compat.md"
      - type: "doc"
        path: "docs/tasks/ruviform-logging-span-events-compat.md"
      - type: "doc"
        path: "docs/impl/ir/ruviform-logging-span-events-compat.md"
    modified:
      - type: "code"
        path: "crates/al-observability/src/lib.rs"
      - type: "test"
        path: "crates/al-cli/tests/observability.rs"
      - type: "doc"
        path: "docs/designs/README.md"
      - type: "doc"
        path: "docs/plans/README.md"
      - type: "doc"
        path: "docs/specs/README.md"
      - type: "doc"
        path: "docs/tasks/README.md"
      - type: "doc"
        path: "docs/test-specs/README.md"
      - type: "doc"
        path: "docs/designs/ruviform-observability-logging-migration.md"
      - type: "doc"
        path: "docs/tasks/ruviform-observability-logging-migration.md"
      - type: "doc"
        path: "docs/test-specs/ruviform-observability-logging-migration.md"
      - type: "doc"
        path: "docs/impl/ir/README.md"
    deleted: []
    renamed: []
    moved: []
    generated:
      - type: "lockfile"
        path: "Cargo.lock"
      - type: "vendor"
        path: "ui/vendor/"
  implements:
    - "docs/tasks/ruviform-logging-span-events-compat.md"
  implemented-by: []
  depends-on:
    - "docs/designs/ruviform-logging-span-events-compat.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/designs/ruviform-observability-logging-migration.md"
    - "https://github.com/scarecrow173/ruviform/issues/335"
    - "https://github.com/scarecrow173/ruviform/issues/334"
  refines: []
  refined-by: []
  derives-from:
    - "docs/plans/ruviform-logging-span-events-compat.md"
  derived-by: []
  verifies: []
  verified-by:
    - "docs/test-specs/ruviform-logging-span-events-compat.md"
  references:
    - "https://github.com/scarecrow173/ruviform/pull/348"
    - "https://github.com/scarecrow173/ruviform/actions/runs/42302538464"
  defers: []
  deferred-by: []
metadata:
  experiments:
    adopted: []
    rejected: []
---

# al-observability SinkConfig::span_events 追従 (AL#127)

## Summary

consumer-verify run 42302538464 で検出された E0063(`SinkConfig`
必須 field `span_events` 欠落)を、AL#127 の追従対象として
修復した。stderr `SinkConfig` literal 構築に
`span_events: SpanEvents::CLOSE` を設定し、migration 前に
両 layer が持っていた `FmtSpan::CLOSE` の stderr 側出力を
復旧した(migration 設計が予約した #335 回復条件の実行)。
`Cargo.lock` の ruviform resolved SHA を追従契約に従い
`update-ruviform-rev.mjs` で `78b4e7e` → `ad99d18`
(ruviform main HEAD)へ明示更新し、vendor を
`sync-ui-deps` で整合した。

## Context

AL#127 の Ruviform 追従作業の一環。consumer-verify run
42302538464(RF candidate `30205f2b` × AL `04f1861` × SV
`d8bf8ca`)で `crates/al-observability` の stderr `SinkConfig`
literal 構築が必須 field `span_events` を欠き E0063 で
失敗した。同 field は ruviform `33d93dd`(Issue #335 /
PR #348)で導入された。migration 前に両 layer が持っていた
`FmtSpan::CLOSE` の stderr 側出力を本変更で復旧した
(migration 設計が予約した #335 回復条件の実行)。上流 doc
chain は specs / designs / plans / tasks / test-specs の
`ruviform-logging-span-events-compat.md` 各件(relations の
implements / derives-from / verified-by と一致)。

## Provenance

| 項目 | 値 |
| --- | --- |
| AL base | `04f18611c51aa4ba8f16d43987e5bd290a78f12f` (origin/main HEAD) |
| ruviform lock(before) | `78b4e7ef671d9b4690df031dd62ba2148006ff52` |
| ruviform lock(after) | `ad99d181c34cded1e56489fa9542027fe0dfc966` |
| 失敗 tuple(証拠) | RF candidate `30205f2b` + AL `04f1861` + SV `d8bf8ca` |
| field 導入 commit | ruviform `33d93dd`(#335 / PR #348) |
| OS / toolchain | Windows 11 x64, Git Bash, mise 2026.10.1, rust 1.99.0, node 26.12.1 |

## Implementation

- `crates/al-observability/src/lib.rs`: `SpanEvents` import、
  stderr `SinkConfig` に `span_events: SpanEvents::CLOSE`、
  module doc の「既知の一時 regression(ruviform#335)」節を
  解消済み記述へ更新。
- `crates/al-cli/tests/observability.rs`:
  `stderr_sink_emits_span_close_events` を追加 — `run --log
  info` の stderr に `time.busy` を含む close record が届く
  外部観測 regression。併せて `--log warn` で close record が
  出ない負例を入れ、close record が per-sink `EnvFilter` を
  通る contract を test-spec の規定どおり検証する。
- `Cargo.lock`: ruviform 17 crate の resolved SHA を
  `78b4e7e` → `ad99d18`(専用ツール経由、manifest 不変)。
- `ui/vendor/`: `sync-ui-deps` で `ad99d181c3` に再生成
  (gitignore 済み生成物)。
- doc チェーン 5 件 + `docs/*/README.md` 生成 index 再生成
  (doc-maintenance apply、5 action・findings 88→77)。
- review 指摘反映: migration 由来 doc 3 件
  (designs/tasks/test-specs)の #335 暫定 regression 記述に
  解消済み注記を追記し、本変更と整合させた。
  `docs/impl/ir/README.md` に本 IR の行を追加(同 index は
  手動管理)。

## Related Experiments

### Adopted

なし。

### Rejected

なし。

(front matter の `metadata.experiments.adopted` / `rejected`
はいずれも空であり、本件では実験ログの採用・却下は
記録されていない)

## Validation

実施済み(全て Windows 11 x64 上の正規 mise タスク):

| command | exit | 結果 |
| --- | --- | --- |
| `node tools/update-ruviform-rev.mjs ad99d18…` | 0 | 17 pkg を同一 SHA へ更新 |
| `mise run check-ci`(修復前 @ ad99d18) | 101 | **RED 再現**: E0063 `missing field span_events`(lib.rs:264) |
| `mise run verify-ci`(修復後) | 0 | fmt→check-ci→lint-ci→test-ci 全 pass |
| `mise exec -- cargo test -p al-cli --test observability` | 0 | 13/13 pass(新規 stderr close test + `--log warn` 負例を含む) |
| `mise run sync-ui-deps` | 0 | vendor @ ad99d181c3 再生成 |
| `mise run check-ruviform-lock` | 0 | 単一 SHA `ad99d18` / 17 pkg |
| `mise run check-ruviform-rev` | 0 | lockfile-primary・REV match |
| `mise run check-cli-headless` | 0 | GUI 依存混入なし |
| `mise run check-ci-workflow` | 0 | Full CI 構造契約 OK |
| `mise run check-ci-tracker-workflows` | 0 | tracker 構造契約 OK |
| `mise run setup-ui` → `check-ui` → `test-ui` | 0 | vendor 照合 OK、typecheck OK、node:test 112 pass |
| `doc_maintenance.js plan/apply` | 0 | 5 index 再生成、残 finding は既存分 |

## Risks

- **consumer-verify tuple 再実行が未実施**: 修復はローカルの
  RED 再現 → `verify-ci` pass で確認したが、consumer-verify
  pipeline 上の tuple(RF candidate × AL 修復 head × SV)
  再実行は本 worker の境界外であり、CI 側での最終確認が残る。
  修復 head push 後に coordinator と調整する
- **macOS / Linux 実 OS lane**: 未実測(本検証は Windows のみ)。
- **al-gui `webview` feature lane**(`check-ui-webview`/
  `test-ui-webview`): 実窓 host 依存、未実行。
- **f64 解析値・実メディア Golden・長尺/性能**: test-ci 内の
  golden は pass。実メディア・性能 re-eval は未実行。
- 既存ドリフト(本変更由来ではない)が検出済みのまま残る:
  `al export` の file sink 構築と USAGE / migration 設計の
  stderr-only 記述の不一致、および artifact-graph の
  broken-relation(al-math 参照先の不存在)

## Follow-ups

- ruviform #334(`RollingFile.filename_suffix`)も解消済みで
  あることを確認。file sink の `SinkDestination::RollingFile`
  移行は本件 scope 外(migration 設計の独立 deferred
  concern)として AL#127 追従の別タスク候補に残す。
- `SpanEvents` の公開型を製品語彙へ再定義する必要はない
  (AL 側に span event 語彙の公開面がないため YAGNI)。
- artifact-graph には本件以前からの broken-relation
  (al-math crate 参照先の不存在)が存在する。本変更とは
  無関係の既存問題として未着手(別タスク)。
- review で検出した既存ドリフト(本変更由来ではない):
  `al export` は `init_logging(.., true)` で file sink を
  構築するが、USAGE と migration 設計は `export` を
  stderr-only と記述(`main.rs:240` 参照)。sink 契約の
  正本側を確定する別タスクとして切り出すのが適当。
- PR マージ・Issue close は coordinator 権限。

---
id: "SPEC-AL127OBSNONINTERFERENCE"
type: "spec"
status: "proposed"
title: "Issue #127 OBS 非干渉 oracle: Logging 状態が f64 解析・CLI JSON・Export に干渉しないこと"
created: "2026-11-28"
updated: "2026-11-28"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/audio-lens/issues/127"
  changes:
    added:
      - type: "file"
        path: "crates/al-cli/tests/obs_noninterference.rs"
      - type: "file"
        path: "docs/specs/issue-127-obs-noninterference.md"
      - type: "file"
        path: "docs/designs/issue-127-obs-noninterference.md"
      - type: "file"
        path: "docs/tasks/issue-127-obs-noninterference.md"
      - type: "file"
        path: "docs/test-specs/issue-127-obs-noninterference.md"
      - type: "file"
        path: "docs/impl/ir/issue-127-obs-noninterference.md"
      - type: "file"
        path: "evidence/al127/obs-noninterference.md"
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements: []
  implemented-by:
    - "../impl/ir/issue-127-obs-noninterference.md"
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "observability-foundation.md"
    - "issue-127-rf628-consumer-oracle.md"
  refines: []
  refined-by: []
  derives-from: []
  derived-by: []
  verifies: []
  verified-by:
    - "../test-specs/issue-127-obs-noninterference.md"
  references: []
  defers: []
  deferred-by: []
---

# Issue #127 OBS 非干渉 oracle

## What Should Be Built

`crates/al-cli/tests/obs_noninterference.rs` に、Observability/Logging が
AudioLens の製品出力に干渉しないことを検査する独立 oracle を新設する。
既存 `crates/al-cli/tests/observability.rs` は変更しない。

同一の決定的 WAV fixture と同一 CLI pipeline
`ingest,cheap_analysis,dataset,export` を、次の 3 状態で実行する。

1. Logging 既定 OFF。
2. `--log info` による Logging ON（stderr sink と file sink）。
3. `<base_dir>/logs` を regular file として占有した file sink 初期化失敗
   （warning 付き stderr 縮退、job 継続）。

3 状態について、抽出した f64 解析値、JSON を返す CLI 出力（`inspect`）、
Export artifact inventory と内容を比較する。job id、base dir、absolute
path、duration など実行ごとに変わる provenance だけを canonical
projection で除き、解析値、artifact 集合、feature 値、hash、export 内容は
比較対象から外さない。byte equality が成立する binary artifact は canonical
化せず exact compare とする。

AL-PRIV の headless 部分として、ON/file-sink fault の stderr・log file と
OFF の stderr に、fixture に埋め込んだ sentinel 文字列と入力 PCM 先頭系列の
具体的表現が露出しないことを確認する。

## Acceptance Criteria

- 新規 integration test target は Cargo の integration test 自動検出で登録
  され、`Cargo.toml`/`Cargo.lock` を変更しない。
- 3 状態の前提 assertion が、既定 OFF で log file を作らないこと、ON で
  stderr+file sink が active であること、fault で warning 付き stderr 縮退
  かつ job 継続であることを確認する。
- fixture、sample rate、channel 数、pipeline、決定性根拠、fixture hash を
  test-spec/design/evidence に記録する。
- canonical projection と除外項目の根拠を design/test-spec に記録する。
- wall time と利用可能な性能指標は hard assert せず evidence に記録する。
- 未実施・環境失敗・範囲外は `NotRun`/`Unsupported`/`InfrastructureFailure`
  として正直に記録し、Pass と書かない。

## Non-goals

- Windows GUI-L2/EXT-L3、macOS/Linux 実走、cross-consumer 検証、consumer pin
  追従、candidate tuple 照合。
- 既存 observability/oracle test のリファクタリング。
- 依存追加、mise lane 追加、CI 変更、docs index 更新。
- 性能閾値の test assertion。

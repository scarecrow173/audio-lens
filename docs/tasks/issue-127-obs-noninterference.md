---
id: "TASKS-AL127OBSNONINTERFERENCE"
type: "tasks"
status: "proposed"
title: "Issue #127 OBS 非干渉 oracle 実装タスク"
created: "2026-11-28"
updated: "2026-11-28"
owners: []
relations:
  source:
    - "../specs/issue-127-obs-noninterference.md"
    - "../designs/issue-127-obs-noninterference.md"
  implements:
    - "../designs/issue-127-obs-noninterference.md"
  verified-by:
    - "../test-specs/issue-127-obs-noninterference.md"
---

# Issue #127 OBS 非干渉 oracle 実装タスク

## タスク分解

1. 既存 `observability.rs`、`rf628_oracle.rs`、observability foundation 文書を
   read-only で調査し、Logging activation/縮退契約と canonical 比較上の
   provenance 要因を固定する。
2. 決定的 fixture（48kHz mono 10 秒 noise + sentinel PCM）と fixture hash を
   実装し、3 状態で同一 bytes を供給する。
3. Logging OFF/ON/file-sink fault の前提 assertion を実装する。
4. `inspect` stdout JSON、f64 feature 値、Export artifact inventory/内容の
   canonical projection と exact compare を実装する。
5. Privacy sentinel と PCM 先頭系列の非露出 assertion を実装する。
6. 正規 7 lane（fmt/check/lint/obs_noninterference/observability/
   rf628_oracle/check-cli-headless）を実行する。
7. `evidence/al127/obs-noninterference.md` に verdict、環境、fixture hash、
   性能観測、範囲外項目を記録する。
8. PR 本文に `Refs #127`、ローカル検証と統合検証の区別、docs index 未同期、
   範囲外項目を記録する。

## 依存関係と制約

- spec → design → 本 tasks/test-spec → implementation record の順で追跡する。
- `Cargo.toml`、`Cargo.lock`、既存 test、docs index、CI、mise 設定は変更しない。
- 製品不具合が見つかった場合は成功条件を弱めず、lease 外修正は事前承認を
  必要とする。

## 完了条件

- `crates/al-cli/tests/obs_noninterference.rs` が 3 状態 oracle として Pass
  する。
- 正規 7 lane の exit code と結果が evidence/PR に記録される。
- 未実施・範囲外が `NotRun`/`Unsupported`/`NotApplicable` で明示される。

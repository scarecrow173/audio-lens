---
id: "PLAN-izcRdci5WYLehmwNZXzE1E"
type: "plan"
status: "approved"
title: "ruviform#464 Step C / AL#127 SemanticGuiTest scenario 採用の計画"
created: "2026-11-09"
updated: "2026-11-09"
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
    - "docs/specs/ruviform-464-scenario-adoption.md"
  implemented-by: []
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related: []
  refines: []
  refined-by: []
  derives-from:
    - "docs/specs/ruviform-464-scenario-adoption.md"
    - "docs/designs/ruviform-464-scenario-adoption.md"
  derived-by: []
  verifies: []
  verified-by:
    - "docs/test-specs/ruviform-464-scenario-adoption.md"
  references: []
  defers: []
  deferred-by: []
---

# ruviform#464 Step C / AL#127 SemanticGuiTest scenario 採用の計画

## 概要

ruviform pin を `54439fdc` → `84d21641` へ更新し（RF#464 Scenario
Contract・RF#314 Golden 接続を含有）、al-gui の desktop jobs 画面と
source waveform/seek 画面に driver-neutral `SemanticGuiTest`
scenario を採用する。shared scenario は `tests/common/scenario.rs`
に置き、`tests/scenario.rs`（Fake）と `tests/scenario_gpui.rs`
（GPUI、`gpui-test` feature）で同一関数を実行する。

## File Map

| ファイル | 変更 |
| --- | --- |
| `Cargo.lock` | ruviform 20 package の解決 SHA を `84d21641` へ |
| `crates/al-gui/tests/common/mod.rs` | `pub mod scenario;` 追加 |
| `crates/al-gui/tests/common/scenario.rs` | AL fixture + driver-neutral scenario + report helper（新規） |
| `crates/al-gui/tests/scenario.rs` | Fake adapter scenario 検証（新規、headless） |
| `crates/al-gui/tests/scenario_gpui.rs` | GPUI adapter scenario 検証（新規、`gpui-test`） |
| `docs/{specs,designs,plans,tasks,test-specs}/ruviform-464-scenario-adoption.md` | 文書チェーン（新規） |

## Tasks

1. ruviform pin を `84d21641a6bd50fa570ed074ae263284d05262ca` へ更新
   （`node tools/update-ruviform-rev.mjs <sha>`）。
2. `tests/common/scenario.rs` に AL fixture（desktop jobs spec、
   source seek spec）と scenario 関数を実装。
3. `tests/scenario.rs` に Fake adapter の semantic gesture pass・
   target resolution・stale pin・capability・Unsupported 報告を実装。
4. `tests/scenario_gpui.rs` に GPUI adapter の同一 scenario 実行・
   能力境界・capture/golden Unsupported を実装。
5. `mise run fmt` / `check` / `lint` / `test` / `check-ui-gpui` /
   `lint-ui-gpui` / `test-ui-gpui` / `check-ruviform-lock` /
   `check-ruviform-rev` で検証。
6. `git fetch origin main && git merge origin/main` で最新 main を
   取り込み、lockfile・docs index を再生成して再検証。
7. Conventional Commits で論理単位に分割して commit、feature branch
   を push、PR を作成。

## Verification Matrix

| 検証 | lane | 対象 |
| --- | --- | --- |
| fmt | `mise run fmt-check` | 全変更ファイル |
| 型/lint | `mise run check` `mise run lint` | workspace（headless） |
| test | `mise run test` | workspace（headless、`tests/scenario.rs` 含む） |
| GPUI check/lint/test | `mise run check-ui-gpui` `lint-ui-gpui` `test-ui-gpui` | `gpui-test` feature（`tests/scenario_gpui.rs` 含む） |
| lock 整合 | `mise run check-ruviform-lock` `check-ruviform-rev` | Cargo.lock の pin |

## Risks

- pin 更新に伴う API 追従 → `check-ui-gpui` で早期検出（現行 pin で
  pass 済み）。
- GPUI adapter の非同期 lifecycle → `settle()` を mount/scenario 後に
  挟み、`gpui_parity.rs` の既存パターンに倣う。
- doc graph の既存 broken relation（`crates/al-math` 欠落 path）→
  本タスクのスコープ外として記録し、修復しない。

## Rollback

`Cargo.lock` と `crates/al-gui/tests/` の新規ファイルを revert すれば
pin `54439fdc` の状態に戻る。製品コード変更がないため影響範囲は
テストと lockfile に限定される。

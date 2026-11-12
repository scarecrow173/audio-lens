---
id: "IMPL-034TTEFI2xaG6ZgMwpNWY9"
type: "impl"
status: "completed"
title: "Native GUI の Dataset workspace Tabs 選択契約の採用と pointer 切替の実装記録(IR, 第10 slice)"
created: "2026-11-08"
updated: "2026-11-08"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/audio-lens/issues/166"
  changes:
    added: []
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements: []
  implemented-by: []
  depends-on:
    - "../../tasks/native-gui-workspace-tabs-selection.md"
  blocks: []
  supersedes: []
  superseded-by: []
  refines: []
  refined-by: []
  derives-from:
    - "../../plans/native-gui-workspace-tabs-selection.md"
  derived-by: []
  verifies: []
  verified-by: []
  references:
    - "https://github.com/scarecrow173/ruviform/pull/458"
  defers: []
  deferred-by: []
metadata:
  experiments:
    adopted: []
    rejected: []
---

# Native GUI の Dataset workspace Tabs 選択契約の採用と pointer 切替の実装記録(IR, 第10 slice)

## Summary

Dataset workspace の Tabs(Source・Spectrogram・Analysis)に ruviform の
選択契約を採用し、pointer 切替を実装した Native GUI slice(第10 slice)の記録。
Issue #166 の一部。

## Context

Issue #166(https://github.com/scarecrow173/audio-lens/issues/166)の slice。
task `docs/tasks/native-gui-workspace-tabs-selection.md` と
plan `docs/plans/native-gui-workspace-tabs-selection.md`(derives-from)に従う。
ruviform 側の Tabs 選択契約は PR scarecrow173/ruviform#458 を参照。
第9 slice(root Tabs の `al.desktop` target)の後続として workspace 側の
`al.workspace` target を追加した。

## Implementation

### Changes

- `al-gui::command`: `WORKSPACE_TARGET`（`al.workspace`）を追加。`ACTION_TAB_SELECT` は
  ruviform の `TAB_SELECT_ACTION` を再利用したまま（第9 slice と同一の定数）。
- `al-gui::view::workspace`: `WORKSPACE_TAB_COUNT = 3`（Source・Spectrogram・Analysis）を定義し、
  workspace の Tabs を `Some(command::WORKSPACE_TARGET)` で組む（従来は `None` で非選択）。
- `al-gui::view::desktop`: module doc に workspace Tabs の選択契約への参照を追記。root Tabs の
  `DESKTOP_TARGET`・`TAB_COUNT` は変更なし。
- `al-gui::service::command()`: `(target, action)` match に `(WORKSPACE_TARGET, tab-select)` を追加。
  desktop と workspace で共通の `tab_select(params, limit)` helper に params 検証（`index` が
  u64 かつ `index < limit`）と `{"status": "applied", "index": n}` 応答を集約した。
  範囲外・params 欠落・型不一致・未知 target/action は `invalid`（`INVALID_PARAMS`）。
- `al-gui::desktop`（session）と `bin/al-desktop.rs`: 変更なし。workspace の `tab-select` も
  runtime → service → receipt の経路で処理され、session の `selected_job`・`projection_dirty` を
  動かさない。表示 tab の正本は ruviform の `ViewHost` のまま。
- ruviform の依存 rev は `a5b1db13` のまま（`Cargo.lock`・`ui/vendor` の変更なし）。

### テスト（先に書いた）

テストは実装より先に書いた。実装前の時点では `WORKSPACE_TAB_COUNT`・`WORKSPACE_TARGET` が
未定義のため、対象テストはコンパイルに失敗する（確認したのはこのコンパイル失敗のみで、
振る舞いの red は観測していない — 第9 slice と同じ状況）。

### Decisions During Implementation

- **target を `al.workspace` とした**: `al.desktop`（root Tabs）と index 空間が別のため、
  service の `(target, action)` match で区別する。`al.job` は行選択・job 操作であり tab 選択と
  意味が異なるため、これも分けた。
- **index 上限を `WORKSPACE_TAB_COUNT` に分けた**: root は `TAB_COUNT = 3`、workspace も 3 だが
  別の tab 集合であり、片方の増減が他方を壊さないよう target ごとに持つ（design §3）。
- **params 検証を service に集約した**: desktop と workspace で同じ規則のため `tab_select` helper に
  まとめ、`limit` だけを target ごとに渡す。ruviform の事前検証と重複するが、IPC 境界の防御として残す。
- **session に tab 状態を持たせなかった**: 表示 tab の正本は ruviform の `ViewHost`。consumer 側に
  第二の状態を持たない（第9 slice と同じ判断）。
- **実 OS window での pointer 確認は NotRun**: GPUI observer 経路（`select_tab` → intent → snapshot）は
  gpui-test で parity 検証済み。実 window のマウス配線は ruviform 側の gpui-test が担保する範囲。
- **ruviform の issue は起票していない**: 本 slice の検証で ruviform 側の欠陥は見つからなかった。
  キーボード（FU-1）は upstream の既存作業のため重複起票しない。

## Related Experiments

本 slice に関連する実験ログは存在しない。`metadata.experiments` の
adopted / rejected はいずれも空で、`docs/impl/exp/` の既存 jsonl
(issue-139 / 143 / 148 / 149 / 153 の CI 関連記録)は本件と無関係。

### Adopted

- なし

### Rejected

- なし

## Validation

### 受入対応表

| 受入（spec） | 結果 | 根拠 |
| --- | --- | --- |
| R1 workspace Tabs の selection target が `al.workspace` | Passed | `workspace_tabs_are_selectable_by_pointer_on_the_workspace_target`（`TabsSpec.selection.target`・tab key 順・`WORKSPACE_TAB_COUNT` を検証） |
| R2 select_tab の intent（target・action・params） | Passed | 同上、`workspace_tab_selection_intent_and_selected_tab_match_between_fake_and_gpui` |
| R3 範囲内 applied・表示保持 | Passed | `command_workspace_tab_select_applies_an_in_range_index_and_rejects_the_rest`、`workspace_tab_select_replies_applied_and_rejected_without_touching_the_projection`、`workspace_tab_selection_is_applied_and_survives_reprojection` |
| R4 範囲外・不正・誤 target の rejected と巻き戻し | Passed | 上記 service・session テスト、`rejected_tab_selection_restores_the_previous_tab`（workspace 側も rejected → Source へ復帰） |
| R5 job 選択・dirty の不変 | Passed | `workspace_tab_select_replies_applied_and_rejected_without_touching_the_projection`（`selected_job` None・dirty false）、`workspace_tab_selection_is_applied_and_survives_reprojection` |
| R6 同一 spec 再射影での保持 | Passed | `workspace_tab_selection_is_applied_and_survives_reprojection`（Spectrogram 選択 → 同 key・同 `active` の再射影後も `selected` = 1） |
| R7 Fake/GPUI parity | Passed | `workspace_tab_selection_intent_and_selected_tab_match_between_fake_and_gpui`（`test-ui-gpui` の gpui_parity 12 件中） |
| R8 検証 | Passed | 下記コマンド表 |

以下、コード HEAD `7b924e6`（main、第9 slice merge 済み）の上の作業ツリーで
実行した検証結果。
`git fetch origin main && git merge origin/main` は "Already up to date"。
すべて `mise run` 経由（canonical）で実行した。環境: Windows x64、rustc/cargo は mise 管理の
repo toolchain（`mise run fmt` 等が解決）、mise 2026.10.1。

| コマンド（mise task） | exit | 結果 |
| --- | --- | --- |
| `mise run sync-ui-deps` | 0 | Passed（`ui/vendor/ruviform` @ `a5b1db13`） |
| `mise run setup-ui` | 0 | Passed（`ui/node_modules` 復元。fresh worktree の初期化） |
| `mise run fmt` | 0 | Passed |
| `mise run check`（cargo check --workspace --all-targets） | 0 | Passed |
| `mise run lint`（biome check + clippy `-D warnings`） | 0 | Passed（biome の既存 warning/info のみ、clippy clean） |
| `mise run test`（cargo test --workspace） | 0 | Passed（failed 0） |
| `mise run test-ui-gpui`（cargo test -p al-gui --features gpui-test） | 0 | Passed（gpui_parity 12 件を含む al-gui 全件 failed 0） |
| `mise run lint-ui-gpui` | 0 | Passed |
| `mise run check-ui-gpui` | 0 | Passed |
| `mise run check-ui` | 0 | Passed |
| `mise run test-ui` | 0 | Passed（112 件、failed 0） |
| `mise run check-cli-headless` | 0 | Passed（10 headless crate に GUI framework 依存なし） |
| `mise run check-ruviform-lock` | 0 | Passed（20 packages が `a5b1db13`） |
| `mise run check-ruviform-rev` | 0 | Passed（lockfile primary、REV match） |

注: `check`/`lint`/`test-ui-gpui` の出力に依存クレート（proc-macro-error2）の
future-incompat 警告があるが、本 slice の変更とは無関係の既存の警告である。

### NotRun / Unsupported

- **実 OS window での pointer タブ切替**: NotRun。GPUI の mouse 押下から `select_tab` までの配線は
  ruviform の gpui-test が検証する。AudioLens 側は `select_tab` 以降（intent・receipt・snapshot）を検証した。
- **キーボードによるタブ切替**: NotRun（upstream FU-1、別 worker が対応中）。
- **`check-ui-driver-boundary`**: mise.toml に存在しない task のため NotRun（PR に明記）。

### 独立レビュー（review-protocol）

- 実施: 1 回。subagent の起動は worker 契約（helper worker 禁止）により行わず、
  Reviewer・Validator・Judge を分離した pass として同一エージェントが実施した。
  このため独立性は subagent 実行より低い点に注意する。
- Reviewer 観点: 契約の一致（target・action・params・index 空間）、service の境界検証、
  session の不変（`is_action` が action 名のみで判定する経路に `tab-select` が誤って入らないか、
  `adopt_selection` が `job_id` を要求するため tab receipt を採用しないこと）、
  再射影の保持、テストの偽陽性、他の `tabs()` 呼び出し箇所との競合。
- Issue Candidate:
  1. （P3）`tests/desktop.rs` の workspace tab テストは `command_intent_on` が `control_id: "job-table"`
    を入れる（第9 slice と同じ既知の点）。service は読まず意味も変えないため非ブロッキング。
  2. （P3）workspace Tabs が Dataset root tab 非選択時にも mount され `select_tab` 可能であることは
    receipt テストで暗黙に確認されたが、pointer の実経路（Dataset 表示中のクリック）としての
    明示的検証は gpui_parity 側に委ねた。
- Validator: 上記はいずれも `CONFIRMED` だが P0/P1 は無し。
- Judge: Blocking なし。判定は **ACCEPT WITH NON-BLOCKING NOTES**。P2/P3 は報告のみで修正ループには入れていない。

## Risks

- 実 OS window での pointer タブ切替は AudioLens 側で未検証。GPUI observer
  経路（`select_tab` → intent → snapshot）は gpui-test で parity 検証済みだが、
  実 window のマウス配線は ruviform 側の gpui-test が担保する範囲。
- workspace Tabs が Dataset root tab 非選択時にも mount され `select_tab`
  可能であることは receipt テストで暗黙に確認されたのみで、pointer の実経路
  （Dataset 表示中のクリック）としての明示的検証は gpui_parity 側に委ねた
  （独立レビューの P3 指摘）。
- キーボードによるタブ切替は未対応（upstream FU-1、別 worker が対応中）。

## Follow-ups

- 実 OS window での pointer 確認（NotRun）は Coordinator 側の手元確認に委ねる。
- キーボードによるタブ切替は upstream FU-1（ruviform 側作業）に依存する。

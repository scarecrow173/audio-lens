---
id: "TASK-034TXamYZkkYpc4jWRMRXa"
type: "task"
status: "in-progress"
title: "Spectrogram tab hang 解消のタスク"
created: "2026-11-08"
updated: "2026-11-08"
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
    - "docs/plans/spectrogram-tab-hang.md"
  implemented-by: []
  depends-on:
    - "docs/plans/spectrogram-tab-hang.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related: []
  refines: []
  refined-by: []
  derives-from: []
  derived-by: []
  verifies: []
  verified-by:
    - "docs/test-specs/spectrogram-tab-hang.md"
  references: []
  defers: []
  deferred-by: []
---

## タスク

| # | 内容 | 状態 |
| --- | --- | --- |
| 1 | ruviform pin を `863a7a9d` へ更新(Cargo.lock + ui/vendor 同期。当初 `129786ba`、RF#461 merge 後に再追従) | 完了 |
| 2 | `gpui_parity` workspace tab selection テストへ spectrogram drawn assertion 追加 | 完了 |
| 3 | `spectrogram_tab_paint` 回帰テスト新設(本番解像度 65,536 cell) | 完了 |
| 4 | spec/design/plan/task/test-spec/IR 文書チェーン作成 | 完了 |
| 5 | mise 検証(fmt/check/lint/test/check-cli-headless/check-ruviform-*/sync-ui-deps/check-ui*/lint-ui-gpui/test-ui-gpui) | 進行中 |
| 6 | 実 OS window(debug al-desktop)で Spectrogram tab pointer 切替の完了証拠取得 | 完了 |
| 7 | origin/main merge → 再検証 → commit/push/PR | 未着手 |

## 完了条件

- Spectrogram tab 選択で恒久的 hang が解消され、GPUI parity と実機で
  切替完了が確認できること。
- ~~残課題(O(n²) paint、debug 約 12 秒/回)が ruviform#461 として記録
  され、PR 本文・文書で区別されていること。~~
  → ruviform#461 は PR #462 として merge 済み(merge commit
  `863a7a9d`)。pin 再追従で paint も数十 ms に改善し、残課題は解消。

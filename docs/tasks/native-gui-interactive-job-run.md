---
id: "TASK-s1DH5J3jmMH1T2IOPpfw65"
type: "task"
status: "in-progress"
title: "Issue #166 Jobs 対話選択と Run 再開・取消 実装タスク (第7 slice)"
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
  implements:
    - "../plans/native-gui-interactive-job-run.md"
  implemented-by: []
  depends-on:
    - "../plans/native-gui-interactive-job-run.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "../specs/native-gui-interactive-job-run.md"
    - "../designs/native-gui-interactive-job-run.md"
    - "../test-specs/native-gui-interactive-job-run.md"
    - "https://github.com/scarecrow173/ruviform/issues/456"
  refines: []
  refined-by: []
  derives-from:
    - "../plans/native-gui-interactive-job-run.md"
  derived-by: []
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# Issue #166 Jobs 対話選択と Run 再開・取消 実装タスク (第7 slice)

## 完了条件

設計 `native-gui-interactive-job-run.md` の受入（spec A1〜A9）を満たし、検証マトリクス（plan）の項目を実行して結果を PR に記録する。

## タスク

- [x] T1 ruviform rev を `a998101c` へ更新し、`check-ruviform-lock`・`check-ruviform-rev` を通す（`3f30b2b`）
- [x] T2 `command` 語彙（`crates/al-gui/src/command.rs`）と `rvf.ui.command.v1` の service op（select・resume）
- [x] T3 view の射影: Jobs 表の selection、Run tab の Resume/Cancel Button、`table` の selection 引数
- [x] T4 session の選択追跡（receipt `applied` の `job_id` 採用）・cancel 受理・dirty 通知
- [x] T5 al-desktop の初期選択と dirty 時の `update_view` 再射影
- [x] T6 テスト: service op（3）・session（4）・screen（1）・GPUI parity（1）
- [x] T7 文書: spec・design・plan・task・test-spec、`native-gui-job-run-mount.md` §3・§6・§8、各 README 索引
- [ ] T8 検証: `fmt`・`check`・`lint`・`test`・`check-cli-headless`・`check-ruviform-lock`・`check-ruviform-rev`・`sync-ui-deps`・`check-ui`・`test-ui`・`check-ui-gpui`・`lint-ui-gpui`・`test-ui-gpui`
- [ ] T9 `git fetch origin main && git merge origin/main` で最新 main を取り込み、生成物（`Cargo.lock`・docs 索引）を再生成
- [ ] T10 コミット（論理単位）・push（`feature/166-interactive-job-run`）・PR（`Refs #166`）
- [ ] T11 ruviform#456 と AudioLens PR を相互参照（ruviform 側へコメント）

## 未実施として残すもの

- 新規 run の起動（`al.job.run`）の UI 接続（入力源 UI が無い）
- 初期選択行の Jobs 表での強調
- 実 OS window の目視確認（NotRun）
- Tabs の active 保持（再射影後の挙動は未検証）

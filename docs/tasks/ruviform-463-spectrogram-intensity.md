---
id: "TASK-034U1kYwpGf3uFTaAbXAvM"
type: "task"
status: "done"
title: "ruviform pin 54439fdc 追従と Spectrogram intensity 実窓検証のタスク"
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
    - "docs/plans/ruviform-463-spectrogram-intensity.md"
  implemented-by: []
  depends-on:
    - "docs/plans/ruviform-463-spectrogram-intensity.md"
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
    - "docs/test-specs/ruviform-463-spectrogram-intensity.md"
  references: []
  defers: []
  deferred-by: []
---

# ruviform pin 54439fdc 追従と Spectrogram intensity 実窓検証のタスク

## Work

ruviform pin を `863a7a9d` → `54439fdc` へ追従し、Dataset workspace
Spectrogram tab の実窓 pixel 解析で intensity 勾配を確認する
consumer 検証タスク。コード変更はなく、`Cargo.lock`・`ui/vendor`・
文書チェーン・証拠のみを扱う。

| # | 内容 | 状態 |
| --- | --- | --- |
| 1 | ruviform pin を `54439fdc7113b99fb551d370a90c1fc3addcb7a0` へ更新(Cargo.lock + ui/vendor 同期) | 完了 |
| 2 | spec/design/plan/task/test-spec/IR 文書チェーン作成 | 完了 |
| 3 | mise 検証(fmt/check/lint/test/build/check-cli-headless/check-ui/test-ui/check-ui-gpui/lint-ui-gpui/test-ui-gpui/check-ruviform-lock/check-ruviform-rev/sync-ui-deps) | 完了 |
| 4 | 実 OS window(al-desktop debug)で Spectrogram tab の intensity 勾配 pixel 解析・証拠保存 | 完了 |
| 5 | origin/main merge → 再検証 → commit/push/PR(Refs scarecrow173/ruviform#463) | 進行中 |

## Classification

- `normal-plan-task`

## Upstream Documents

- `docs/plans/ruviform-463-spectrogram-intensity.md`
- `docs/specs/ruviform-463-spectrogram-intensity.md`
- `docs/designs/ruviform-463-spectrogram-intensity.md`
- `docs/test-specs/ruviform-463-spectrogram-intensity.md`

## Dependencies

- ruviform `54439fdc`(RF#463 / PR #466 merge commit)が ruviform main
  に存在すること — 確認済み。
- fixture job `JOB-Zt7KXJqz7qf8fo9JBcmlX1` の dataset が
  `F:/repositry/audio-products/.devin/evidence/al166-acceptance-verify/jobs/`
  に存在すること。

## Affected Files

- `Cargo.lock`(ruviform source fragment の SHA)
- `ui/vendor/`(gitignore 済み生成物、`sync-ui-deps` で再生成)
- `docs/specs|designs|plans|tasks|test-specs|impl/ir/ruviform-463-spectrogram-intensity.md` + 各 README index

## Done When

- [x] pin が `54439fdc` 以降で `check-ruviform-lock`/`check-ruviform-rev` が pass。
- [x] 実窓 Spectrogram pixel 解析で intensity 勾配(中間色・複数色)を確認し証拠を `.devin/evidence/` に保存。
- [x] `spectrogram_tab_paint`・`gpui_parity` 等の既存回帰テストが pass。
- [x] 正規の mise 検証コマンドが全て pass。

## Verification

- [x] `mise run fmt` / `check` / `lint` / `test` / `build` /
  `check-cli-headless` / `check-ui` / `test-ui` / `check-ui-gpui` /
  `lint-ui-gpui` / `test-ui-gpui` / `check-ruviform-lock` /
  `check-ruviform-rev` / `sync-ui-deps`
- [x] 実窓 capture の pixel 解析結果(色分布)を証拠として保存。

---
id: "TASK-ntv7UntJkPy8ECuh6iG7Lx"
type: "task"
status: "in-progress"
title: "Issue #166 drawn parity の Spectrogram・Label・Table 拡張と Job/Run 射影 実装タスク (第5 slice)"
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
    - "../plans/native-gui-drawn-parity-run-mount.md"
  implemented-by: []
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  refines: []
  refined-by: []
  derives-from:
    - "../specs/native-gui-drawn-parity-run-mount.md"
  derived-by: []
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# Issue #166 drawn parity の Spectrogram・Label・Table 拡張と Job/Run 射影 実装タスク

## T1: ruviform rev を 3121f8e6 へ更新

- [x] `node tools/update-ruviform-rev.mjs 3121f8e69c5d6f300ba9c8fcf952425f0cbeeb5d`(Cargo.lock の 20 source のみ)
- [x] `mise run check-ruviform-lock` / `sync-ui-deps` / `check-ruviform-rev --require-vendor`

## T2: drawn parity の Spectrogram・Label・Table 照合

- [x] `tests/gpui_parity.rs` に fixture(gap 列・非有限セル)と Fake 期待値の導出を追加
- [x] Spectrogram・Label・Table の照合テスト 3 件を追加(`mise run test-ui-gpui` 相当を実行して pass)

## T3: Job/Run の射影 e2e 拡張

- [x] `tests/view.rs::real_dataset_produces_view_models` に job 一覧 Table・Run 画面・Fake mount を追加

## T4: Job/Run の al-desktop mount 判断

- [x] 入力源を確認し、tab 追加は見送る判断を設計 4 節に記録(未達)
- [ ] coordinator による base_dir・Job 操作の UI 契約判断(未決。AL-G4 へ送る)

## T5: RF443 の上流契約移行

- [x] ruviform#443 が OPEN であることを確認し、`spectrogram_columns` の複製を維持
- [ ] ruviform#443 merge 後に上流 API へ差し替える(NotRun: 上流未 merge)

## T6: 文書 chain と索引

- [x] spec・design・plan・test-spec・tasks を追加
- [x] impl IR を追加し、生成索引を再生成する(`docs/impl/ir/native-gui-drawn-parity-run-mount.md`)

## T7: 検証と完了

- [x] `mise run fmt` / `check` / `lint` / `test` / `build`(merge 前の作業 HEAD で実行)
- [x] `mise run check-ui-gpui` / `lint-ui-gpui` / `test-ui-gpui`
- [x] `mise run check-cli-headless` / `check-ruviform-lock` / `check-ruviform-rev` / `sync-ui-deps` / `check-ui` / `test-ui`(`setup-ui` 後)
- [x] review-protocol(独立サブエージェントによる diff レビュー)。blocker なし(IR の記録参照)
- [ ] PR 作成(日本語、`Refs #166`、Issue は close しない)と head SHA の報告
- [ ] merge 後 HEAD で最終検証(coordinator の merge 後に実施)

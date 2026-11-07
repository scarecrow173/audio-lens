---
id: "PLAN-1FzCSROtUwCSQOdQx5Ua4f"
type: "plan"
status: "draft"
title: "Native GUI drawn parity の Spectrogram・Label・Table 拡張と Job/Run 射影の実装計画 (第5 slice)"
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
    - "../specs/native-gui-drawn-parity-run-mount.md"
  implemented-by: []
  depends-on:
    - "../plans/native-gui-spectrogram-analysis-projection.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "../designs/native-gui-drawn-parity-run-mount.md"
    - "../test-specs/native-gui-drawn-parity-run-mount.md"
    - "../tasks/native-gui-drawn-parity-run-mount.md"
  refines: []
  refined-by: []
  derives-from:
    - "../designs/native-gui-drawn-parity-run-mount.md"
  derived-by: []
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# Native GUI drawn parity の Spectrogram・Label・Table 拡張と Job/Run 射影の実装計画

## Scope

設計 `DESIGN-KmsgLgUSS9WY4blzhdDRqp` の 1〜3 節を実装する。al-desktop の Job/Run tab 追加と
RF443 の複製除去は本計画の対象外(設計 4・5 節の判断に従い未達)。

## File map

| ファイル | 変更 | 目的 |
| --- | --- | --- |
| `Cargo.lock` | 変更(ruviform 20 source のみ) | rev を `3121f8e6` へ |
| `ui/vendor/ruviform/**` | 生成(gitignore) | `sync-ui-deps` で再生成 |
| `crates/al-gui/tests/gpui_parity.rs` | 変更 | Spectrogram・Label・Table の drawn 照合 |
| `crates/al-gui/tests/view.rs` | 変更 | 実 job の Job 一覧・Run 射影と Fake mount |
| `docs/{specs,designs,plans,test-specs,tasks,impl/ir}/native-gui-drawn-parity-run-mount.md` | 新規 | 文書 chain |
| `docs/*/README.md` | 更新(生成索引) | 文書索引の登録 |

`src/` の production コードは変更しない。

## Dependency graph

1. Cargo.lock の rev 更新 → `sync-ui-deps` → `check-ruviform-lock` / `check-ruviform-rev`
2. gpui_parity の drawn 照合(1 に依存: `DrawnPlan` の新 variant)
3. view の Job/Run e2e(1 に依存しない。2 と並行可)
4. 文書 chain(1〜3 の結果を反映。IR は検証後)
5. 検証(fmt → check → lint → test → gpui・ui・headless・ruviform check)
6. commit・PR・review-protocol・merge 後 HEAD の再検証

## Risk register

| リスク | 影響 | 対策 |
| --- | --- | --- |
| 期待値が実装と同じ式の複製で、同じ誤りを両方が含む | 偽陽性 | 期待値は contract の規則(viewport 分数・clamp・gap 除外)だけから導き、fixture の gap・非有限セル数を assert する |
| Table の欠損セル比較が `Option<String>` の表示差で揺れる | 誤判定 | 期待値は `CellValue::to_string` を使い、欠損セルの存在を assert する |
| 実 job の Run 形が al-gui 側の `stage_statuses` と乖離 | 射影の不一致が検出されない | 設計 3 節の既知の制約として記録。形を変える場合は両方を同時更新 |
| Cargo.lock の更新で他の ruviform crate が意図せず変わる | 再現性 | diff が 20 source の SHA 変更のみであることを確認 |

## Rollback

`git checkout -- Cargo.lock` と `ui/vendor` の再生成(`mise run sync-ui-deps`)で rev を
`627e4bc2` へ戻せる。テストの追加は独立しており、削除のみで戻せる。

## Verification matrix

| 検証 | 対象 | 手段 |
| --- | --- | --- |
| rev 整合 | Cargo.lock / ui/vendor | `mise run check-ruviform-lock`・`check-ruviform-rev`・`sync-ui-deps` |
| drawn parity | Spectrogram・Label・Table | `mise run test-ui-gpui` |
| Job/Run 射影 e2e | 実 job → Table / Label / Fake mount | `mise run test`(`tests/view.rs`) |
| 型・lint | 全 crate | `mise run check` / `lint` / `check-ui-gpui` / `lint-ui-gpui` |
| GPUI 型の隔離 | headless 依存木 | `mise run check-cli-headless` |
| 数値 oracle 非回帰 | 全 crate | `mise run test` 全件 |
| 文書整合 | docs | `doc-status` の audit(生成索引の差分なし) |

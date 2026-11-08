---
id: "PLAN-OQLFcsEL9ZueFsfsB5zoZU"
type: "plan"
status: "draft"
title: "Native GUI の Job/Run tab mount と Tile::columns 消費の実装計画 (第6 slice)"
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
    - "../designs/native-gui-job-run-mount.md"
  implemented-by: []
  depends-on:
    - "native-gui-drawn-parity-run-mount.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "../specs/native-gui-job-run-mount.md"
    - "../test-specs/native-gui-job-run-mount.md"
  refines: []
  refined-by: []
  derives-from:
    - "../designs/native-gui-job-run-mount.md"
  derived-by: []
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# Native GUI の Job/Run tab mount と Tile::columns 消費の実装計画

## Scope

- 設計書 `../designs/native-gui-job-run-mount.md` の §1〜§7 を実装する。
- 対象は `crates/al-gui`(view・bin・tests)、`Cargo.lock`(rev)、`docs/`。
- 対象外: ruviform 側の実装、Job の起動・再開・取消、対話的な job 選択、AL-G5。

## File map

| ファイル | 変更 | 内容 |
| --- | --- | --- |
| `Cargo.lock` | 更新 | ruviform 20 source を `d3be1ad9` へ(`tools/update-ruviform-rev.mjs`) |
| `ui/vendor/ruviform`(生成物・ignore) | 再生成 | `sync-ui-deps` |
| `crates/al-gui/src/view/screen.rs` | 変更 | `spectrogram_columns` 削除、`Tile::columns` を消費 |
| `crates/al-gui/src/view/mod.rs` | 変更 | `tabs`・`unavailable` を共有 helper として追加 |
| `crates/al-gui/src/view/workspace.rs` | 変更 | 共有 helper へ移行 |
| `crates/al-gui/src/view/job.rs` | 変更 | `RunOutcome::from_manifest` 追加 |
| `crates/al-gui/src/view/desktop.rs` | 新規 | root Tabs(Jobs・Run・Dataset)の射影 |
| `crates/al-gui/src/bin/al-desktop.rs` | 変更 | 引数 `[<base-dir> [<job-id>]]`、`desktop_screen` の mount |
| `crates/al-gui/tests/common/mod.rs` | 変更 | `make_job` fixture |
| `crates/al-gui/tests/desktop_screen.rs` | 新規 | 実 job の tab 構造・Job 一覧・Run・Dataset・fallback |
| `crates/al-gui/tests/spectrogram.rs` | 変更 | `Tile::columns` 一致・空 tile の境界 |
| `crates/al-gui/tests/gpui_parity.rs` | 変更 | `find` の Tabs 対応、desktop の drawn parity |
| `docs/**` | 新規・更新 | spec・design・plan・test-spec・task・ir、各 README の索引 |

## Dependency graph

1. Cargo.lock の rev 更新と `sync-ui-deps`(以降の全工程の前提)
2. RF443 の消費(`screen.rs`)— 独立。テストを先に追加する
3. 共有 helper の移行(`mod.rs`・`workspace.rs`)— 2 の後でもよい
4. `RunOutcome::from_manifest`(`job.rs`)
5. `view::desktop`(3・4 に依存)
6. `al-desktop` の引数と mount(5 に依存)
7. テスト(`desktop_screen.rs`・`gpui_parity.rs`)(5 に依存)
8. 検証(全 mise task)・IR・レビュー・PR

## Risk register

| リスク | 影響 | 対策 |
| --- | --- | --- |
| 上流の列規則が複製と異なる | Spectrogram の列 frame 区間がずれる | `Tile::columns` を唯一の規則にし、非空 tile は既存テストで比較、空 tile は新テストで固定 |
| 旧 CLI 利用者の起動が壊れる | 旧 `<dataset-root> <input-id>` が使えない | pre-release のため互換層は作らず、PR と IR に明記 |
| Dataset tab の入力選択の誤り | 別 input の画面が出る | 先頭 input を仕様化し、テストで input の一致を確認 |
| `make_job` のコストで test が遅い | CI 時間の増加 | 1 秒の wav・3 stage に限定。実測 < 1 秒 |
| GPUI parity の Tabs 観測の仕様差 | drawn 照合が偽陽性になる | active tab のみが描かれる前提を `[JOBS_KEY]` の期待で固定 |

## Rollback

- rev: `git checkout -- Cargo.lock` の後に `mise run sync-ui-deps`。
- 画面: `view/desktop.rs` と `al-desktop.rs` の変更を戻す。`screen.rs` の変更は単独で戻せる。
- 本 slice の commit は論理単位ごとに分けるため、単位ごとの revert が可能。

## Verification matrix

| 観点 | 検証 | 期待 |
| --- | --- | --- |
| rev 整合 | `check-ruviform-lock`・`check-ruviform-rev`・`sync-ui-deps`・`check-ui` | 全 20 source が `d3be1ad9` に一致 |
| 型・lint | `check`・`lint`・`check-ui-gpui`・`lint-ui-gpui` | 警告ゼロ |
| 射影 e2e | `test`・`test-ui-gpui`(`desktop_screen`・`spectrogram`・`gpui_parity`) | 全件 pass |
| GPUI 境界 | `check-cli-headless` | GUI 依存が CLI に現れない |
| 整形 | `fmt` | 差分なし |
| 最終 | 最終 commit SHA 上で上記を再実行 | 同一 SHA の結果として記録 |

## Commit 境界

1. `chore(ruviform)`: rev 更新(`Cargo.lock`)
2. `feat(al-gui)`: `Tile::columns` の消費(RF443)とテスト
3. `feat(al-gui)`: `view::desktop`・`RunOutcome::from_manifest`・al-desktop の引数と mount・テスト
4. `docs`: spec・design・plan・test-spec・task、索引
5. `docs`: IR(実装結果)

検証は最終 commit の SHA で全 task を実行する(中間 commit ごとの検証は行わない)。各 commit は 1 つの論理変更に限る。

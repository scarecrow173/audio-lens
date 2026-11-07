---
id: "TASK-nyXpQYAjTy5czWCwaEXHcz"
type: "task"
status: "in-progress"
title: "Issue #166 Native 初期 slice AL-G1〜G3 実装タスク"
created: "2026-11-07"
updated: "2026-11-07"
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
    - "../plans/native-desktop-initial-slices.md"
  implemented-by:
    - "../impl/ir/native-desktop-initial-slices.md"
  depends-on:
    - "../plans/native-desktop-initial-slices.md"
    - "../test-specs/native-desktop-initial-slices.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "../specs/native-desktop-initial-slices.md"
    - "../designs/native-desktop-initial-slices.md"
    - "../test-specs/native-desktop-initial-slices.md"
  refines: []
  refined-by: []
  derives-from:
    - "../plans/native-desktop-initial-slices.md"
  derived-by: []
  verifies: []
  verified-by:
    - "../test-specs/native-desktop-initial-slices.md"
  references: []
  defers: []
  deferred-by: []
---

# Issue #166 Native 初期 slice AL-G1〜G3 実装タスク

## Goal

[plan](../plans/native-desktop-initial-slices.md) に従い、
`al-gui` に AL-G1〜G3 の先行実装を追加し reviewable PR を作る。
Issue #166 の Close や AL-G4/G5 正式受入は目的外。

## Checklist

- [x] discovery / spec / design / plan / test-spec を作成
- [x] `al-gui` 依存・feature 配線 + `lib.rs` 公開
- [x] AL-G1: `desktop.rs` + `bin/al-desktop.rs` + gpui mise lane
- [x] AL-G1: FakeDriver fixture(`tests/desktop.rs`)
- [x] AL-G2: token 文書 + `theme.rs` + `preferences.rs` + `tests/theme.rs`
- [x] AL-G3: `view/*` + `tests/view.rs`(実 dataset fixture)
- [x] `docs/impl/ir/native-desktop-initial-slices.md` 作成
- [x] doc index 再生成(`doc_maintenance.js apply` + impl/ir README)
- [x] 独立 review A/B → Validator → Judge、P0/P1 修正
- [x] `fmt`/`check`/`lint`/`test`/UI/gpui/headless lane 検証
- [ ] origin/main merge → union → 再生成 → 最終 verify
- [ ] 日本語 Conventional Commit → push → PR(Refs #166)

## Blockers / 条件

- RF #387 Component/View renderer: 実描画接続は後段統合条件。
- RF #389 実 desktop 入力 slot: 担当外・触れない。
- doc graph の pre-existing broken relations(al-math 参照)は
  本件 scope 外として coordinator へ報告済みの扱いとする。

## Out-of-scope

AL-G4 実 OS lifecycle、AL-G5 visual/numeric/CLI 正式受入、
Plugin Dock UX、新共通 GUI framework、旧 WebView 成果の削除。

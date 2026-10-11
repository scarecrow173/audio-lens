---
id: "PLAN-NvzYVuupuqGBJmuEuJJPjA"
type: "plan"
status: "approved"
title: "spectral_envelope の fft_size 入口 assert 実装"
created: "2026-10-11"
updated: "2026-10-11"
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
    - "docs/specs/spectral-envelope-fft-size.md"
  implemented-by:
    - "docs/tasks/al-math-envelope-fft-size-assert.md"
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/discovery/spectral-envelope-fft-size.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/specs/spectral-envelope-fft-size.md"
    - "docs/designs/spectral-envelope-fft-size.md"
    - "docs/adr/spectral-envelope-fft-size-assert.md"
  derived-by: []
  verifies: []
  verified-by:
    - "docs/test-specs/spectral-envelope-fft-size.md"
  references: []
  defers: []
  deferred-by: []
---

# spectral_envelope の fft_size 入口 assert 実装 Implementation Plan

## Goal

`crates/al-math/src/filter.rs` の `Envelope::with_simd` 先頭に
`assert!(self.fft_size.is_power_of_two(), ...)` を追加し、
`fft_size: 0`・非2冪の silent 置き換えを排除する。`max()` 底上げは
維持して doc に規則を明記し、`#[should_panic]` テストを追加する。

## Scope

単一タスクで完結する規模のため、スライス分割しない。

- In scope: `filter.rs` の assert 1行 + doc 1行 + モジュール内
  テスト数行の追加
- Out of scope(design の Out of Scope を継承): 底上げ撤廃、
  Result/エラー返却化、`bandpass` 系の検証、`bins` 契約変更、
  他クレートの変更

## Upstream Documents

- `docs/specs/spectral-envelope-fft-size.md`(approved): 受入基準 —
  非ゼロ2冪必須の assert、底上げ規則の doc 明記、panic テスト、
  既存テスト全合格
- `docs/adr/spectral-envelope-fft-size-assert.md`(accepted): 案A
  (入口 assert + 底上げ維持)の決定と不採用案
- `docs/discovery/spectral-envelope-fft-size.md`: 問題分析と
  既存呼出し監査の根拠資料

## Design Inputs

- `docs/designs/spectral-envelope-fft-size.md`(approved): 本計画の
  正本。assert の配置(`with_simd` 先頭、`bins == 0` 早期 return
  より前)、`is_power_of_two()` による非ゼロ検査兼用、doc 文言、
  テスト追加内容を規定

## File Map

| Path | Responsibility | Notes |
|------|----------------|-------|
| `crates/al-math/src/filter.rs` | `Envelope::with_simd` 先頭への assert 追加、`spectral_envelope` doc の底上げ規則明記、モジュール内テストに `#[should_panic]` 2件追加 | 変更の全量。この1ファイルのみ |

## Implementation Sequence

| Slice | 内容 | Depends on | Blocks |
|-------|------|------------|--------|
| T1 | `filter.rs` への assert + doc + テスト追加と検証 | — | — |

## Task Breakdown

### T1: `filter.rs` の fft_size 契約修正

1. `Envelope::with_simd` の先頭(`if self.bins == 0` の早期
   return より前)に
   `assert!(self.fft_size.is_power_of_two(), "fft_size must be a non-zero power of two, got {}", self.fft_size);`
   を追加する
2. `spectral_envelope` の doc コメントを
   「fft_size は 2 のべき乗(0 は不可)。fft_size が信号長より
   小さい場合は信号長まで拡大する。bins=0 は空結果を返す」に
   更新する
3. モジュール内テスト(`mod tests`)に `#[should_panic]` テストを
   2件追加する: `fft_size: 0`、および非2冪(例: 1000)。
   `Envelope` 経由または `spectral_envelope` 公開関数経由で呼び、
   panic することを確認する
4. 検証: `mise exec -- cargo test -p al-math` が合格する
   (新規 panic テスト2件 + 既存テスト全合格)
5. 検証: `mise exec -- cargo build --workspace` が成功する

## Dependencies and Risks

外部依存の追加はない。変更は `filter.rs` 1ファイルに閉じる。

| # | Risk | Likelihood | Impact | Mitigation |
|---|------|-----------|--------|------------|
| 1 | 底上げに依存する隠れた呼出しが assert で panic する | low | med | 既存呼出し(`SpectralEnvelope` 既定 1024、`cfg.stft_window` 既定 4096)は非ゼロ2冪と監査済み。`cargo test -p al-math` と workspace ビルドで検出可能 |
| 2 | `bins == 0` 経路で assert を通過し契約違反が隠れる | low | low | assert を早期 return より前に配置する設計で防止済み |

### Rollback Strategy

- 対象: `filter.rs` 1ファイルのコード・doc・テスト変更のみ
- Trigger: 検証失敗で原因切り分けに時間を要する場合
- Procedure: 変更コミットを `git revert` する
- Verification: revert 後に `mise exec -- cargo test -p al-math`
  が合格すること

## Verification

| Step | Verification command or criteria | Pass condition |
|------|----------------------------------|----------------|
| T1 | `mise exec -- cargo test -p al-math` | 新規 `#[should_panic]` 2件を含む全テスト合格 |
| T1 | `mise exec -- cargo build --workspace` | ビルド成功 |

plan レベルの検証は上表の通り。振る舞いレベルの契約(panic 条件・
底上げ・scalar/dispatch 一致)は `test-spec` 文書
(`docs/test-specs/spectral-envelope-fft-size.md`)に切り出し、
`relations.verified-by` で本 plan にリンク済み。

## Open Questions / Gaps

- なし。上流は spec(approved)・design(approved)・ADR(accepted)
  が揃っており、変更内容は design で確定済み

## Self-Review

- [ ] Every spec requirement maps to a task.
- [ ] No placeholders remain.
- [ ] File paths, relations, status values, and terminology are consistent.

## Implementation Handoff

- Recommended mode: inline implementation(単一ファイル・数行の
  変更であり委譲の意味が薄い)
- Independent work streams: なし(単一タスク)
- User approval required before dispatch: yes
- If approved: T1 を実施し、Verification の2コマンドで検証する
- If declined or unavailable: 同一内容を inline で実施する

## Review Handoff

- 前提: `max()` 底上げは維持する(厳格化は案Bとして不採用)。
  レビューで底上げ撤廃を求める場合は ADR の再審が必要であり、
  本 plan の範囲を超える
- 未解決依存なし
- レビュー参照先: `docs/designs/spectral-envelope-fft-size.md`
  (正本)、`docs/specs/spectral-envelope-fft-size.md`(受入基準)、
  `docs/adr/spectral-envelope-fft-size-assert.md`(決定根拠)

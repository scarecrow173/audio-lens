---
id: "PLAN-034AaBtyLhpTxk9Vb4eftR"
type: "plan"
status: "approved"
title: "ゴールデン fixture マトリクスの実装"
created: "2026-10-04"
updated: "2026-10-04"
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
    - "docs/specs/validation-strategy.md"
  implemented-by: []
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related: []
  refines: []
  refined-by: []
  derives-from:
    - "docs/specs/validation-strategy.md"
    - "docs/designs/golden-validation.md"
    - "docs/discovery/validation-fixture-gaps.md"
  derived-by: []
  verifies: []
  verified-by:
    - "docs/test-specs/golden-validation.md"
  references: []
  defers: []
  deferred-by: []
---

# ゴールデン fixture マトリクスの実装 Implementation Plan

## Goal

`al-testkit` 共有 fixture 基盤と、spec 列挙ケースを網羅する
回帰テスト行列を実装する。

## Scope

1 つの実装ストリーム(テスト基盤 + 既存機能のケース行列)。
新規機能(video/MP4/expensive 追加)の fixture は各 WS の
タスクが担い、本プランは既存機能の行列と基盤を担当する。

## Upstream Documents

- `docs/specs/validation-strategy.md`
- `docs/discovery/validation-fixture-gaps.md`

## Design Inputs

- `docs/designs/golden-validation.md`(approved)

## File Map

- `crates/al-testkit/Cargo.toml` / `src/lib.rs` - 補助クレート
- `crates/al-testkit/src/signal.rs` - 信号生成・変換
  (gain/EQ/delay/jitter/polarity/mix/SRC/clip/drop/repeat/stuck)
- `crates/al-testkit/src/media.rs` - WAV/AVI fixture、
  (MP4 fixture は WS-C の writer を利用)
- `crates/al-testkit/src/expect.rs` - 期待メトリクス検証ヘルパ
- `crates/al-locate/tests/golden.rs` - locate ケース行列
- `crates/al-analysis/tests/golden.rs` - reference ケース行列
- `crates/al-media/tests/golden.rs` - video/A-V・コンテナケース

## Implementation Sequence

- [ ] `al-testkit-crate`: 補助クレート + 信号/メディアビルダ
- [ ] `testkit-expect-helpers`: 期待メトリクス検証ヘルパ
- [ ] `golden-locate-matrix`: locate ケース行列
- [ ] `golden-reference-matrix`: reference ケース行列
- [ ] `golden-av-matrix`: video/A-V・コンテナケース行列

## Dependencies and Risks

- 浮動小数の安定性 → 期待値は許容誤差必須
- 既存テストとの重複 → 既存 fixture は移行ではなく
  testkit 側で正規化し、既存テストは維持

## Task Breakdown

- `al-testkit-crate` + `testkit-expect-helpers` →
  `golden-locate-matrix` / `golden-reference-matrix` /
  `golden-av-matrix`(並列可)

## Verification

- [ ] `cargo test --workspace`
- [ ] spec 列挙ケースがテストとして存在する
- [ ] 既知パラメータが許容誤差内で回復される

## Open Questions / Gaps

- spec 要求ケースのうち新規機能依存のもの(video フレーム特徴・
  MP4)は各 WS で追加する。本行列は既存機能のみで閉じる

## Self-Review

- [x] Every spec requirement maps to a task.
- [x] No placeholders remain.
- [x] File paths, relations, status values, and terminology are consistent.

## Implementation Handoff

- Recommended mode: subagent-capable implementation
- Independent work streams: testkit 基盤後に 3 行列は並列化可能
- User approval required before dispatch: yes
- If approved: discover available implementation and delegation capabilities in the current environment, then execute task-by-task with review checkpoints.
- If declined or unavailable: execute inline using the same dependency graph and verification matrix.

## Review Handoff

- al-testkit を dev-dependency 専用とする構成、
  期待値をコード内に保持する方針をレビュー対象とする

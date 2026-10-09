---
id: "PLAN-034AaBrYBgbeSTDlZzbaGl"
type: "plan"
status: "completed"
title: "Expensive 追加解析ファミリの実装"
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
    - "docs/specs/expensive-analysis.md"
  implemented-by: []
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related: []
  refines: []
  refined-by: []
  derives-from:
    - "docs/specs/expensive-analysis.md"
    - "docs/designs/expensive-families.md"
    - "docs/discovery/expensive-family-gaps.md"
  derived-by: []
  verifies: []
  verified-by:
    - "docs/test-specs/expensive-families.md"
  references: []
  defers: []
  deferred-by: []
---

# Expensive 追加解析ファミリの実装 Implementation Plan

## Goal

spec の未実装 expensive ファミリを、既存契約を保ったまま
`al-analysis` に追加する。

## Scope

1 つの実装ストリーム。`expensive.rs` のサブモジュール分割
(振る舞い不変)と新ファミリの追加を含む。

## Upstream Documents

- `docs/specs/expensive-analysis.md`
- `docs/discovery/expensive-family-gaps.md`

## Design Inputs

- `docs/designs/expensive-families.md`(approved)
- `docs/designs/analysis-stages.md`(approved)

## File Map

- `crates/al-analysis/src/expensive.rs` - config/trigger/
  plan_regions/analyze_region オーケストレーション(分割後も保持)
- `crates/al-analysis/src/expensive/transfer.rs` - 精密伝達 /
  カットオフ推定(既存ファミリの退避先)
- `crates/al-analysis/src/expensive/ir.rs` - IR 候補・early/late・
  decay
- `crates/al-analysis/src/expensive/codec.rs` - pre-echo・
  transient smear
- `crates/al-analysis/src/expensive/dynamics.rs` - transfer-curve・
  pumping/ducking・level-dependent transfer
- `crates/al-analysis/src/expensive/noise.rs` - stationary/tonal/
  modulated 分類
- `crates/al-analysis/src/expensive/channel.rs` - 周波数依存混合行列
- `crates/al-analysis/src/expensive/timing.rs` - click/splice・
  stuck buffer 深掘り
- `crates/al-analysis/tests/expensive.rs` - 既知ケース検証

## Implementation Sequence

- [ ] `expensive-module-split`: 振る舞い不変のサブモジュール分割
- [ ] `exp-ir-decay`: IR/early-late/decay
- [ ] `exp-dynamics-curve`: transfer-curve + pumping + level依存
- [ ] `exp-codec-preecho`: pre-echo + transient smear
- [ ] `exp-noise-decomp`: stationary/tonal/modulated
- [ ] `exp-channel-matrix`: 周波数依存混合行列 + intermod 証拠
- [ ] `exp-timing-splice`: click/splice + stuck buffer
- [ ] `expensive-families-tests`: ファミリ別既知ケース検証

## Dependencies and Risks

- deconvolution/混合推定の数値安定性 → 条件数記録と
  support 不足時の invalid 化で防御
- 分割で既存テストが壊れる可能性 → split は単独タスクで先に
  行い全テストを通してからファミリ追加

## Task Breakdown

- `expensive-module-split` → 各ファミリタスク(互いに独立、
  並列可) → `expensive-families-tests`

## Verification

- [ ] `cargo test -p al-analysis`
- [ ] 各ファミリの既知合成ケースで期待値が記録される
- [ ] 分割後も既存テストがすべて通る

## Open Questions / Gaps

- intermodulation のトーン選択ポリシーは実装時に
  最も単純な規則(上位ピークのみ)で開始する

## Self-Review

- [x] Every spec requirement maps to a task **or** is explicitly
  deferred in `docs/discovery/expensive-family-gaps.md`
  (精密スペクトル自己相関の独立指標化・エイリアス/折り返し独立
  記述子・ナイキスト領域構造・ドロップアウト/ギャップ形状精緻化)。
- [x] No placeholders remain.
- [x] File paths, relations, status values, and terminology are consistent.

## Implementation Handoff

- Recommended mode: subagent-capable implementation
- Independent work streams: `expensive-module-split` 完了後、
  ファミリタスクは互いに独立で並列化可能
- User approval required before dispatch: yes
- If approved: discover available implementation and delegation capabilities in the current environment, then execute task-by-task with review checkpoints.
- If declined or unavailable: execute inline using the same dependency graph and verification matrix.

## Review Handoff

- サブモジュール分割の方法(expensive.rs + expensive/ 併存)と
  各ファミリの観測定義が既存 push 規約に整合するかを
  レビュー対象とする

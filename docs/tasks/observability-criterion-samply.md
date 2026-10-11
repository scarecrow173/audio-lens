---
id: "TASK-034EevkeIeBV0D2XgS6uTw"
type: "task"
status: "todo"
title: "criterion smoke bench と samply 手順書"
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
    - "docs/plans/observability-foundation.md"
  implemented-by: []
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related: []
  refines: []
  refined-by: []
  derives-from:
    - "docs/designs/observability-foundation.md"
    - "docs/specs/observability-foundation.md"
  derived-by: []
  verifies: []
  verified-by:
    - "docs/test-specs/observability-foundation.md"
  references: []
  defers: []
  deferred-by: []
---

# criterion smoke bench と samply 手順書

## Work

L1 計測(criterion)と L3 計測(samply 手順書)を整備する
(plan T5)。他タスクと共有ファイルを持たず独立に実施可能。

- `crates/al-math` に criterion を dev-dependency として追加
  する。add 時点で crates.io 公開から ≥7 日経過した版を pin
  し、採用版と公開日を Implementation Record に記録する
- `[[bench]]` に `harness = false` を設定し、
  `crates/al-math/benches/dsp_smoke.rs` を新設する。内容は既存
  公開カーネル(例: `al_math::filter::spectral_envelope` または
  `al_math::correlation::ncc`)を既存 dispatch 経路で測る最小
  criterion bench 1件 — smoke 目的のみで、vec プリミティブ
  本格ベンチは `al-math-simd-reduction` チェーンに委ねる
- `docs/guides/` を新設し `profiling-samply.md` を作成する。
  構成: (1) install 方法、(2) `[profile.release] debug =
  true` 設定、(3) 実行例、(4) `.samply` 出力の読み方。
  CPU sampling の外部ツールとして随時使う位置付けで、計装は
  不要であることを明記する

## Classification

- `normal-plan-task`

## Upstream Documents

- `docs/plans/observability-foundation.md`
- `docs/designs/observability-foundation.md`
- `docs/specs/observability-foundation.md`(criterion/samply
  の In Scope 要件)

## Dependencies

- なし(独立。al-math には tracing 依存を追加しない)

## Affected Files

- `crates/al-math/Cargo.toml`: criterion dev-dep(pin)+
  `[[bench]] harness = false`
- `crates/al-math/benches/dsp_smoke.rs`(新設): 最小 criterion
  bench
- `docs/guides/profiling-samply.md`(新設): samply 手順書

## Done When

- [ ] `mise exec -- cargo bench -p al-math` で dsp_smoke ベンチ
  が実行できる
- [ ] 採用した criterion の版と crates.io 公開日が
  Implementation Record に記録されている
- [ ] `docs/guides/profiling-samply.md` が存在し、install・
  release-debug 設定・実行例・出力の読み方を含む

## Verification

- [ ] `mise exec -- cargo bench -p al-math` が成功する
- [ ] `mise exec -- cargo test -p al-math` が green
- [ ] `mise exec -- cargo fmt --check` が green

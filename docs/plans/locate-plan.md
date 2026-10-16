---
id: "PLAN-034GMK73n1nSCbsGEfC6Np"
type: "plan"
status: "completed"
title: "locate 相関 plan ワーカープール導入"
created: "2026-10-16"
updated: "2026-10-16"
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
    - "docs/specs/locate-plan-cache.md"
  implemented-by: []
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related: []
  refines: []
  refined-by: []
  derives-from:
    - "docs/specs/locate-plan-cache.md"
    - "docs/designs/locate-plan.md"
  derived-by: []
  verifies: []
  verified-by:
    - "docs/test-specs/locate-plan-pool-inputcache-coarse-to-fine.md"
  references: []
  defers: []
  deferred-by: []
---

# locate 相関 plan ワーカープール導入 Implementation Plan

## Goal

`run_pool` にワーカーコンテキストを追加し、locate の全相関経路を
形状キー `CorrelationPool` 経由に移行して、plan 構築・workspace
確保・curve コピーを amortize する。あわせて `InputCache` 計数と
coarse-to-fine 境界テストを追加する。

## Scope

1 実装ストリーム。境界: al-locate crate 内の呼出配線と計測口のみ。
`al-math` API・検出アルゴリズム・合議は不変。

## Upstream Documents

- `docs/specs/locate-plan-cache.md`
- `docs/adr/locate-plan-run-pool.md`

## Design Inputs

- `docs/designs/locate-plan.md`

## File Map

- `crates/al-locate/src/pool.rs`(新設) — `CorrelationPool`:
  `HashMap<(usize,usize), Entry>`、`ncc`/`gcc_phat` 借用 API、
  `MAX_SHAPES` bound
- `crates/al-locate/src/lib.rs` — `pub(crate) mod pool` 登録
- `crates/al-locate/src/stage.rs` — `run_pool` 拡張(make_ctx)、
  `run_method_tasks`・`window_anchors_in`・`best_window_match` 移行
- `crates/al-locate/src/methods.rs` — `LocateMethod` シグネチャに
  `pool: &mut CorrelationPool`、`propose_coarse`/
  `propose_coarse_to_fine`/`refine_coarse` 配線
- `crates/al-locate/src/prepared.rs` — `InputCache` hits/misses
  カウンタ、`CacheStats`、`stats()`/`len()`
- `crates/al-locate/tests/prepared.rs` — 呼出更新 + 監査/境界テスト
- `crates/al-locate/tests/` その他の `locate`/`locate_prepared`/
  `propose_coarse_to_fine` 呼出の追従

## Implementation Sequence

- [ ] `pool.rs`: `CorrelationPool` + 計数フック(テスト用に構築
      回数を観測できる `entry_count()`/`stats()`)
- [ ] `prepared.rs`: `CacheStats`・`stats()`・`len()`・hit/miss 計数
- [ ] `stage.rs`: `run_pool` に `make_ctx`/`&mut C` を追加
      (ctx 不要呼出は `|| ()`)
- [ ] `methods.rs`: トレイトと各メソッドを pool 引数に移行、
      `dsp::ncc`/`dsp::gcc_phat` 呼出を pool 経由に置換
- [ ] `stage.rs`: `best_window_match`/`window_anchors_in`/
      `run_method_tasks` を pool 経由に移行
- [ ] テスト追従 + 新規テスト(P1/P2/P3/C1/C2/F1/F3)
- [ ] 検証: `cargo test -p al-locate` → workspace → clippy/fmt
- [ ] 計測: 小規模 fixture で locate 実行し plan 構築回数・
      InputCache hit/miss・時間を記録

## Dependencies and Risks

- `LocateMethod` シグネチャ変更でテスト・呼出点が波及(一覧は
  `grep locate_prepared\|\.locate(`)
- pool 借用の直列化で `BandNcc` の band ループは iteration ごとに
  借用を解放する必要あり(現行コード形状で成立)
- `window_anchors_in` の可変 shape は hit 率低下の可能性。
  bound 規約で現状以下にはならないことを確認

## Task Breakdown

- T1: `CorrelationPool` 新設 + 単体テスト(P1/P2/P3)
- T2: `InputCache` 計数 + `len()`/`stats()`(C1/C2)
- T3: `run_pool` ctx 拡張 + `best_window_match`/`window_anchors_in` 移行
- T4: `LocateMethod` + メソッド/`propose_coarse` 移行 +
      `run_method_tasks` 配線(呼出点・テスト追従)
- T5: coarse-to-fine 境界テスト(F1/F3)+ 監査テスト
- T6: 全検証 + 計測記録(IR)

依存: T1→T3/T4、T2 は独立、T5 は T4 後、T6 が最後。

## Verification

- [ ] `cargo test -p al-locate` pass
- [ ] `cargo test --workspace` 全 pass
- [ ] `cargo clippy --workspace --all-targets -D warnings` clean
- [ ] `cargo fmt --all -- --check` clean
- [ ] plan 構築回数の実測記録(同一 shape 2 回目以降で 0)

## Open Questions / Gaps

- `MAX_SHAPES` 初期値 8 の妥当性: 実測 RSS で確認し過大なら下げる
- `propose_coarse_to_fine` の外部呼出(tests)は pool を都度生成
  する形で追従

## Self-Review

- [x] Every spec requirement maps to a task.
- [x] No placeholders remain.
- [x] File paths, relations, status values, and terminology are consistent.

## Implementation Handoff

- Recommended mode: inline execution
- Independent work streams: none
- User approval required before dispatch: yes
- If approved: discover available implementation and delegation capabilities in the current environment, then execute task-by-task with review checkpoints.
- If declined or unavailable: execute inline using the same dependency graph and verification matrix.

## Review Handoff

- トレイトシグネチャ変更の波及範囲と pool 借用設計が reviewer の
  焦点。数値経路は一切変更しない

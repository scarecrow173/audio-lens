---
id: "PLAN-034P4YN0NpG6X9OObVjUYA"
type: "plan"
status: "approved"
title: "pre-migration baseline の実装計画"
created: "2026-10-30"
updated: "2026-10-31"
owners:
  - "scarecrow173"
relations:
  source:
    - "../adr/pre-migration-baseline.md"
    - "../specs/pre-migration-baseline-golden.md"
  changes:
    added: []
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements: []
  implemented-by: []
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "../discovery/pre-migration-baseline-issue-98.md"
  refines: []
  refined-by: []
  derives-from:
    - "../designs/pre-migration-baseline.md"
  derived-by: []
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# pre-migration baseline の実装計画 Implementation Plan

## Goal

`docs/designs/pre-migration-baseline.md` に基づき、
`tools/baseline/`(Node.js capture/normalize/store/compare)
と `baseline/`(commit 済み成果物)を実装し、run matrix の
実測を記録して Issue #98 の baseline を固定する。

## Scope

単一の実装ストリーム。境界: 測定ツール・成果物・最小 guard
テスト・ドキュメント。product コードの解析ロジック変更や
ruviform 側への変更は含まない。

## Upstream Documents

- `docs/specs/pre-migration-baseline-golden.md`
- `docs/adr/pre-migration-baseline.md`

## Design Inputs

- `docs/designs/pre-migration-baseline.md`

## File Map

- `tools/baseline/capture.mjs` — run matrix 実行・収集・記録
  オーケストレーション
- `tools/baseline/lib/normalize.mjs` — run グローバル ID
  正規名 mapping・絶対パス placeholder・record-only 判定
- `tools/baseline/lib/store.mjs` — al-dataset columnar
  table/chunked tensor/wav の読み取り
- `tools/baseline/lib/procmon.mjs` — 子プロセス CPU/RSS
  ポーリング(Windows Get-Process / POSIX /proc)
- `tools/baseline/compare.mjs` — contracts.json に従う
  golden/index 比較 + `--self-test` 摂動検証
- `tools/baseline/README.md` — 使い方
- `crates/al-analysis/examples/bench_alloc.rs` — counting
  allocator による代表処理の alloc 計測
- `mise.toml` — `baseline` task 追加
- `baseline/` — environment.json / inputs.json / config.json /
  coverage.json / contracts.json / runs/*/ / bench/ / README.md
- `crates/al-*/tests/`(必要分のみ)— coverage 台帳で判定した
  最小 guard テスト

## Implementation Sequence

1. `tools/baseline/lib/store.mjs` + `lib/normalize.mjs` を
   実装し、既存 `JOB-OPNZOQvl2fj706ZclHk8K8` の成果物で
   decode・正規化を単体確認する
2. `compare.mjs` を実装し、`--self-test` で摂動検出を確認
3. `capture.mjs` + `lib/procmon.mjs` を実装(`canonical` を
   まずドライランで 1 run 通す)
4. `bench_alloc.rs` example を追加し動作確認
5. `baseline/config.json`・`contracts.json`・`coverage.json`
   の骨格を作成(coverage は既存テスト棚卸し結果を転記)
6. `mise run baseline` で run matrix 全 run を実測し
   `baseline/` を生成
7. coverage 台帳で「追加 test」に分類した最小 guard を
   `crates/al-*/tests/` に追加
8. `baseline/README.md`・environment/inputs/bench を記録し
   `mise run test`/`lint`/`fmt` を通す

## Dependencies and Risks

- run matrix 全体で約 40 分・生成物 ~20 GB。disk 空きを
  事前に確認する(測定後に `_runs` を削除すれば baseline
  本体は小さい)
- ffmpeg は外部(global mise 9.0.2)。capture は ffmpeg を
  探索し、版数を environment.json に記録する
- `resume` run は stage 境界 kill 依存。kill タイミング race
  で半書き成果物が残った場合は `verify_completed_artifacts`
  の demote 経路で自然に再実行される想定だが、失敗したら
  base-dir を作り直して再試行する
- al-dataset の store レイアウトに store.mjs が依存。変更時は
  追従が必要(deferred concern として記録済み)

## Task Breakdown

- T-1: store.mjs + normalize.mjs 実装・単体確認
- T-2: compare.mjs 実装 + self-test
- T-3: capture.mjs + procmon.mjs 実装・canonical ドライラン
- T-4: bench_alloc.rs example + bench 転記
- T-5: config/contracts/coverage 骨格 + run matrix 実測
- T-6: 最小 guard テスト追加 + baseline README 整備
- T-7: 全体検証(mise test/lint/fmt)+ commit

## Verification

- [ ] `mise run baseline` で run matrix の成果物が生成される
- [ ] `compare.mjs` で canonical vs aa が bit-exact 面で一致
  (または determinism.json に非決定面が記録される)
- [ ] `compare.mjs` で resume vs canonical の normalized
  golden が一致する
- [ ] `compare.mjs --self-test` が意図差分を報告する
- [ ] `baseline/coverage.json` が Issue #98 列挙を全て割当
- [ ] `mise run test` / `lint` / `fmt` が green

## Open Questions / Gaps

- 既存 bench(criterion)の記録面を `bench/results.json` に
  どこまで含めるか — サマリ値のみとし、詳細は
  `target/criterion` を参照する旨を記録予定
- variants で使う AVI の生成は ffmpeg 依存(入力 hash は
  記録のみ・契約外)

## Self-Review

- [x] Every spec requirement maps to a task.
- [x] No placeholders remain.
- [x] File paths, relations, status values, and terminology are consistent.

## Implementation Handoff

- Recommended mode: inline execution(単一ワーカー内で
  capture・実装・計測を行う)
- Independent work streams: なし(T-1..T-7 は直列依存)
- User approval required before dispatch: yes
- If approved: task graph 経由で順次実行
- If declined or unavailable: inline で同一順に実行

## Review Handoff

- ID 正規化の採番順と resume run の kill 境界は ADR/design
  で確定済み。実装で判明する細部(列挙すべき record-only
  field 等)は contracts.json に集約する

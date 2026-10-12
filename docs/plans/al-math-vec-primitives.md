---
id: "PLAN-034EpNVDehwMClwkJioA7x"
type: "plan"
status: "approved"
title: "al-math vec プリミティブ集約と dsp ファサード移行"
created: "2026-10-12"
updated: "2026-10-12"
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
    - "docs/specs/al-math-vec-primitives.md"
    - "docs/adr/al-math-vec-primitives-slice-boundary.md"
    - "docs/adr/simd-reduction-lane-fma.md"
  implemented-by:
    - "docs/tasks/al-math-vec-primitives.md"
    - "docs/tasks/al-locate-vec-migration.md"
    - "docs/tasks/al-analysis-vec-migration.md"
    - "docs/tasks/vec-primitives-verification-bench.md"
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related: []
  refines: []
  refined-by: []
  derives-from:
    - "docs/designs/al-math-vec-primitives.md"
  derived-by: []
  verifies: []
  verified-by:
    - "docs/test-specs/al-math-vec-primitives.md"
  references: []
  defers: []
  deferred-by: []
---

# al-math vec プリミティブ集約と dsp ファサード移行 Implementation Plan

## Goal

approved design `docs/designs/al-math-vec-primitives.md`
(DESIGN-034Epg45jvGomamsJiywFK) に基づき、`al_math::vec`
モジュールに f64 ベクトル演算プリミティブ 8 種(`dot`/
`sum_sq`/`corr0`/`rms`/`scale`/`axpy`/`sub_scaled`/`mag_sq`)
を `Simd` ジェネリックカーネル + 入口 1 回 dispatch の 3 層で
集約し、`al_locate::dsp` ファサードの re-export を唯一の消費
経路として、al-locate・al-analysis に散在する重複実装を置換
する。等価性検証(scalar ビット一致 + dispatch↔Scalar 許容
誤差)・criterion ベンチ・residual 再構成の採否計測までを
含む。

## Scope

単一の実装ストリームとして扱い、T1–T4 の 4 タスクに分割する。
`al_math::vec` の公開面(T1)→ dsp re-export(T2)→ dsp 経由
消費(T3)の依存方向が直列で、等価性検証と計測(T4)は移行
完了後のコードを対象とする。

- In scope: `crates/al-math/src/vec.rs` 新設と `linalg.rs` の
  共有カーネル化、`dsp.rs` re-export + crate root mirror、
  design file map の al-locate/al-analysis 重複置換、残存許容
  リストの遵守、dispatch↔Scalar 等価テスト、
  `benches/vec_primitives.rs`、examples `bench_*` による置換
  前後比、`residual.rs` 条件付き再構成の計測・採否記録
- Out of scope(spec/design の Out of Scope・Deferred を継承):
  FFT・相関カーネルのアルゴリズム変更(issue #42)、
  `al-viz` `lod.rs` incremental `sum_sq`、lag 走査の
  プリミティブ化・ncc 置換、ベクトル/行列抽象型、al-analysis
  への al-math 依存追加、pulp 型の公開面露出、golden 判定意味・
  Validity 閾値の変更、criterion CI 回帰ゲート、小ベクトル
  per-call dispatch コスト対策

## Upstream Documents

- `docs/specs/al-math-vec-primitives.md`(approved): 受入基準 —
  公開プリミティブ集合、dsp ファサード経由・al-analysis 非依存、
  重複置換の列挙と残存許容、panic/値/等価性契約、bench と
  residual 判定の記録義務
- `docs/adr/al-math-vec-primitives-slice-boundary.md`(accepted):
  公開境界・消費経路・等長 panic 契約・値の契約・linalg 共有の
  権威
- `docs/adr/simd-reduction-lane-fma.md`(accepted): reduction
  順序(lane acc + FMA)・`mul_add_e_f64s` 規則・等価性
  predicate・サイズ行列の権威
- `docs/discovery/al-math-simd-reduction.md`: 重複実装の棚卸し表
  (file:line の根拠)

## Design Inputs

- `docs/designs/al-math-vec-primitives.md`(approved): 本計画の
  正本。vec.rs の 3 層構成(L111-141)、公開シグネチャと等長
  panic・値の契約(L142-166)、プリミティブ別カーネル表と
  `mul_add_e_f64s` 規則(L167-191)、linalg 共有(L192-208)、
  dsp re-export + root mirror(L210-222)、al-locate/al-analysis
  移行表(L224-291)、エイリアス規約(L286-291)、residual
  条件付き再構成の計測プロトコル(L293-327)、ベンチ計画
  (L328-349)、テスト計画(L351-381)

## File Map

| Path | Responsibility | Task |
|------|----------------|------|
| `crates/al-math/src/vec.rs`(新設) | `check_same_len` + 8 プリミティブの `pub(crate) *_kernel<S: Simd>` + `WithSimd` アダプタ + 公開 wrapper(入口 1 回 dispatch)。`#[cfg(test)]` に契約・panic・scalar ビット一致テスト | T1 |
| `crates/al-math/src/lib.rs` | `pub mod vec;` 追加 | T1 |
| `crates/al-math/src/linalg.rs` | 内部 `dot` を `vec::dot_kernel` に統一、`reflect` を `dot_kernel`+`sub_scaled_kernel` 消費へ | T1 |
| `crates/al-locate/src/dsp.rs` | `pub use al_math::vec::{axpy, corr0, dot, mag_sq, rms, scale, sub_scaled, sum_sq};` 追加 | T2 |
| `crates/al-locate/src/lib.rs` | root mirror(`pub use dsp::{...}` :22)に vec シンボル追加 | T2 |
| `crates/al-locate/src/methods.rs` | `energy`(:142)削除 + 呼出し(:175,:183,:363,:408,:501)を `dsp::sum_sq` へ、`normalize_gain`(:500-506)を `rms`+`scale`。envelope NCC(:539-547)は **未移行 — 残存許容へ逸脱**(risk 8・IMPL-034EqrFgTWTPu16W3CuG6r 参照) | T2 |
| `crates/al-locate/src/mixer.rs` | インライン `rms`(:74)を `dsp::rms` へ | T2 |
| `crates/al-locate/src/residual.rs` | マルチch `rms`(:133)を per-channel `sum_sq` 再合成へ | T2 |
| `crates/al-analysis/src/cheap.rs` | `rms`×4・mean-square・`sum_sq`×2・inline corr0×3(pair_correlation は sx/sy 保持の `corr0`+個別 `sum_sq`) | T3 |
| `crates/al-analysis/src/reference.rs` | `rms`(:119)・`corr0`(:134)・`ls_gain`(:127-131、ゼロ分母ガード保持)・マルチch energy(:1066)・`res_rms`(:1164) | T3 |
| `crates/al-analysis/src/expensive/mod.rs` | `pub(crate) rms`(:257)・`corr0`(:264)削除、消費側を dsp import へ | T3 |
| `crates/al-analysis/src/expensive/timing.rs` | `use super::{corr0, rms}`(:11)→ `use al_locate::dsp::{...}`、呼出し 6 箇所追従 | T3 |
| `crates/al-analysis/src/expensive/dynamics.rs` | rms import + 呼出し(:86,:149,:350,:351)追従 | T3 |
| `crates/al-analysis/src/expensive/ir.rs` | `mag2` 合成(:178)→ `dsp::mag_sq`(タプルスライス :155-161 は残存許容) | T3 |
| `crates/al-analysis/src/band_gain.rs` | `sum_sq`(:94,:101)・window dot(:96) | T3 |
| `crates/al-analysis/src/stft.rs` | `power`/`psd`/`magnitude`(:47-84)を `mag_sq`(+`scale`/要素毎 sqrt)へ | T3 |
| `crates/al-math/src/vec.rs` tests | dispatch↔Scalar 等価テスト(0..=33・1k・64k・~1M) | T4 |
| `crates/al-math/benches/vec_primitives.rs` + `Cargo.toml` | criterion `vec` group・`[[bench]] harness = false` | T4 |
| `crates/al-locate/src/residual.rs` | 採用時のみ減算ループ(:163-169)を per-source `sub_scaled` 再構成(列 gather dot :166 も解消) | T4 |

## Implementation Sequence

| Task | 内容 | Depends on | Blocks |
|------|------|------------|--------|
| T1 | `al_math::vec` モジュール(カーネル + dispatch wrapper + linalg 共有 + 契約/ビット一致テスト) | — | T2 |
| T2 | dsp re-export + root mirror + al-locate 移行(residual 条件付き再構成を除く) | T1 | T3 |
| T3 | al-analysis 移行(dsp ファサード経由、ガード付き合成保持) | T2 | T4 |
| T4 | dispatch↔Scalar 等価テスト + criterion ベンチ + 置換前後比 + residual 再構成計測・採否 | T3 | — |

T2 は `al_math::vec` の公開面(T1)に、T3 は dsp re-export
(T2)に、T4 は移行完了後コード(T3)に依存する直列チェーン。
T1 の vec.rs テストと T4 の等価性テストは同一 `#[cfg(test)]`
モジュールを共有するが、~1M を含む経路間検証を計測タスクに
集約するため分割した。

## Dependencies and Risks

新規依存なし(criterion は observability foundation で al-math
dev-dep として導入済み)。

| # | Risk | Likelihood | Impact | Mitigation |
|---|------|-----------|--------|------------|
| 1 | residual 減算ループ再構成が strided gather→連続 stream 化でも回帰する | med | low | T4 の計測プロトコル(正しさゲート + bench_locate_prepared ±5%)で採否を判定。見送りは ADR で許容済み・列 gather dot は残存許容に留まる |
| 2 | float `assert_eq!` 厳密比較テストの棚卸し漏れで ~ulp 差に落ちる | med | low | planning 時 sweep 済み(下記)。候補は `crates/al-locate/tests/locate.rs:327`(`assert_eq!(rms, expected.energy.after)` — 永続化値との経路またぎ厳密比較)。T2/T3 の Done When で置換対象に関わる比較の判定・修正を義務化し、修正一覧を IR に記録 |
| 3 | `pair_correlation` で sx/sy を落とし Validity(`SilenceInsufficientEnergy`)・`ild_db` が壊れる | low | high | design が `corr0` + 個別 `sum_sq` で保持する形を規定。T3 の Done When に保持を明記、Validity 系既存テストで回帰検出 |
| 4 | `ls_gain` でゼロ分母ガードを落とし NaN が混入 | low | med | design の置換形 `if den > 0.0 { dot/den } else { 0.0 }` をそのまま適用。T3 Done When で検査 |
| 5 | `mul_add_f64s` の誤用で Scalar 経路も融合しビット一致契約が破れる | low | med | カーネルは `mul_add_e_f64s` のみ。scalar ビット一致テスト(T1)で即座に検出される |
| 6 | dispatch 経路の最終ビットが ISA 依存で永続化値(ResidualEnergy 等)が ~ulp 揺れ | — | low | ADR で受容済み(Validity 閾値との桁差 6〜12 桁を確認済み)。golden は位置比較で float 厳密比較なし |
| 7 | 小スライス呼出しの per-call dispatch オーバーヘッド回帰 | low | low | ポリシー上許容(Deferred Design Concerns)。bench サイズ 32 で観測し、問題なら別施策 |
| 8 | 合議の argmax が FP ノイズの票分散に暗黙依存する構造問題: envelope NCC に `corr0`(および `dot`/`sum_sq` 再合成形)を適用すると、周期入力で全 lag 厳密 1.0 となり stable-sort argmax が lag 0 に倒れ gcc_phat と 2 手法合意 → golden `pinned_periodic_everywhere_not_accepted_at_wrong_phase` を破壊(T2 で確定)。method 内曖昧性(全 lag で peak_ratio≈1.0)は現合議機構では検出されない | —(T2 で確定) | med | 当サイトは逐次式を維持し残存許容へ逸脱(IMPL-034EqrFgTWTPu16W3CuG6r に実験経緯を記録)。将来このサイトを `corr0` へ移行するには決定的タイブレーク、または method 内曖昧性検出(peak_ratio≈1.0 の全 lag 同点検出)が前提となる — 発見された構造課題として別 issue に切り出し、本チェーンの scope 外とする |

planning 時の厳密比較棚卸し(al-locate/al-analysis/tests を
`assert_eq!` で sweep): 経路間比較に該当する確定的な候補は
`crates/al-locate/tests/locate.rs:327` のみ(同 :328 は既に
`1e-15` 許容比較)。`methods.rs:637` や `band_gain.rs:449-461`
の float 比較はテストが構築したリテラル入力の pass-through で
あり置換対象外。`expensive/tests.rs`・`tests/expensive.rs` の
`val(record(...))` 厳密比較は整数値・符号値が主で、T3 実施時に
置換で生成される feature 値への該当有無を再確認する。

### Rollback Strategy

- 対象: 全タスクがコード・文書の追加・置換のみで不可逆な
  データ変換はない。永続化値の ~ulp 揺れは ADR 受容済みの
  許容差であり形式互換を壊さない
- Trigger: 検証マトリクスの失敗(既存 test 非回帰・golden/
  Validity 破壊・等価 predicate 逸脱)で原因切り分けに時間を
  要する場合
- Procedure: 当該タスクのコミットを `git revert` する。直列
  チェーンのため後続タスク未着手なら前タスクの revert で完結
  する(T3 未着手なら T2 revert で al-locate 内完結、等)。
  residual 再構成のみなら T4 内の該当差分だけ戻せる
- Verification: revert 後に `mise exec -- cargo test
  --workspace` が合格すること

## Task Breakdown

### T1: `al_math::vec` モジュール — `docs/tasks/al-math-vec-primitives.md`

`vec.rs` 新設(`check_same_len` + 8 `*_kernel<S: Simd>` +
`WithSimd` アダプタ + dispatch wrapper)、`lib.rs` の
`pub mod vec;`、`linalg.rs` 内部 `dot` 共有化と `reflect` の
`dot_kernel`+`sub_scaled_kernel` 化、`#[cfg(test)]` に契約・
`#[should_panic]` 等長・scalar ビット一致テスト。

### T2: dsp re-export + al-locate 移行 — `docs/tasks/al-locate-vec-migration.md`

`dsp.rs` re-export、crate root mirror、`methods.rs`/`mixer.rs`/
`residual.rs`(マルチch rms のみ)の置換。residual 減算ループ
の条件付き再構成は含まない。

### T3: al-analysis 移行 — `docs/tasks/al-analysis-vec-migration.md`

`cheap.rs`・`reference.rs`・`expensive/mod.rs`(+ timing.rs・
dynamics.rs・ir.rs)・`band_gain.rs`・`stft.rs` を dsp
ファサード経由へ。`ls_gain` ガード・`pair_correlation` sx/sy
保持・al-math 依存非追加・残存許容の遵守。

### T4: 等価性検証・ベンチ・residual 判定 — `docs/tasks/vec-primitives-verification-bench.md`

dispatch↔Scalar 等価テスト(サイズ行列)、
`benches/vec_primitives.rs`、examples `bench_*` による置換
前後比、residual 減算ループ再構成の計測プロトコルと採否記録。

## Verification

| spec AC(要旨) | 検証方法 | 担当 |
|---|---|---|
| 8 プリミティブ公開・pulp 型非露出 | `cargo test -p al-math` + 公開シグネチャのコード検査 | T1 |
| dsp re-export・al-analysis に al-math 直接依存なし | `mise exec -- cargo tree -p al-analysis --depth 1` 出力検査 / Cargo.toml 確認 | T2, T3 |
| 列挙した重複の置換・残存許容以外の独立実装なし | file map のサイト検査 + rg 走査 | T2, T3 |
| 等長 panic の `#[should_panic]` テスト | `cargo test -p al-math`(2 入力全種 + `mag_sq` 3 スライス) | T1 |
| `corr0` ゼロ/NaN → 0.0・`rms` 空 → 0.0 | `cargo test -p al-math` 契約テスト | T1 |
| scalar ビット一致 + dispatch↔Scalar 許容誤差・サイズ行列・x86-v3/Scalar | `cargo test -p al-math`(T1: scalar 一致、T4: 等価テスト) | T1, T4 |
| `cargo test -p al-math -p al-locate -p al-analysis`・`--workspace`・clippy・fmt green | 各タスク Verification + 最終確認 | 全体 |
| `cargo bench -p al-math` で vec ベンチ実行・実測値を IR 記録 | bench 実行 + Implementation Record | T4 |
| 代表経路の置換前後比(±5% 超回帰なし) | examples `bench_*` Instant 計測 | T4 |
| residual 再構成の採否と計測根拠の記録 | 計測プロトコル実施 + IR 記録 | T4 |
| 許容比較へ修正した float テストの一覧記録 | IR 記録(棚卸し結果は本計画に記載済み) | T2, T3 |

## Open Questions / Gaps

- residual 再構成の採否は T4 の計測結果次第(採用/見送りどちら
  も正規結果。見送り時は列 gather dot :166 が残存許容に残る)
- 置換前後比の「置換前」計測は移行前コミットで取る必要がある —
  T4 着手時点では T1–T3 適用済みなので、T4 は移行前ベース
  コミットでの計測手順(checkout/別 worktree 等)を含めるか、
  タスク着手前の main 相当で事前取得するかを実施時に決める
- `expensive/tests.rs` 等の feature 値厳密比較で置換生成値に
  該当するものが残るかは T3 実施時の最終確認事項(planning
  sweep では pass-through リテラル比較のみ確認)

## Self-Review

- [x] Every spec requirement maps to a task.
- [x] No placeholders remain.
- [x] File paths, relations, status values, and terminology are consistent.

## Implementation Handoff

- Recommended mode: inline execution(T1→T2→T3→T4 の直列依存)
- Independent work streams: なし(T4 の等価性テスト部分のみ
  T1 完了後に先行可能だが、同一ファイルのため直列扱いとした)
- User approval required before dispatch: yes
- If approved: discover available implementation and delegation capabilities in the current environment, then execute task-by-task with review checkpoints.
- If declined or unavailable: execute inline using the same dependency graph and verification matrix.

## Review Handoff

- design の file map・カーネル表・計測プロトコルを正本として
  転記済み。レビュアは Verification matrix が spec の
  Acceptance Criteria を網羅しているか、直列依存 T1→T2→T3→T4
  とガード付き合成(ls_gain・pair_correlation sx/sy)の
  タスク割当が妥当かを確認すること
- 厳密比較棚卸しの実施結果は Dependencies and Risks の risk 2
  に記録済み(`locate.rs:327` が唯一の確定的候補)

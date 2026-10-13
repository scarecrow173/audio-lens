---
id: "TASK-034EpPesSpbjbAQBsmjvJx"
type: "task"
status: "done"
title: "al-locate: dsp re-export と vec プリミティブ移行"
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
    - "docs/plans/al-math-vec-primitives.md"
  implemented-by:
    - "docs/impl/ir/al-locate-dsp-re-export-vec-migration.md"
  depends-on:
    - "docs/plans/al-math-vec-primitives.md"
    - "docs/tasks/al-math-vec-primitives.md"
  blocks:
    - "docs/tasks/al-analysis-vec-migration.md"
  supersedes: []
  superseded-by: []
  related:
    - "docs/discovery/locate-consensus-argmax-ambiguity-contract.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/designs/al-math-vec-primitives.md"
    - "docs/adr/al-math-vec-primitives-slice-boundary.md"
  derived-by: []
  verifies: []
  verified-by:
    - "docs/test-specs/al-math-vec-primitives.md"
  references: []
  defers: []
  deferred-by: []
---

# al-locate: dsp re-export と vec プリミティブ移行

## Work

`al_locate::dsp` に vec プリミティブの re-export を追加し、
al-locate 内の重複実装をプリミティブ呼出しに置き換える
(plan T2)。`residual.rs` の条件付き再構成(減算ループの
per-source `sub_scaled` 化・列 gather dot の解消)は本タスクの
対象外で、T4 の計測プロトコルで採否を決める。

- `crates/al-locate/src/dsp.rs`: 既存の
  `pub use al_math::correlation::{...}` パターンに倣い
  `pub use al_math::vec::{axpy, corr0, dot, mag_sq, rms, scale,
  sub_scaled, sum_sq};` を追加(design L210-222)
- `crates/al-locate/src/lib.rs`: 既存 mirror 規約の
  `pub use dsp::{bandpass, deinterleave, fft, gcc_phat, ncc,
  spectral_envelope};`(:22)に vec シンボルを同一行で追加する
  root mirror(正規消費経路は `dsp::` のまま。名前衝突なし)
- `crates/al-locate/src/methods.rs`:
  - `:142` の独自 `fn energy`(Σv²)を削除し、呼出し
    `:175,:183,:363,:408,:501` を `dsp::sum_sq` へ
  - `:500-506` `normalize_gain` → `dsp::rms(x)` +
    `dsp::scale(x, r.recip(), &mut out)`。`v / rms` から
    `v * r.recip()` への ulp 級差は再合成許容ポリシーで許容
    (Implementation Record に記録)
  - `:539-547` envelope NCC は**未移行 — 残存許容へ逸脱**。
    `corr0` 置換は周期入力で全 lag 厳密 1.0 となり argmax
    タイブレークが lag 0 に倒れ、golden
    `pinned_periodic_everywhere_not_accepted_at_wrong_phase`
    の曖昧ケース非受理契約を破る(IMPL-034EqrFgTWTPu16W3CuG6r)
- `crates/al-locate/src/mixer.rs`: `:74` のインライン `rms` →
  `dsp::rms(cap)`
- `crates/al-locate/src/residual.rs`: `:133` の `&[Vec<f64>]`
  マルチch `rms` を per-channel `dsp::sum_sq` 合成で再表現
  (design L240-252 のコードそのまま:
  `channels.iter().fold((0.0, 0usize), |(s, n), c| (s +
  dsp::sum_sq(c), n + c.len()))` → `n == 0 ? 0.0 :
  (sum / n as f64).sqrt()`)。結合順変更で永続化値
  `ResidualEnergy` は ~ulp 級に揺れ得る(ADR 許容)

### Out of scope(本タスク)

- `residual.rs:163-169` の転置アクセス形減算ループの per-source
  `sub_scaled` 再構成と列 gather dot(:166)の解消 — T4 の計測
  プロトコルで採否
- al-analysis 側の置換(T3)

## Classification

- `normal-plan-task`

## Upstream Documents

- `docs/plans/al-math-vec-primitives.md`
- `docs/designs/al-math-vec-primitives.md`(al-locate 移行表
  L224-252)
- `docs/adr/al-math-vec-primitives-slice-boundary.md`(dsp
  ファサード消費経路・root mirror 規約の権威)

## Dependencies

- `docs/tasks/al-math-vec-primitives.md`(T1): `al_math::vec` の
  公開プリミティブが前提

## Affected Files

- `crates/al-locate/src/dsp.rs`: vec re-export 追加
- `crates/al-locate/src/lib.rs`: root mirror に vec シンボル追加
- `crates/al-locate/src/methods.rs`: `energy` 削除 + 呼出し置換
- `crates/al-locate/src/mixer.rs`: インライン `rms` 置換
- `crates/al-locate/src/residual.rs`: マルチch `rms` 再合成
  (条件付き再構成は除く)

## Done When

- [ ] `al_locate::dsp::{axpy, corr0, dot, mag_sq, rms, scale,
  sub_scaled, sum_sq}` が re-export され、crate root mirror にも
  vec シンボルが追加されている
- [ ] 列挙した al-locate サイトがプリミティブ呼出しに置換され、
  残存許容リスト以外に同一式の独立実装が残らない
- [ ] `normalize_gain` の除算→逆数乗算差分が Implementation
  Record に記録される
- [ ] `crates/al-locate/tests/locate.rs:327` の
  `assert_eq!(rms, expected.energy.after)` が dispatch 経路差を
  またぐ厳密比較か判定され、該当する場合は許容比較
  (同ファイル既存の `1e-15` 形に準拠)へ修正して一覧を IR に
  記録する

## Verification

- [ ] `mise exec -- cargo test -p al-locate` が green
- [ ] `mise exec -- cargo build --workspace` が green
- [ ] `mise exec -- cargo clippy`・`cargo fmt --check` が green
- [ ] `rg "fn energy|iter\\(\\)\\.map\\(\\|v\\| v \\* v\\)"`
  等で al-locate 内に残存する同一式独立実装が残存許容以外に
  ないことを確認

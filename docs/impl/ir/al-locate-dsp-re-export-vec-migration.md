---
id: "IMPL-034EqrFgTWTPu16W3CuG6r"
type: "impl"
status: "completed"
title: "al-locate dsp re-export vec migration"
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
    - "docs/tasks/al-locate-vec-migration.md"
  implemented-by: []
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/discovery/locate-consensus-argmax-ambiguity-contract.md"
  refines: []
  refined-by: []
  derives-from: []
  derived-by: []
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
metadata:
  experiments:
    adopted: []
    rejected: []
---

# al-locate dsp re-export vec migration

## Summary

`al_locate::dsp` に `al_math::vec` の 8 プリミティブ
(`axpy`/`corr0`/`dot`/`mag_sq`/`rms`/`scale`/`sub_scaled`/`sum_sq`)の
re-export を追加し、crate root の既存 mirror 規約に倣って lib.rs にも
同一シンボルを追加した。file map の列挙箇所をプリミティブ呼出しへ
置換したが、`methods.rs` envelope NCC サイトのみ `corr0` 化が
pinned golden(強周期信号の曖昧性棄却)を決定的に破壊するため、
残存許容として逐次式を維持する逸脱を採った(詳細は下記)。

## Context

task `docs/tasks/al-locate-vec-migration.md`、design
`docs/designs/al-math-vec-primitives.md`(file map L224-252)、ADR
`docs/adr/al-math-vec-primitives-slice-boundary.md`(dsp ファサード
消費経路・root mirror 規約)に従う plan T2。

## Implementation

### Changes

- `crates/al-locate/src/dsp.rs`: `pub use al_math::vec::{axpy, corr0,
  dot, mag_sq, rms, scale, sub_scaled, sum_sq};` を既存の
  `al_math::correlation`/`al_math::filter` re-export の直下に追加
- `crates/al-locate/src/lib.rs`: `:22` の mirror 行を拡張し、
  `pub use dsp::{...}` に 8 シンボルを追加(名前衝突なし)
- `crates/al-locate/src/methods.rs`:
  - `fn energy`(旧:142)を削除し、呼出し 5 箇所
    (preflight の template/signal、band フィルタ 2 箇所、
    normalize_gain 経由)を `dsp::sum_sq` に置換。MIN_ENERGY との
    比較形は字面どおり保持
  - `normalize_gain` を `dsp::rms(x)` + `dsp::scale(x, r.recip(),
    &mut out)` に置換。除算→逆数乗算で ulp 級差が生じる
    (再合成許容ポリシーで許容済み)
  - envelope NCC(:539-547)は **置換せず逐次式を維持**(下記)
- `crates/al-locate/src/mixer.rs`: `:74` のインライン
  `(Σv²/len).sqrt()` を `dsp::rms(cap)` に置換
  (`use crate::dsp;` 追加)
- `crates/al-locate/src/residual.rs`: `:133` の `&[Vec<f64>]`
  マルチch `rms` を design のコードそのまま per-channel
  `dsp::sum_sq` fold 合成に置換(`use crate::dsp;` 追加)。
  `n == 0 → 0.0` ガードは保持。列 gather dot(:166)と転置
  アクセス減算ループ(:163-169)は T4 の計測プロトコル範囲の
  ため未着手
- `crates/al-locate/tests/locate.rs:327`: `assert_eq!(rms,
  expected.energy.after)` を `abs() < 1e-15` 許容比較へ緩和
  (永続化値との dispatch 経路またぎ厳密比較 — plan risk 2 で
  棚卸し済みの確定的候補。同 :328 既存の `1e-15` 形に準拠)

### Decisions During Implementation

- **envelope NCC の `corr0` 置換を残存許容へ逸脱**:
  `methods.rs` envelope NCC を `dsp::corr0` に置換すると、
  golden テスト
  `pinned_periodic_everywhere_not_accepted_at_wrong_phase` が
  `accepted: false`/`ambiguous_alignment` から `accepted: true` へ
  変わり失敗した。原因は、周期入力では全 visited lag の
  envelope がビット一致し `corr0` が厳密な `1.0` を返すため、
  `peaks_from_correlation` の安定ソートで argmax が先頭 lag
  (位置 0)に移り、gcc_phat の top(0、非 ulp 依存)と同じ
  cluster を形成して 2 手法合意が成立するため。旧逐次式では
  同点が生じず env top は位置 384 で、4 手法票が全て単独
  cluster となり `only 1 method(s) agree` で棄却されていた。
  「曖昧な locate ケースでの偽受理の不存在」は
  `docs/specs/validation-strategy.md` および ADR
  `multi-method-locate-consensus` の文書化された契約であり、
  plan の rollback trigger(golden/Validity 破壊)に該当する。
  プリミティブ再合成形(`dsp::dot`/`dsp::sum_sq` +
  `num/(ex.sqrt()*ey.sqrt())`)でも同様に失敗することを実験で
  確認したため、順序が累算順の ulp に本質依存する当サイトは
  契約保持を優先して逐次式のまま残存許容とし、コード内コメントに
  理由を記した。design file map への残存許容追加を上位に提案する
- `normalize_gain` の除算→逆数乗算差分(`v / rms` →
  `v * r.recip()`)は再合成許容ポリシーで許容。ゼロ・低エネルギー
  の `x.to_vec()` 早期 return と `r < MIN_ENERGY` ガードは保持
  (空入力は `dsp::rms` が 0.0 を返し同一経路に落ちる)
- `corr0` のガード同値性を確認した経緯を記録: 旧 `tn*sn > 0` と
  `ex>0 && ey>0` は非負和・NaN を含め同値で、値差は
  `sqrt(a)*sqrt(b)` vs `sqrt(a*b)` の ulp 級のみ(置換自体は
  数値契約上等価だが、上記の順序副作用で採用不可と判断)

## Related Experiments

### Adopted

<!-- No experiment logs were created for this task. -->

### Rejected

<!-- No experiment logs were created for this task. -->

## Validation

- `mise exec -- cargo test -p al-locate`: 66 tests pass
  (golden 16 含む全件 green)
- `mise exec -- cargo test -p al-analysis`: 全件 green
  (dsp ファサード共有のため確認)
- `mise exec -- cargo build --workspace`: green
- `mise exec -- cargo fmt --all -- --check`: green
- `mise exec -- cargo clippy -p al-locate --all-targets -- -D warnings`:
  green
- 残存同一式確認: `fn energy` は消滅。`map(|v| v*v)` 系の残存は
  methods.rs envelope NCC(残存許容逸脱)・residual.rs:222 の
  `Vec<Vec<f64>>` 行列ゲイン合成(spec 残存許容)・dsp.rs テスト
  コードのみ

### 許容比較へ修正した float テスト一覧

- `crates/al-locate/tests/locate.rs:327`:
  `assert_eq!(rms, expected.energy.after)` → `abs() < 1e-15`。
  residual.rs マルチch rms の per-channel `sum_sq` 再合成による
  結合順差(dispatch 経路またぎ)が原因の ~ulp 差のみで、
  チャンネル配列自体の `assert_eq!`(:317)は厳密比較のまま保持

## Risks

- envelope NCC の逐次式は SIMD dispatch を通らないため、当サイト
  のみ ISA 非依存の値を返し続ける。将来 `corr0` 化を再検討する
  場合は、同点ピークの順位付けまたは合議層の手法内曖昧性
  (`peak_ratio_12 ≈ 1`)検出が前提となる
- `residual.rs` の per-channel `sum_sq` 合成で `ResidualEnergy` の
  永続化値は ~ulp 級に揺れ得る(ADR 許容済み)

## Follow-ups

- design `docs/designs/al-math-vec-primitives.md` の file map に
  envelope NCC を残存許容として追記する修正案(上位判断要)
- 合議層が手法内曖昧性(同点 argmax)を検出しない構造的問題 —
  ulp 順序への依存を解消する場合は別イシューとして切り出す
- T3(al-analysis 側移行: `docs/tasks/al-analysis-vec-migration.md`)、
  T4(residual 減算ループの条件付き再構成採否)

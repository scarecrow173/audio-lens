---
id: "TASK-034Fo9esKwwz1f702dj84C"
type: "task"
status: "todo"
title: "spectral_envelope 逐次 NCC の dsp::corr0 移行(完了証明)"
created: "2026-10-13"
updated: "2026-10-13"
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
    - "docs/plans/implement-locate-argmax-ambiguity.md"
  implemented-by: []
  depends-on:
    - "docs/plans/implement-locate-argmax-ambiguity.md"
    - "docs/tasks/locate-argmax-ambiguity-detection.md"
  blocks:
    - "docs/tasks/locate-argmax-ambiguity-closeout.md"
  supersedes: []
  superseded-by: []
  related:
    - "docs/tasks/al-locate-vec-migration.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/designs/locate-consensus-argmax-ambiguity.md"
    - "docs/specs/locate-consensus-ambiguity-contract.md"
    - "docs/adr/locate-argmax-ambiguity-representation.md"
  derived-by: []
  verifies: []
  verified-by:
    - "docs/test-specs/locate-argmax-ambiguity.md"
  references: []
  defers: []
  deferred-by: []
---

# spectral_envelope 逐次 NCC の dsp::corr0 移行(完了証明)

## Work

plan T3(完了証明)。`SpectralEnvelope::locate` の逐次内積/ノルム
(`crates/al-locate/src/methods.rs:539-547`、残存許容コメント
「`dsp::corr0` 化すると周期入力で各 lag が厳密に同点(≈1.0)に
なり…偽受理の再現」付き)を `dsp::corr0`(`dsp.rs:38` で
re-export 済み、`al-math/src/vec.rs:248`)へ置換し、残存許容
コメントを除去する。

置換対象は hop ループ内の3行:

```rust
let dot: f64 = t_env.iter().zip(&s_env).map(|(a, b)| a * b).sum();
let tn: f64 = t_env.iter().map(|v| v * v).sum::<f64>().sqrt();
let sn: f64 = s_env.iter().map(|v| v * v).sum::<f64>().sqrt();
corr[lag] = if tn * sn > 0.0 { dot / (tn * sn) } else { 0.0 };
```

を `dsp::corr0` 呼出し(正規化内積。ゼロノルム時の 0 扱いは
`corr0` の契約に合わせ、差分は IR に記録)へ置き換える。

この置換は **T1/T2 が green であること(曖昧性契約の定着)を前提**
とする: corr0 により周期入力で厳密同点が生じても、T2 の
`argmax_tie_count` が `AmbiguousAlignment` へ構造的に検出するため
拒否が維持される。golden `pinned_periodic_everywhere_not_accepted_at_wrong_phase`
が、argmax の偶然散乱ではなく構造的検出によって不受理となることを
確認する(TASK-034EpPesSpbjbAQBsmjvJx の残存許容解消)。

### Out of scope(本タスク)

- T2 の曖昧性検出実装そのもの(`argmax_tie_count` 等)
- `full_ncc`/`band_ncc`/`gcc_phat` のカーネル変更(既に決定的
  FFT 経路。spectral_envelope の逐次経路のみが残存許容)
- hop 疎化・`MAX_WINDOWS`・`fft_size`/`bins` 設定の変更

## Classification

- `normal-plan-task`

## Upstream Documents

- `docs/plans/implement-locate-argmax-ambiguity.md`(T3)
- `docs/designs/locate-consensus-argmax-ambiguity.md` §8(T3 輪郭)
- `docs/specs/locate-consensus-ambiguity-contract.md` AC「完了証明」
- `docs/adr/locate-argmax-ambiguity-representation.md`(ゲート条件:
  corr0 移行は契約確立が前提)
- 残存許容の経緯: `docs/tasks/al-locate-vec-migration.md`、
  `docs/impl/ir/al-locate-dsp-re-export-vec-migration.md`

## Dependencies

- `docs/tasks/locate-argmax-ambiguity-detection.md`(T2): 曖昧性
  検出が green でなければ corr0 置換は偽受理を再現する

## Affected Files

- `crates/al-locate/src/methods.rs`(`:539-547` 逐次式 →
  `dsp::corr0` + 残存許容コメント除去)

## Done When

- [ ] `methods.rs` の spectral_envelope 逐次内積/ノルムが
  `dsp::corr0` 呼出しに置換され、残存許容コメントが除去されている
- [ ] `pinned_periodic_everywhere_not_accepted_at_wrong_phase` が
  corr0 下でも不受理を維持する(厳密同点 → `AmbiguousAlignment`
  検出 → 拒否。偽受理の再現なし)
- [ ] golden 全件・`tests/locate.rs`・`tests/prepared.rs` が green
- [ ] corr0 置換の差分(ゼロノルム扱い等)が IR に記録される

## Verification

- [ ] `mise exec -- cargo test -p al-locate` が green
- [ ] `mise exec -- cargo test -p al-locate --test golden
  pinned_periodic` が green(構造的拒否の確認)
- [ ] `rg "残存許容|corr0" crates/al-locate/src/methods.rs` で残存
  許容コメントが除去され `dsp::corr0` 呼出しが存在すること
- [ ] `mise exec -- cargo clippy --workspace --all-targets -- -D
  warnings`・`mise exec -- cargo fmt --all -- --check` が green

---
id: "IMPL-034EbzDAYRzwpQdaAO9pTT"
type: "impl"
status: "completed"
title: "spectral_envelope fft_size entry assert"
created: "2026-10-11"
updated: "2026-10-11"
owners: []
relations:
  source: []
  changes:
    added: []
    modified:
      - type: "code"
        path: "crates/al-math/src/filter.rs"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/tasks/al-math-envelope-fft-size-assert.md"
  implemented-by: []
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related: []
  refines: []
  refined-by: []
  derives-from: []
  derived-by: []
  verifies: []
  verified-by: []
  references:
    - "docs/plans/spectral-envelope-fft-size.md"
    - "docs/designs/spectral-envelope-fft-size.md"
    - "docs/specs/spectral-envelope-fft-size.md"
    - "docs/adr/spectral-envelope-fft-size-assert.md"
  defers: []
  deferred-by: []
metadata:
  experiments:
    adopted: []
    rejected: []
---

# spectral_envelope fft_size entry assert

## Summary

`spectral_envelope` の入口で `fft_size.is_power_of_two()` を assert し、
`0` や非2冪が `max()` 底上げで黙って正常値として扱われる契約違反の
黙置換を解消した。`bins == 0` の早期 return より前に検証するため、
`fft_size` 不正かつ `bins == 0` の入力も panic になる。

## Context

- task: `docs/tasks/al-math-envelope-fft-size-assert.md`
- plan: `docs/plans/spectral-envelope-fft-size.md`
- design: `docs/designs/spectral-envelope-fft-size.md`
- spec: `docs/specs/spectral-envelope-fft-size.md`
- ADR: `docs/adr/spectral-envelope-fft-size-assert.md`(案A: 入口 assert)

本 IR はワークフロー都合で実装完了後に作成した(本来はタスク開始時に
in-progress で作成し逐次更新する運用)。本文は実施済みの差分と検証
結果の実測を記録する。

## Implementation

### Changes

`crates/al-math/src/filter.rs`(+21/-1):

1. `Envelope::with_simd` 先頭(`bins == 0` 早期 return の前)に
   `assert!(self.fft_size.is_power_of_two(), "fft_size must be a non-zero power of two, got {}", self.fft_size)` を追加。
2. `spectral_envelope` の doc に「fft_size は 2 のべき乗(0 は不可)。
   fft_size が信号長より小さい場合は信号長まで拡大する。bins=0 は
   空結果を返す」を明記。`max()` 底上げロジックは維持。
3. `mod tests` に `#[should_panic]` 2 件追加。公開関数経由で
   `fft_size: 0` と `fft_size: 1000` を検証。

### Decisions During Implementation

- assert メッセージは design 文書指定の文言を採用し、実際の不正値を
  含めて panic 原因を特定できるようにした。
- テストは lib 内 `mod tests` に既存スタイルで追加(公開 API 経由で
  契約を検証)。

## Related Experiments

### Adopted

なし(既知の解を持つ機械的修正のため Experiment Log は作成しない)。

### Rejected

なし。

## Validation

- `mise exec -- cargo test -p al-math`: 全合格(unit 17 件・
  integration 14 件、新規 panic テスト 2 件を含む)
- `mise exec -- cargo build --workspace`: 成功
- `cargo fmt --all -- --check`: クリーン
- `mise exec -- cargo clippy -p al-math --lib -- -D warnings`: クリーン
- `audit_docs.js --type all`: findings 0

コミット: `e6c99a3` `fix: assert non-zero power-of-two fft_size at
spectral_envelope entry`(ブランチ
`bugfix/spectral-envelope-fft-size-assert`)。

## Risks

- 不正な `fft_size` を渡していた外部呼出しが存在すれば panic になるが、
  既存呼出し(methods.rs 既定 1024、reference.rs の stft_window)は
  全て正常値であり、既存テストも合格している。
- `bins == 0` でも `fft_size` 不正なら panic になる順序は意図的
  (契約違反を早期 return で隠蔽しない)。

## Follow-ups

なし。タスク done で終了。

---
id: "TASK-DcAsg3VFFjE7QXhkMFM6Hl"
type: "task"
status: "done"
title: "al-math: spectral_envelope の fft_size 入口 assert と doc/テスト追加"
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
    - "docs/plans/spectral-envelope-fft-size.md"
  implemented-by: []
  depends-on:
    - "docs/plans/spectral-envelope-fft-size.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related: []
  refines: []
  refined-by: []
  derives-from: []
  derived-by: []
  verifies: []
  verified-by:
    - "docs/test-specs/spectral-envelope-fft-size.md"
  references: []
  defers: []
  deferred-by: []
---

# al-math: spectral_envelope の fft_size 入口 assert と doc/テスト追加

## Work

plan T1: `crates/al-math/src/filter.rs` に fft_size 契約の assert、
doc 明記、`#[should_panic]` テストを追加する。

- `Envelope::with_simd` の先頭(`if self.bins == 0` の早期
  return より前)に
  `assert!(self.fft_size.is_power_of_two(), "fft_size must be a non-zero power of two, got {}", self.fft_size);`
  を追加する
- `spectral_envelope` の doc コメントを
  「fft_size は 2 のべき乗(0 は不可)。fft_size が信号長より
  小さい場合は信号長まで拡大する。bins=0 は空結果を返す」に
  更新する
- `mod tests` に `#[should_panic]` テストを2件追加する:
  `fft_size: 0` と非2冪(例: 1000)
- `max()` 底上げは変更しない(既存呼出しが依存する契約)

## Classification

- `normal-plan-task`

## Upstream Documents

- `docs/plans/spectral-envelope-fft-size.md`(T1・Verification)
- `docs/designs/spectral-envelope-fft-size.md`(assert 配置・
  doc 文言の正本)
- `docs/test-specs/spectral-envelope-fft-size.md`(R1〜R4)

## Dependencies

- `docs/plans/spectral-envelope-fft-size.md`(承認済み前提文書。
  実質的なブロッカーなし)

## Affected Files

- `crates/al-math/src/filter.rs` のみ(assert 1行 + doc 1行 +
  テスト数行)。他クレート・他ファイルへの変更なし

## Done When

- [ ] `Envelope::with_simd` 先頭に fft_size の2冪 assert がある
  (`bins == 0` 早期 return より前)
- [ ] `spectral_envelope` の doc に底上げ規則が明記されている
- [ ] `fft_size: 0` と非2冪で panic する `#[should_panic]`
  テストが2件追加されている
- [ ] 検証コマンドが全て合格する

## Verification

- [ ] `mise exec -- cargo test -p al-math` が合格する
  (新規 panic テスト2件 + 既存テスト全合格)
- [ ] `mise exec -- cargo build --workspace` が成功する

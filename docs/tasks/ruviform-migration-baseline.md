---
id: "TASK-034P3ObdxjvpOfn9scXk0D"
type: "task"
status: "todo"
title: "移行 baseline の確定(AL#98)"
created: "2026-10-30"
updated: "2026-10-30"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/audio-lens/issues/98"
  changes:
    added: []
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/plans/ruviform-foundation-migration.md"
  implemented-by: []
  depends-on:
    - "docs/plans/ruviform-foundation-migration.md"
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
  references: []
  defers: []
  deferred-by: []
---

# 移行 baseline の確定(AL#98)

## Work

移行前の現行実装を「比較の正」として固定する。Issue
[#98](https://github.com/scarecrow173/audio-lens/issues/98) を正本とし、
commit SHA・toolchain・OS/CPU・入力 hash・設定・golden 出力・実行
manifest を before 側として保存し、各切替 Issue の比較手順が参照
できる形にする。

- 現行アルゴリズム・実サンプル・golden を再現可能な形で固定
- 性能計測(wall/busy-time・CPU・メモリ/alloc・I/O・成果物サイズ)
  の手順と基準値を確定
- bit-exact 維持契約と浮動小数誤差許容契約の区分を明文化

## Classification

- `normal-plan-task`

## Upstream Documents

- `docs/plans/ruviform-foundation-migration.md`
- `docs/specs/ruviform-foundation-migration.md`
- Issue [#98](https://github.com/scarecrow173/audio-lens/issues/98)

## Dependencies

- `docs/plans/ruviform-foundation-migration.md`(段1 前提)

## Affected Files

- `crates/al-testkit`(fixture・signal・media 生成)
- `crates/al-cli`(再現実行の入口)
- `crates/al-dataset`(golden 比較対象の読出し)
- golden / 計測スクリプトの保存先(#98 で確定)

## Done When

- [ ] 再現可能な入力一覧・golden・性能 baseline が確定し、#98 の
      完了条件を満たす
- [ ] bit-exact / 許容誤差の比較区分が文書化されている
- [ ] 切替 Issue が同一手順で before/after を再現できる

## Verification

- [ ] 保存した baseline から golden が再生成できる
- [ ] 計測手順が再現可能(同一結果が得られる)

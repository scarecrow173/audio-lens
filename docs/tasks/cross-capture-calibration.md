---
id: "TASK-036eLkJhGfDsAqWeRtYu"
type: "task"
status: "todo"
title: "analysis: キャプチャ間経路校正(#41)"
created: "2026-10-09"
updated: "2026-10-09"
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
    - "docs/plans/media-format-analysis-extensions.md"
  implemented-by: []
  depends-on:
    - "docs/tasks/band-lag-gain.md"
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
    - "docs/test-specs/media-format-analysis-extensions.md"
  references:
    - "https://github.com/scarecrow173/audio-lens/issues/41"
  defers: []
  deferred-by: []
---

# analysis: キャプチャ間経路校正(#41)

## Work

元ツールチェーン手法4(録画経路の校正: 共通 BGM を基準に
bug ÷ no_bug で経路差を分離)を自動化する。

- 両キャプチャで locate された共通ソースの `transfer_h` を
  差分/比率化し「キャプチャ間経路差」観測を生成
- この観測の置き場所を dataset スキーマに定義(ペア単位ではなく
  キャプチャ間比較という新しいスコープ — dataset-design の
  input/capture/job 軸のどこに載せるか決める)
- 共通ソースが複数ある場合の集約方法(中央値等)を定義
- 共通ソースが無いキャプチャ組では validity=unsupported として
  記録(観測不能を区別)

## Classification

- `normal-plan-task`

## Upstream Documents

- `docs/plans/media-format-analysis-extensions.md`
- `docs/designs/dataset-schema.md`(スキーマの置き場所)
- issue #41

## Dependencies

- `docs/tasks/band-lag-gain.md`(transfer 系の validity メタデータ
  形を揃える)

## Affected Files

- `crates/al-analysis/src/`(新規ファミリ or reference 拡張)
- `crates/al-dataset/`(スキーマ: キャプチャ間比較の置き場所)

## Done When

- [ ] 共通ソースを持つキャプチャ間で経路差観測が出力される
- [ ] 共通ソースが無い場合 validity=unsupported で記録される
- [ ] 既存ペア単位観測が変わらない

## Verification

- [ ] `cargo test -p al-analysis -p al-dataset`

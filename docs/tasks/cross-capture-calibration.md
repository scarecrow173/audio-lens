---
id: "TASK-036eLkJhGfDsAqWeRtYu"
type: "task"
status: "done"
title: "analysis: キャプチャ間経路校正(#41)"
created: "2026-10-09"
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

- [x] 共通ソースを持つキャプチャ間で経路差観測が出力される
- [x] 共通ソースが無い場合 validity=unsupported で記録される
- [x] 既存ペア単位観測が変わらない

## Verification

- [x] `cargo test -p al-analysis -p al-dataset`

## Implementation

- ExpensiveAnalysisStage に job スコープのキャプチャ組観測を追加。
  schema の input/capture/job 軸では比較が複数 input にまたがるため job を採用した。
  `CaptureComparison` が比較方向と共通ソースを型として定義し、既存ペア観測を保持する。
- `transfer_h` の振幅比を bin 単位で共通ソース間の中央値に集約し、
  比率・dB 差・ソース支持数をチャンクテンソルへ保存する。
  位相は locate の時刻原点依存のため今回の観測には含めない。
- accepted/valid locate、alignment に対応する H、同一 rate/FFT grid、
  両側 MSC >= 0.5、有限の正振幅を採用条件とする。
  共通ソースなしは新しい共通 Validity::Unsupported で観測不能を表し、
  共通ソースありの欠損・不一致・支持不足は InsufficientSupport に分離する。
- 複数ソース・外れ値・偶数中央値・bin ごとの欠損支持・逆方向・grid 不一致を
  unit test で検証。実 Locate → Cheap → Reference → Expensive の integration test で
  2倍の経路差、保存スコープ、tensor 部分読み出し、unsupported、ペアJSON不変を検証する。

## Verification Results (2026-10-11)

- `mise exec -- cargo test -p al-analysis -p al-dataset`: 135 passed / 0 failed (golden 19含む)。
- `mise exec -- cargo fmt --all -- --check`: passed。
- `mise exec -- cargo clippy --workspace --all-targets -- -D warnings`: passed。
- `git diff --check`: passed。

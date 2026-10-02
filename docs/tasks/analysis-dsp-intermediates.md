---
id: "TASK-034z9nuJ4U5rxECJrXf3yj"
type: "task"
status: "todo"
title: "al-analysis: 複素 STFT + スペクトル中間生成物"
created: "2026-10-02"
updated: "2026-10-02"
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
    - "docs/plans/analysis-stages.md"
  implemented-by: []
  depends-on:
    - "docs/plans/analysis-stages.md"
    - "docs/tasks/analysis-dataset-foundation.md"
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
    - "docs/test-specs/analysis-stages.md"
  references: []
  defers: []
  deferred-by: []
---

# al-analysis: 複素 STFT + スペクトル中間生成物

## Work

`al-analysis::dsp` に複素 STFT とスペクトル中間生成物を実装する
(外部 DSP クレート不使用、`al-locate::dsp` の FFT を再利用)。

- 複素 STFT(窓関数、hop、フレーム分割、窓正規化)
- 各分解能クラス(fine / medium / coarse)ごとの派生:
  magnitude / power / PSD / 固定帯域パワー / 1/3 オクターブ
  パワー / スペクトル包絡
- CSD(相互スペクトル密度)と複素コヒーレンス(2ch 間)
- すべて `intermediate` ロールへ tensor として永続化できる
  形で返す(呼び出し側が DatasetWriter に書く)
- 実解像度(window/hop/bins)を返却して FeatureRecord の
  メタデータに使えるようにする

## Classification

- `normal-plan-task`

## Upstream Documents

- `docs/plans/analysis-stages.md`
- `docs/specs/cheap-analysis-features.md`
- `docs/specs/reference-analysis.md`

## Dependencies

- `docs/tasks/analysis-dataset-foundation.md`

## Affected Files

- `crates/al-analysis/src/dsp.rs`(新規)

## Done When

- [ ] 複素 STFT が既知信号で正しい bin を返す
- [ ] 全中間生成物が決定的に計算できる
- [ ] CSD / 複素コヒーレンスが位相情報を保持する

## Verification

- [ ] `cargo test -p al-analysis`

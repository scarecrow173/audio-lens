---
id: "TASK-034z9oB8rhYcBdeSRvtsKy"
type: "task"
status: "done"
title: "al-viz: dataset 範囲/チャンク読み出し基盤"
created: "2026-10-02"
updated: "2026-10-03"
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
    - "docs/plans/visualization-export.md"
  implemented-by: []
  depends-on:
    - "docs/plans/visualization-export.md"
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
    - "docs/test-specs/visualization-export.md"
  references: []
  defers: []
  deferred-by: []
---

# al-viz: dataset 範囲/チャンク読み出し基盤

## Work

`al-viz` クレート新設。dataset から派生データのみを読む基盤を作る
(生 PCM 全体を再デコードしない)。

- `DatasetReader` に範囲/チャンク読み出しを追加:
  - audio store: サンプル範囲 `[start, end)` の部分読み出し
    (WAV 全体をメモリに載せない)
  - tensor: 指定チャンクのみの読み出し
  - table: 述語/範囲での行選択
- schema version 検証: dataset のスキーマが想定と違えば
  明示的失敗(互換レイヤなし)
- viz が必要とする入力一覧(artifact 名のクロスプラン契約):
  STFT / PSD / 帯域メトリクス / locate・time-map / events /
  residuals — 存在しない role は skipped 扱いにできるよう
  問い合わせ API を用意

## Classification

- `normal-plan-task`

## Upstream Documents

- `docs/plans/visualization-export.md`
- `docs/specs/visualization-dataset.md`
- `docs/specs/analysis-dataset.md`

## Dependencies

- `docs/tasks/analysis-integration-tests.md`(上流ロールの
  artifact 名契約が確定していること)

## Affected Files

- `Cargo.toml`(workspace members)
- `crates/al-viz/`(新規)
- `crates/al-dataset/src/reader.rs`(範囲/チャンク読み出し)

## Done When

- [ ] 長尺 audio の範囲読み出しが全体ロードより小さいメモリで動く
- [ ] schema 不一致が明示的エラーになる
- [ ] クロスプラン契約の artifact 名が解決できる

## Verification

- [ ] `cargo test -p al-dataset -p al-viz`

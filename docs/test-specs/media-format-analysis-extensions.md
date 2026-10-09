---
id: "TSPEC-035jZxCvBnMqWeRtYuIo"
type: "test-spec"
status: "proposed"
title: "メディア形式拡大・正規化アーキテクチャと解析精度拡張"
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
  implements: []
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
  verifies:
    - "docs/plans/media-format-analysis-extensions.md"
    - "docs/specs/media-ingest.md"
    - "docs/specs/reference-analysis.md"
  verified-by: []
  references: []
  defers: []
  deferred-by: []
metadata:
  experiments:
    adopted: []
    rejected: []
---

# メディア形式拡大・正規化アーキテクチャと解析精度拡張

## Verifies

- `docs/plans/media-format-analysis-extensions.md`
- `docs/specs/media-ingest.md`
- `docs/specs/reference-analysis.md`

## Coverage

### MediaDecoder ADR(TASK-036a)

- [ ] ADR が `accepted` で `docs/adr/` に存在する
- [ ] native 優先 + フォールバックの backend 方針が決定済み
- [ ] `mp4-demux-decode.md` との関係(supersede/amend)が明記

### native バックエンド移行(TASK-036b)

- [ ] WAV/AVI/MP4 の既存 ingest テストがパス
- [ ] decode 経路がレジストリ経由になる
- [ ] provenance に decoder/backend 種別が記録される

### ROI 輝度(TASK-036c)

- [ ] ROI 指定の輝度時系列が出力される
- [ ] ROI 輝度ステップが avsync イベントとして照合される
- [ ] ROI 未指定時の既存挙動が変わらない

### 帯域ラグ補償ゲイン(TASK-036d)

- [ ] 帯域ごとの最良ラグ+LS ゲイン+上位半分中央値が出力される
- [ ] coherence/support の validity メタデータが付く
- [ ] 非コヒーレント帯域で PSD 比由来の過大評価が出ない

### キャプチャ間校正(TASK-036e)

- [ ] 共通ソースで経路差観測が出力される
- [ ] 共通ソースなしの組で validity=unsupported になる
- [ ] 既存ペア単位観測が変わらない

## Verdict Criteria

- 全項目がパス: `implemented`
- いずれか欠落: `failed`

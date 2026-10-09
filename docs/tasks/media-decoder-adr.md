---
id: "TASK-036aQwErTyUiOpAsDfGhJk"
type: "task"
status: "todo"
title: "MediaDecoder トレイト + レジストリ設計の ADR(#36 フェーズ0)"
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
    - "docs/plans/media-format-analysis-extensions.md"
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
    - "https://github.com/scarecrow173/audio-lens/issues/36"
  defers: []
  deferred-by: []
---

# MediaDecoder トレイト + レジストリ設計の ADR(#36 フェーズ0)

## Work

issue #36 のプラガブルデコーダ設計を ADR として決定する。

- `MediaDecoder` トレイトの責務境界(コンテナ demux と codec
  decode の分離/一体)
- backend 評価: `symphonia`(純 Rust、音声のみ)/ `ffmpeg`
  子プロセスフォールバック(feature-gated・承認制)/ 自前拡張
- 出力正規化契約(統一 PCM 表現 + provenance 記録)
- 既存 ADR `mp4-demux-decode.md`(codec decode をスコープ外と
  した判断)との関係 — supersede/amend の形を決める
- ADR ドキュメントを `docs/adr/` に追加し README 索引を更新

## Classification

- `normal-plan-task`

## Upstream Documents

- `docs/plans/media-format-analysis-extensions.md`
- `docs/adr/mp4-demux-decode.md`
- issue #36

## Dependencies

なし(設計フェーズ。実装タスクの前提)

## Affected Files

- `docs/adr/`(新規 ADR)
- `docs/adr/README.md`(索引)

## Done When

- [ ] ADR が `accepted` ステータスで追加される
- [ ] backend 選定(native 優先 + フォールバック方針)が決定
- [ ] #36 のフェーズ1(既存実装の移行)に必要な契約が定義済み

## Verification

- [ ] ADR が `docs/adr/README.md` 索引に登録される

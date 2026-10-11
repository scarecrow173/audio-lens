---
id: "PLAN-035bGhJkLmNpQrStUvWxYz"
type: "plan"
status: "approved"
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
  derives-from:
    - "docs/specs/media-ingest.md"
    - "docs/specs/reference-analysis.md"
    - "docs/adr/mp4-demux-decode.md"
    - "docs/designs/media-format-analysis-extensions.md"
  derived-by: []
  verifies: []
  verified-by:
    - "docs/test-specs/media-format-analysis-extensions.md"
  references:
    - "https://github.com/scarecrow173/audio-lens/issues/36"
    - "https://github.com/scarecrow173/audio-lens/issues/38"
    - "https://github.com/scarecrow173/audio-lens/issues/39"
    - "https://github.com/scarecrow173/audio-lens/issues/41"
  defers: []
  deferred-by: []
metadata:
  experiments:
    adopted: []
    rejected: []
---

# メディア形式拡大・正規化アーキテクチャと解析精度拡張

## Goal

ゲームキャプチャ運用で普通に使われる主要コンテナ/コーデックへの
対応経路(#36)、A/V 同期の精密化のための ROI 輝度追跡(#39)、
非コヒーレント PSD 比に依存しない帯域別ラグ補償ゲイン(#38)、
録画経路差の分離(#41)を実装する。

対応 issue: #36, #38, #39, #41

## Scope

`al-media` のデコーダ抽象化(#36)はスコープが大きいため、
フェーズ 1(トレイト・レジストリ設計と既存実装の移行)までを
本プランの対象とし、各コーデックバックエンドの追加は後続計画に
分離する。

## Design Inputs

- `docs/specs/media-ingest.md` / `docs/designs/ingest-av.md`
- `docs/designs/video-features.md`(video_features の既存構造)
- `docs/adr/mp4-demux-decode.md`: MP4 codec decode 範囲外の既存
  ADR — #36 の実装はこの ADR の supersede/amend が必要
- 元ツールチェーン資料:
  `C:\Users\akiyoshi\Downloads\analysis_toolchain.md` との差分は
  issues #36/#38/#39/#41 に記録済み

## Task Decomposition

1. `MediaDecoder` トレイト + レジストリ設計の ADR(#36 フェーズ0:
   backend 選定 — symphonia / ffmpeg 子プロセス / 自前の評価)
2. 既存 WAV/AVI/MP4 実装を `MediaDecoder` の `native` バックエンド
   へ移行(#36 フェーズ1)
3. ROI 輝度追跡: `VideoFeatureConfig` に `rois` 追加、
   領域平均輝度時系列・ステップイベント化、avsync 接続(#39)
4. 帯域別ラグ補償ゲイン: 1/3oct 帯域ごとの最良ラグ+LS ゲイン、
   相関上位半分中央値(#38)
5. キャプチャ間経路校正: 共通ソース transfer_h の差分を
   FeatureRecord/tensor 化(#41)

## Verification

- `docs/test-specs/media-format-analysis-extensions.md` の契約
- 既存 golden 互換 + 新規ケース(ROI 検出、帯域ゲイン、校正差分)

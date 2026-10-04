---
id: "DESIGN-034AasAnWroBLn28EPuQ5Y"
type: "design"
status: "approved"
title: "MP4 demux とタイミング観測設計"
created: "2026-10-04"
updated: "2026-10-04"
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
  depends-on:
    - "ingest-av.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related: []
  refines: []
  refined-by: []
  derives-from:
    - "../specs/media-ingest.md"
    - "../discovery/container-coverage-gaps.md"
    - "../adr/mp4-demux-decode.md"
    - "../adr/implementation-stack.md"
  derived-by: []
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# MP4 demux とタイミング観測設計

## Context

spec `../specs/media-ingest.md` は video capture files を要求するが、
現状は WAV と非圧縮 AVI のみ。ADR `../adr/mp4-demux-decode.md` に
基づき、MP4(ISO-BMFF)を「demux + コンテナ/トラック/タイミング
観測」に限定して対応する。コーデックの decode は行わない
(非圧縮 PCM サンプルエントリの取り出しは demux の範囲とする)。

## Scope

### In Scope

- ISO-BMFF box 構造のパース(ftyp/moov/trak/mdia/minf/stbl 等)
- トラック単位のメタデータ(hdlr 種別・timescale・duration・
  stsd の codec fourcc・言語)
- サンプルタイミング観測: stts(DTS 系列)・ctts(合成オフセット
  →PTS)・stss・stsz・stsc/stco/co64 を用いた per-sample 時刻
  系列を `SequenceTimestamps` / `TimingObservation` に載せる
- 非圧縮 PCM サンプルエントリ(`sowt`/`twos`/`lpcm` 等)の
  サンプル取り出し(demux 範囲)
- 圧縮コーデック(avc1/hev1/mp4a 等)に対する
  `decode/input unavailable` の明示失敗とメタデータ記録
- mdat 参照用の stsc/stco/co64 チャンクマップ

### Out of Scope

- フラグメント MP4(moof)の網羅的対応(検出して明示失敗でよい)
- H.264/H.265/AAC 等のコーデック decode
- edit list / 複合タイムラインの解釈(観測として記録はする)
- DRM/暗号化トラック

## Components and Boundaries

- `al-media::mp4` コンポーネント: box パーサ + trak 別
  メタデータ + サンプルテーブル解釈。`avi.rs` と同じく
  「宣言事実 + mux 由来の観測」を分けて記録する
- `al-media::format` に MP4 シグネチャ(ftyp)検出を追加
- ingest stage は `vids`/`soun` トラックを区別して記録し、
  PCM 取り出し可能な `soun` のみ PCM として渡す
- 境界: コーデックは識別・記録のみ。decode 失敗は
  `MediaError` の明示失敗に統一

## Data and Control Flow

- ftyp/brand を記録 → moov 配下を走査 → trak ごとに
  tkhd/mdhd/hdlr/stbl を解釈 → stts/ctts からサンプル時刻列を
  構築 → `observe_timing` で `TimingObservation` を生成
- soun が PCM サンプルエントリなら mdat から PCM を取り出し、
  WAV と同じ decoded PCM として後段へ渡す
- 圧縮トラックはメタデータ+タイミングのみ記録し、decode 要求は
  明示失敗

## Non-Functional Constraints

- 純 Rust・外部依存なし・決定的
- 大きなファイルはサンプルテーブル(インデックス)のみ読み、
  mdat 全量を常時読み込まない
- timescale 由来の時刻のみで nominal FPS 演算をしない

## Risks and Trade-offs

- ISO-BMFF の箱は多様で全網羅は不可能 → 必要な box のみ解釈し、
  未知 box はスキップして記録
- stts/ctts 解釈の端ケース → 合成 fixture で既知系列を検証

## Deferred Design Concerns

- fMP4 / edit list / 多言語トラックの扱い | 実需要未確認 |
  実材料が出た時点 | 一部実 MP4 が観測対象外になる
- 圧縮コーデック decode | 外部依存の要否 | 別 ADR/決定 |
  実用解析が制限される

## Verification Notes

- [ ] 既知 MP4 fixture のトラック/タイミングが記録される
- [ ] VFR 的な stts 系列が正しく観測される
- [ ] 圧縮コーデックの decode 要求が明示失敗になる

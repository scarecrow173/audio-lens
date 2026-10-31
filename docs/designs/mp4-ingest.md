---
id: "DESIGN-034AasAnWroBLn28EPuQ5Y"
type: "design"
status: "approved"
title: "MP4 demux とタイミング観測設計"
created: "2026-10-04"
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
- サンプルタイミング観測: stts/ctts/stss/stsz/stsc/stco/co64 を
  解釈し per-sample 時刻系列を構築する。`observe_timing` は
  単調非減少を要求するため、B フレームの再並び込みで負の区間を
  生じうる ctts 由来 PTS をそのまま流さない。**観測対象は
  decode order の DTS 系列**(stts のみで単調)とし、ctts 由来
  PTS は別系列として記録する(または提示順にソートして別名で
  記録)。両系列はラベルで区別する
- 非圧縮 PCM サンプルエントリ(`sowt`/`twos`/`lpcm` 等)の
  サンプル取り出し(demux 範囲)。エンディアン・符号は fourcc/
  lpcm フラグで解釈し(twos=BE、sowt=LE、lpcm は isFloat/
  isBigEndian/isSignedInteger/isPacked フラグ)、`DecodedAudio`
  契約どおり**整数スケール f64 + 対応する `SampleFormat`** で
  返す(WAV と同じ前正規化契約。`decode_input` が format_scale
  で割るため二重正規化は禁止)
- 複数 `soun` トラックがある場合、PCM 取り出し対象は**最初の
  PCM-decodable `soun`** とし、残りはメタデータ+タイミングのみ
  記録(AVI の first-`auds` 規則と揃える)
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
- `al-media::format` に MP4 シグネチャ(ftyp)検出を追加。
  ftyp は仕様上 optional であり `free`/`wide`/`styp`/`mdat` で
  始まる正当なファイルは「unknown format」として扱い MP4 観測
  を試みない
- ingest stage は `vids`/`soun` トラックを区別して記録する。
  **部分 ingest 契約**: 現行 `IngestStage` はデコード失敗を
  `StageError` に伝播させ成果物が一切登録されない。本設計では
  コンテナ観測(トラックメタデータ・タイミング)は decode の
  可否と独立に記録し、PCM のみ `Option` として扱う。全トラック
  が圧縮の主流ケースでも ingest は metadata/timing 成果物を
  永続化して完了し、`decoded_pcm` 成果物は省略される
  (分析ステージは `decode_input` → `Ok(None)` →
  `DecodeInputUnavailable` 記録の既存経路で処理する。
  dataset / export の metadata-only 通過は
  既存経路に含まれず、`video-av-features` 側の部分 ingest
  タスク(`docs/tasks/video-features-ingest.md`)が追加実装する
  下流伝播に依存する)。
  1 入力の失敗は他入力の記録を巻き込まない。
  **エラー境界の不変条件**: コンテナ構造の破損(ボックス不整合・
  サンプルテーブルの矛盾・stco/co64 の EOF 外参照)は codec に
  関係なく probe/direct decode 共通境界で `Malformed` として拒否する。
  stsc の先頭・昇順・非ゼロ samples_per_chunk・stsd index・stsz 件数を
  全トラックで検証する。空の完全テーブルは有効。
  サンプルデータ層のデコード不可は `decode_unavailable` integrity
  観測として記録し ingest を完走させる。detail.kind は registry と
  共通の `UnavailableKind` (`capability`/`malformed`/`io`) で、codec
  非対応は capability、不完全 PCM sample/frame やゼロ rate/channel は
  局所 malformed。構造検証後の PCM decode だけを局所失敗境界とする
- 永続化するストリームタイミングのフィールド名は
  AVI 固有の `movi_order_timestamps` ではなく、コンテナ中立の
  命名(例: `stream_timestamps` + 由来ラベル
  `movi_order`/`sample_table`)に一般化してから MP4 側も記録する
  (表現ドリフト防止)
- 境界: コーデックは識別・記録のみ。decode 自体の「要求」は
  `MediaError`/`decode/input unavailable` の明示失敗に統一するが、
  ingest 時点ではこれは失敗ではなく integrity 観測として記録し、
  PCM を必要とする後段へは `decoded_pcm` 不在として伝える

## Data and Control Flow

- ftyp/brand を記録 → moov 配下を走査 → trak ごとに
  tkhd/mdhd/hdlr/stbl を解釈 → stts/ctts からサンプル時刻列を
  構築。decode order DTS 系列に `observe_timing` を適用して
  `TimingObservation` を生成し、ctts 由来 PTS 系列は別ラベルで
  記録する
- soun が PCM サンプルエントリなら mdat から PCM を取り出し、
  エンディアン/符号を解釈して整数スケール f64 + `SampleFormat`
  で decoded PCM として後段へ渡す
- 圧縮トラックはメタデータ+タイミングのみ記録する。decode が
  行えなくても ingest 自体は成功し、`decode_unavailable`
  はメタデータの integrity 観測として記録される

## Non-Functional Constraints

- 純 Rust・外部依存なし・決定的
- mdat の PCM 取り出しは必要なチャンクのみを参照し、デマックス
  目的で追加の全量コピーを作らない。なお現行 ingest は入力全体を
  `fs::read` でバッファしており、本 WS はその I/O モデルを
  変えない(全入力バッファリングは既知のスケーリング制限として
  別 ADR で扱う)
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

---
id: "DISC-034AajDdIqPQODwBlBJPPC"
type: "discovery"
status: "resolved"
title: "コンテナカバレッジのギャップ(MP4/ISO-BMFF)"
created: "2026-10-04"
updated: "2026-10-04"
owners: []
relations:
  source:
    - "../specs/media-ingest.md"
    - "../../crates/al-media/src/format.rs"

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
  derived-by:
    - "../designs/mp4-ingest.md"
    - "../adr/mp4-demux-decode.md"
    - "../plans/mp4-ingest.md"
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# コンテナカバレッジのギャップ(MP4/ISO-BMFF)

## Exploration Goal

issue #3(media ingest)は「video capture files」を要求するが、
現状は WAV と非圧縮 AVI(PCM audio)のみ。実際のゲームキャプチャで
主流の MP4/ISO-BMFF をどこまでサポートするかを確定する。

## Key Issues and Assumptions

- 実装済み: WAV デコード(PCM/float 各種)、AVI demux
  (PCM auds + vids 宣言メタデータ)、タイミング観測、辞書
- 未対応: MP4/MKV/MOV 等のコンテナ、AAC/H.264 等の不可逆
  コーデック。現状は `decode/input unavailable` で明示失敗
- 前提: 外部依存(FFmpeg 等)の追加はしない。純 Rust・決定的・
  失敗は常に explicit

## Alternatives and Comparison

| Option | Pros | Cons | Lean |
| --- | --- | --- | --- |
| MP4 demux: コンテナ/トラック/タイミング観測のみ(コーデック decode は明示失敗) | 外部依存なしで実用キャプチャの A/V タイミング観測が可能・issue #3 のメタデータ/タイミング要求の大半を満たす | 音声 PCM が取れないため解析本体は動かない | yes |
| FFmpeg 依存でフル decode | 完全な実用性 | 大きな依存・再現性リスク・別途承認が必要 | no |
| 現状維持(WAV/AVI のみ) | 変更不要 | issue #3 の「video capture files」に実用上応えられない | no |

## Tentative Conclusions and Hypotheses

- confirmed: MP4(ISO-BMFF)の box 構造パース + stts/ctts 等の
  タイミングテーブル観測は純 Rust で決定的に実装可能
- confirmed: 音声 PCM を取れない入力でも、コンテナ/トラック/
  タイミング観測を成果物として残すことは ingest の
  「observation」責務に合致する。音声 decode が必要な後続は
  `decode/input unavailable` で明示失敗
- hypothesis: 既存 `StreamTiming`/`TimingObservation` の抽象に
  MP4 のタイミング系列を載せられる(design で確認)

## Open Questions

- MP4 の音声 decode を伴わない ingest 成果物をどのロール/
  成果物形状で永続化するか → design で決定
- フラグメント MP4(fMP4)や複数 trak の扱い → design で決定

## Promotion Candidates

- [ ] spec-doc needed: 既存 `docs/specs/media-ingest.md` が
  コンテナ一般を要求済み。MP4 固有の補遺が必要なら追加
- [x] adr-doc needed: 「MP4 は demux/タイミング観測のみ、
  コーデック decode は外部依存判断として延期」のスコープ決定

## Sources and Relations

- GitHub issue #3 (media ingest)
- GitHub issue #11 (A/V timing が MP4 でも観測できると価値が増す)
- `docs/specs/media-ingest.md`
- `docs/adr/implementation-stack.md`
- `crates/al-media/src/format.rs`(明示失敗ポリシー)

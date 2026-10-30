---
id: "TASK-034P3OO5I3ZctV72lHagYB"
type: "task"
status: "todo"
title: "Media I/O の Ruviform 切替(AL#100)"
created: "2026-10-30"
updated: "2026-10-30"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/audio-lens/issues/100"
  changes:
    added: []
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/plans/ruviform-foundation-migration.md"
  implemented-by: []
  depends-on:
    - "docs/plans/ruviform-foundation-migration.md"
    - "docs/tasks/ruviform-migration-baseline.md"
    - "docs/tasks/ruviform-dependency-ci.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related: []
  refines: []
  refined-by: []
  derives-from: []
  derived-by: []
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# Media I/O の Ruviform 切替(AL#100)

## Work

`al-media` の汎用メディア I/O(WAV/AVI/MP4 demux、DIB/AAC decode、
`MediaDecoder` registry、container/timing 観測の原材料)を
Ruviform media-io + backend(#151 観測契約・#154 backend)へ
切り替える。Issue
[#100](https://github.com/scarecrow173/audio-lens/issues/100) を正本とする。

- 段階切替順序: WAV → AVI/MP4 + AAC → 動画フレーム・時刻・
  複数 stream
- `al-media` は ingest adapter へ縮小し、`IngestResult`/
  `Unavailable` 分類・partial-ingest ポリシー・integrity
  observation・辞書適用・Manifest/Provenance 記録の固有意味論は
  AudioLens に残す
- 元 PCM 契約・時刻契約・decode unavailable/corrupt/IO 失敗の
  区別を切替後も保持する

## Classification

- `normal-plan-task`

## Upstream Documents

- `docs/plans/ruviform-foundation-migration.md`
- `docs/designs/ruviform-migration-responsibility-map.md` §2
- Issue [#100](https://github.com/scarecrow173/audio-lens/issues/100)

## Dependencies

- `docs/tasks/ruviform-migration-baseline.md`
- `docs/tasks/ruviform-dependency-ci.md`
- 外部依存: Ruviform [#151](https://github.com/scarecrow173/ruviform/issues/151)
  (観測契約)、[#154](https://github.com/scarecrow173/ruviform/issues/154)
  (codec backend)

## Affected Files

- `crates/al-media`(wav/avi/dib/mp4/symphonia/decoder/timing/metadata
  の汎用部を Ruviform 呼出へ置換、ingest 意味論は残留)
- `crates/al-media/src/video.rs`(`extract_features` のフレーム供給元を
  Ruviform decode(#154)出力経由へ再配線。特徴抽出意味論
  (`VideoFeatureConfig`/`VideoEvent`/`VideoThumbs`/
  `extract_decoded_features`)は AL に残留)
- `crates/al-locate/src/decode.rs`(取得元を Ruviform I/O 型へ)
- `crates/al-analysis/src/video.rs`(変更なし — `VideoFeatureArtifact`
  を dataset record に投影するのみで切替対象ではない)
- `crates/al-cli`(再現実行・失敗診断の入口)

## Done When

- [ ] 責務マップ §2 の観測情報が損失なく取得でき、#100 の完了
      条件を満たす
- [ ] 段階別切替ごとに #98 baseline との比較記録がある
- [ ] ingest 意味論・provenance 記録が維持される

## Verification

- [ ] WAV/AVI/MP4+AAC fixture で旧実装との観測・golden 比較
- [ ] decode unavailable/corrupt/IO 失敗の区別が維持される
- [ ] `mise run test` 相当が通る

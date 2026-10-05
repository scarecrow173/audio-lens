---
id: "TASK-034AaFzhq1S2Z7tm0XFXSc"
type: "task"
status: "done"
title: "video: DIB/rawvideo フレームデコード + 輝度/差分/シーンチェンジ"
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
  implements:
    - "docs/plans/video-av-features.md"
  implemented-by: []
  depends-on:
    - "docs/plans/video-av-features.md"
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
    - "docs/test-specs/video-av-features.md"
  references: []
  defers: []
  deferred-by: []
---

# video: DIB/rawvideo フレームデコード + 輝度/差分/シーンチェンジ

## Work

`al-media` に非圧縮 AVI vids フレームのデコードを実装する。BI_RGB 系 24/32bpp、bottom-up/top-down(BITMAPINFOHEADER height の符号)を扱い、24bpp 行の DWORD パディングを正しく処理する。movi の非フレームチャンク(pc 等)はタイムスタンプ消費とフレーム index をずらさないよう区別する。フレームごとの平均輝度・前フレーム差分・シーンチェンジ候補フラグを算出する(フレーム全体は保持しない)。

## Classification

- `normal-plan-task`

## Upstream Documents

- `docs/plans/video-av-features.md`
- `docs/test-specs/video-av-features.md`

## Dependencies

- none

## Affected Files

- `crates/al-media/src/video.rs`
- `crates/al-media/src/avi.rs`
- `crates/al-media/src/lib.rs`

## Done When

- [ ] 非圧縮 AVI の各フレームで輝度・差分・シーンチェンジ候補が算出される
- [ ] top-down DIB と 24bpp パディング入り行が正しくデコードされる
- [ ] 非フレーム vids チャンクでフレーム index がずれない
- [ ] 圧縮 vids は `decode/input unavailable` として扱われる

## Verification

- [ ] `cargo test -p al-media`

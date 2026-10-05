---
id: "ADR-034AapHIAOqZfyrLPIa14w"
type: "adr"
status: "accepted"
title: "映像フレームデコードは非圧縮 AVI に限定する"
created: "2026-10-04"
updated: "2026-10-04"
owners: []
relations:
  source:
    - "../discovery/video-av-feature-gaps.md"
    - "../adr/implementation-stack.md"

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
    - "../discovery/video-av-feature-gaps.md"
    - "../adr/implementation-stack.md"

  derived-by: []
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
metadata:
  adr:
    decision-makers: []
    consulted: []
    informed: []
---

# 映像フレームデコードは非圧縮 AVI に限定する

## Context and Problem Statement

issue #11 はシーンチェンジ・輝度変化・モーション系列など
フレーム内容由来の映像特徴を要求するが、これらはフレームの
デコードを前提とする。現状の `al-media` は宣言メタデータと
タイミング観測のみで、フレーム本体はデコードしない。

## Decision Drivers

* 外部依存を増やさない(決定性・再現性・配布容易性)
* 「観測はデータ、失敗は explicit」という既存方針との一貫性
* 実装コストと issue #11 達成のバランス

## Considered Options

* 非圧縮 AVI(DIB/rawvideo)のみフレームデコードする
* FFmpeg 等の外部デコーダで任意コーデックに対応する
* フレーム由来の特徴を実装しない

## Decision Outcome

Chosen option: "非圧縮 AVI(DIB/rawvideo)のみフレームデコードする",
because 外部依存なしに決定的な映像特徴を実装でき、既存の
コンテナスコープと「対応不能は明示失敗」ポリシーを維持できるため。

### Consequences

* Good, because 依存なし・決定的・テスト fixture をプログラム生成できる。
* Bad, because MP4/H.264 等の実用キャプチャでは映像特徴が
  `decode/input unavailable` になる。別コンテナ対応は別決定。

### Confirmation

非圧縮 AVI fixture で scene-change / luminance / motion 系列が
算出されること、圧縮コーデック入力では映像特徴が
`decode/input unavailable` として記録されることをテストで確認する。

## Implementation Plan

* Affected paths: `crates/al-media`(フレームデコード)、
  `crates/al-analysis` または `al-media` の映像特徴算出
* Patterns to follow: `avi.rs` の demux/`StreamTiming`、
  `timing.rs` の観測モデル
* Patterns to avoid: 公称 FPS からの時刻推定、コーデック推測
* Dependencies or configuration: 新規依存なし

## Verification

* [ ] 非圧縮 AVI のフレームがデコードされ映像特徴が記録される
* [ ] 圧縮コーデック映像は明示失敗として記録される

## Pros and Cons of the Options

### 非圧縮 AVI のみ

* Good, because 決定的・依存なし・既存方針と一致。
* Bad, because 実用キャプチャの多くをカバーしない。

### 外部デコーダ

* Good, because 実用上のカバレッジが大きい。
* Bad, because 大きな外部依存・再現性リスク・承認プロセスが必要。

### 実装しない

* Good, because 変更不要。
* Bad, because issue #11 が達成できない。

## More Information

* GitHub issue #11
* `docs/discovery/video-av-feature-gaps.md`
* `crates/al-media/src/avi.rs`

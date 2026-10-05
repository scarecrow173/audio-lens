---
id: "DISC-034AajwySUrLTGaPkqCqe8"
type: "discovery"
status: "resolved"
title: "映像フレーム特徴と A/V 同期のギャップ"
created: "2026-10-04"
updated: "2026-10-04"
owners: []
relations:
  source:
    - "../specs/video-av-timing.md"
    - "../../crates/al-media/src/avi.rs"
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
    - "../designs/video-features.md"
    - "../adr/avi.md"
    - "../plans/video-av-features.md"
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# 映像フレーム特徴と A/V 同期のギャップ

## Exploration Goal

issue #11(video timing / A-V sync)の受け入れ基準と現在の実装を
照合した棚卸しの結果、映像フレーム内容に依存する特徴と音声↔映像
イベント同期機構が未実装である。何をどこまで実装すれば issue の
要求を満たせるかを確定する。

## Key Issues and Assumptions

- 現状 `al-media` は映像フレームを一切デコードしない
  (`avi.rs` は vids ストリームの宣言メタデータと mux 順序由来の
  タイムスタンプのみ記録。PR #22 で意図的スコープとして決定済み)
- 実装済み: フレーム位置タイムスタンプ(`StreamTiming`)、
  フレーム間隔系列、duplicate/drop/discontinuity 観測
  (`timing.rs` の `TimingObservation`)
- 未実装(issue #11 要求): scene-change タイムスタンプ、粗い
  luminance 変化系列、粗い motion/activity 系列、音声イベント↔
  映像イベントの汎用比較機構
- 前提: コンテナスコープは既存方針どおり非圧縮 AVI
  (DIB/rawvideo)に限定し、圧縮コーデックは引き続き明示失敗

## Alternatives and Comparison

| Option | Pros | Cons | Lean |
| --- | --- | --- | --- |
| 非圧縮 AVI(DIB/rawvideo)のみフレームデコード | 外部依存なし・既存コンテナスコープ内で完結・決定的 | MP4/H.264 等の実用キャプチャでは使えない | yes |
| FFmpeg 等で任意コーデック対応 | 実用キャプチャを網羅 | 大きな外部依存・決定性/再現性のリスク・依存追加は別途承認が必要 | no |
| フレーム特徴を実装しない | 変更不要 | issue #11 が永遠に未達 | no |

## Tentative Conclusions and Hypotheses

- confirmed: 非圧縮 AVI フレームデコード + scene-change /
  luminance / motion 系列で issue #11 の「cheap video features」
  を満たせる。圧縮コーデック対応は別 ADR/スコープ判断に分離する
- confirmed: A/V 同期は「音声イベント時刻・映像イベント時刻・差分
  ・確度・分布・ドリフト」を出力する汎用比較で足り、
  ゲーム固有の検出器をコアに入れない
- hypothesis: 映像特徴は Ingest 時点ではなく dataset の中間生成物
  として分析側で算出する方が責務がきれい(検討は design で)

## Open Questions

- 映像特徴の算出を `al-media`(decode 直結)と `al-analysis`
  (stage として分離)のどちらに置くか → design で決定
- A/V 同期の照合アルゴリズム(最近傍/窓内対応付け)の詳細
  → design で決定

## Promotion Candidates

- [x] spec-doc needed: 映像フレーム特徴と A/V 同期の仕様
  (video-av-timing spec が要求を網羅済みのため新規不要)
- [x] adr-doc needed: 非圧縮 AVI のみフレームデコードする
  スコープ決定(圧縮コーデックは外部依存判断として延期)

## Sources and Relations

- GitHub issue #11 (video-derived timing features)
- GitHub issue #3 (media ingest: コンテナスコープの親)
- `docs/specs/video-av-timing.md`(本 discovery が補完する既存 spec)
- `docs/adr/implementation-stack.md`(外部依存方針の文脈)
- `crates/al-media/src/avi.rs`(現行の映像スコープ注記)

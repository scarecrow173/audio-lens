---
id: "TSPEC-034AaE4bOFBcUsOjs95C7G"
type: "test-spec"
status: "proposed"
title: "映像フレーム特徴と A/V 同期の検証"
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
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related: []
  refines: []
  refined-by: []
  derives-from:
    - "docs/plans/video-av-features.md"
  derived-by: []
  verifies:
    - "docs/specs/video-av-timing.md"
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---


# 映像フレーム特徴と A/V 同期の検証

## Purpose

issue #11 の映像特徴と A/V 同期が、タイムスタンプベースで
正しく観測・照合されることを保証する。

## Feature

非圧縮 AVI の映像フレーム特徴と、音声↔映像イベント照合

## Rules

- Rule: 映像イベント時刻は公称 FPS 演算ではなく
  ストリームクロック由来のタイムスタンプで記録される
- Rule: シーンチェンジ・輝度・モーション系列が映像特徴として
  永続化される
- Rule: A/V 同期は音声イベントと映像イベントの差分・確度・
  分布・ドリフトを出力する
- Rule: 映像証拠がなくても音声解析は完結する
- Rule: 圧縮映像は明示失敗として記録される

## Examples

- Example: 既知シーンチェンジ位置を持つ合成 AVI
  → 対応時刻にシーンチェンジ候補が記録される
- Example: 既知 A/V オフセットを持つ合成素材
  → `avsync.delta` がそのオフセットを回復する
- Example: 圧縮コーデックを宣言する AVI
  → 映像特徴が `decode/input unavailable` になる

## Guarantees

- フレーム特徴がタイムスタンプ付きで記録される
- 音声解析が映像非依存で完結する

## Non-goals

- 圧縮コーデックのフレーム精度
- ゲーム固有検出器の正しさ

## Risk

- 偽のシーンチェンジや誤対応が、タイミング異常として
  誤読される

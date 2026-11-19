---
id: "TSPEC-034FPu2UHR9IyKz1LxreDD"
type: "test-spec"
status: "approved"
title: "Issue 33 正式保存仕様と維持契約"
created: "2026-10-14"
updated: "2026-10-14"
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
    - "docs/designs/spec-residual-contract-finalization.md"
  derived-by: []
  verifies:
    - "docs/specs/spec-residual-contract-finalization.md"
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# Issue 33 正式保存仕様と維持契約

## Purpose

今回の WAV のみへの正式仕様変更で、既存候補成果物・音声解析値・分解能契約が変わらないことを保証する。
上流仕様が置き換えられた場合に廃止を判断する。plan 承認範囲の具体化であり新しい動作要件はない。

## Feature

WAV の正式出力仕様と fused candidate stage の独立成果物。

## Rules

- Rule 1: 現行の残差・中間音声出力は WAV。Float64 PCM と lineage を保持する。
- Rule 2: 候補抽出 → 候補保存 → trigger → expensive の順序。空候補を保存し未実行と区別する。
- Rule 3: Resolution.sample_rate は解析 PCM レート。欠落は拒否し旧成果物は再生成する。
- Rule 4: 現在契約と旧 FLAC defer を日付で区別する。manifest/resume/StageKind 境界は変えない。

## Examples

既存 clipping / suppressed / manual ranges の fixture で候補保存・trigger 抑制・cheap 非破壊性を検証。
既存 residual_child fixture で保存前後の Float64 サンプルと lineage の完全一致を検証。
32/44.1 kHz cheap、8/32 kHz expensive、有効/無効値、44.1 → 48 kHz 正規化、JSON 欠落拒否を既存回帰で確認。
空候補保存は既存 manual ranges fixture の table 読出し結果と artifact を確認する。

## Guarantees

spec AC-1〜AC-4 を守る。動的検証の assert 範囲と静的コード確認を区別して記録する。

## Non-goals

FLAC encode、依存追加、#36 入力 decode、#52 plan 再利用、#55 耐久性、性能比較。

## Risk

単なるテスト成功を空候補保証と同一視しない。既存 assert 不足時は不足を記録し scope を再 review する。

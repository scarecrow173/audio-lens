---
id: "ADR-034FP1KCQ5PnMOE20EmIjo"
type: "adr"
status: "accepted"
title: "候補成果物を独立保存し音声出力を WAV に限定する"
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
  related:
    - "../specs/spec-residual-contract-finalization.md"
  refines: []
  refined-by: []
  derives-from:
    - "../discovery/spec-residual-contract-finalization.md"
  derived-by: []
  verifies: []
  verified-by: []
  references:
    - "analysis-dataset-storage.md"
    - "residual-issue-boundaries.md"
  defers: []
  deferred-by: []
metadata:
  adr:
    decision-makers: []
    consulted: []
    informed: []
---

# 候補成果物を独立保存し音声出力を WAV に限定する

## Context and Problem Statement

#33 の旧判断は candidate detection の fused 実行と FLAC defer、sample_rate の実装。
2026-10-14 にユーザーは fused を維持し、FLAC 保存を将来要件として残さず WAV のみを
正式仕様にすることを確定した。この ADR は storage ADR の音声保存部分だけを具体化する。
旧 ADR 全体や無関係な保存方式を supersede しない。今回の判断を別レビューで承認する。

## Decision Drivers

- 実装済み機能を正式仕様で正確に説明する。
- 候補データの独立参照と候補0件の観測可能性を失わない。
- 新依存や実測根拠のない圧縮機能を追加しない。
- 解析値・来歴・resume 境界を維持する。

## Considered Options

- fused 実行 + WAV のみへの仕様改訂（今回選択）。
- CandidateDetection の独立 StageKind と FLAC エンコーダを追加。
- fused 実行と FLAC defer の旧仕様を維持。

## Decision Outcome

fused 実行と独立 candidate artifact、WAV のみを採用する。
候補抽出 → 空を含む events/candidates 保存 → トリガ評価 → expensive 解析の順序と、
Float64 WAV のサンプル値・provenance を維持する。#36 の FLAC 入力デコードとは独立。

### Consequences

新しい実装依存や stage/resume 境界変更を伴わない。
圧縮による容量削減は提供しない。圧縮が必要になれば実測を根拠に別 Issue で判断し、
浮動小数 PCM の量子化による解析精度損失を採用条件に含める。

### Confirmation

spec の AC-1〜AC-4 に文書変更と既存契約検証を対応付ける。
旧承認を流用せず、review-protocol の今回レビューで Blocker がなくなれば承認扱いとする。
修正可能な非 Blocker も修正する。

## Implementation Plan

1. 今回 spec/ADR をレビューする。
2. 既存 overview / dataset / candidate spec と storage ADR の現在契約を更新し、
   residual-compliance / residual-fixes / 既存taskには日付付き変更履歴を残す。
3. 既存の候補テーブル・sample_rate・残差 WAV 精度/lineage 検証を対象限定で実行する。
4. 文書監査・関連indexを更新し、今回の変更を論理単位で commit する。

新依存・StageKind・互換層・コード変更は予定しない。
契約の実装欠落が検証で判明した場合は docs-only 完了とはせず、Graph の修正 route と
今回 review を経て必要な変更範囲を明示する。

## Verification

- [ ] spec AC-1〜AC-4 の確認。
- [ ] spec / ADR の文書監査と git diff --check が成功。
- [ ] 今回 review-protocol による承認証拠を記録。

## Pros and Cons of the Options

fused + WAV は現行動作と一致し、契約と履歴を明確化できる。
stage 分離 + FLAC 実装は独立実行や圧縮を提供するが、今回の要求外で新依存が必要。
旧 defer の維持は変更が不要だが、今回確定した WAV のみの正式仕様と一致しない。

## More Information

既存 task の 2026-10-11 判断は履歴として維持する。
今回の適用範囲は #33。#52 や #55 の実装契約を変更しない。

## 2026-10-14 判断レビュー

今回 coordinator が独立 Reviewer A / Reviewer B、Validator、Judge の結果を集約した。
A/B の candidates は各0件、Validator の追加 findings は0件、Judge は ACCEPT。
承認対象は spec/ADR の判断と文書関係。実装 AC や旧仕様への適用、維持契約の動的検証を
承認済みと扱わない。ユーザー指定に従い review-protocol の Blocker なしで承認扱いとした。

Graph の旧 overview/roadmap への lineage 接続は複数既存 plan を巻き込み focus-required を
発生させたため、今回変更だけの discovery を共通 lineage anchor とする。旧契約は
references と本文の対象パスで保持し、旧計画の status と完了履歴を変更しない。
この関係調整は次の design checkpoint の review 対象に含める。

---
id: "SPEC-034FP1KsTCa0WviopfvrnQ"
type: "spec"
status: "approved"
title: "Issue 33 保存仕様と候補成果物の確定"
created: "2026-10-14"
updated: "2026-10-14"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/audio-lens/issues/33"
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
    - "../adr/spec-residual-contract-finalization.md"
    - "../tasks/spec-residual-decisions.md"
  refines: []
  refined-by: []
  derives-from:
    - "../discovery/spec-residual-contract-finalization.md"
  derived-by: []
  verifies: []
  verified-by: []
  references:
    - "audiolens-overview.md"
    - "analysis-dataset.md"
    - "candidate-event-model.md"
  defers: []
  deferred-by: []
---

# Issue 33 保存仕様と候補成果物の確定

## What Should Be Built

2026-10-14 のユーザー確定事項に基づき、#33 の現行実装と正式仕様を一致させる。
候補データの独立性を維持し、残差・中間音声の出力形式は WAV のみとする。

## Why Now

2026-10-11 の判断は FLAC 保存を将来要件として defer した。今回の判断は
WAV のみへの仕様改訂であり、その判断記録を消さず追加の変更として扱う。
前回の承認・実装済み task は今回の承認を代替しない。今回の判断を別レビューで承認する。

## Users and Value

音声デバッグの利用者と開発者が、候補なしと未実行を区別し、実際に利用できる
音声保存形式と精度保持契約を読み取れる。

## Scope

対象は overview / analysis-dataset / candidate-event-model、関連する storage ADR と
residual-compliance / residual-fixes の判断・設計・検証記録の文言更新。
新しい StageKind、manifest/resume 境界変更、FLAC エンコーダ追加、sample_rate の再実装は行わない。
#36 の入力デコード、#52 の plan/workspace、#55 の耐久性変更とは別の変更単位とする。

## Requirements

- CandidateDetection の独立 StageKind を追加しない。ExpensiveAnalysisStage 内で
  候補抽出 → events/candidates 保存 → トリガ評価 → Expensive Analysis を実行する。
- events/candidates は解析結果から独立に参照できる構造化成果物として保持する。
  候補0件でも空テーブルを永続化し、候補なしと未実行を区別する。
- 残差・中間音声の出力は既存 WAV のみ。Float64 WAV の解析サンプル値と来歴を維持する。
  FLAC 保存を現行要件として残さず、FLAC エンコーダ依存を追加しない。
- Resolution.sample_rate は解析対象 PCM の実レートを表す既存必須 u32 契約を維持する。
  旧成果物は再生成し、互換層を追加しない。

## Acceptance Criteria

- [ ] AC-1: overview、dataset、storage ADR が現行の正式出力形式を WAV のみと定義し、
  過去の FLAC defer 記録は日付付き履歴として保持する。
- [ ] AC-2: candidate-event-model と overview が上記の順序・独立成果物・空保存を定義する。
  StageKind と manifest/resume 境界を変更しない。
- [ ] AC-3: 既存の候補テーブル・残差 WAV lineage・sample_rate 回帰検証を実行し、
  維持する契約を確認する。実行した検証と静的確認を区別して記録する。
- [ ] AC-4: 関連文書の現在の判断と旧判断の履歴を区別し、文書監査と diff check が成功する。

## Deferred Design Concerns

候補だけの実行・再実行が実需要になった場合に stage 分離を再検討する。
保存容量・保存時間・I/O 性能の実測が問題になった場合に、圧縮形式を別 Issue で検討する。
浮動小数 PCM の量子化損失を含め、解析に必要な精度を損なわないことを将来採用条件とする。

## Sources and Relations

ロードマップの確定決定とユーザー引継ぎが要件入力。
既存 TASK-034dLdqYw5JFCrkMD5dSPk は sample_rate 実装と旧判断の証拠であり、
今回の WAV のみへの正式仕様変更の完了証拠ではない。

## 2026-10-14 判断レビュー

今回 coordinator が独立 Reviewer A / Reviewer B、Validator、Judge の結果を集約した。
A/B の candidates は各0件、Validator の追加 findings は0件、Judge は ACCEPT。
承認対象は spec/ADR の判断と文書関係。実装 AC や旧仕様への適用、維持契約の動的検証を
承認済みと扱わない。ユーザー指定に従い review-protocol の Blocker なしで承認扱いとした。

Graph の旧 overview/roadmap への lineage 接続は複数既存 plan を巻き込み focus-required を
発生させたため、今回変更だけの discovery を共通 lineage anchor とする。旧契約は
references と本文の対象パスで保持し、旧計画の status と完了履歴を変更しない。
この関係調整は次の design checkpoint の review 対象に含める。

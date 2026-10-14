---
id: "DESIGN-034FPeXgGtwiUXHI98tTsU"
type: "design"
status: "approved"
title: "Issue 33 確定仕様の文書適用設計"
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
    - "docs/specs/spec-residual-contract-finalization.md"
    - "docs/adr/spec-residual-contract-finalization.md"
  derived-by: []
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# Issue 33 確定仕様の文書適用設計

## Context and Scope

approved spec と accepted ADR の AC-1〜AC-4 を docs-only 変更で満たす。
旧 done task と 2026-10-11 の判断履歴は維持。新依存・stage/resume/schema 変更はない。

## Components and Boundaries

| 対象 | 適用内容 |
| --- | --- |
| specs/audiolens-overview.md、analysis-dataset.md | 現行の残差・中間音声出力を WAV のみとする。Float64 の解析値・lineage を維持 |
| specs/candidate-event-model.md、overview | 候補抽出→空を含む候補保存→trigger→expensive の fused 順序と独立成果物を明記 |
| adr/analysis-dataset-storage.md | 現行の選択・検証欄を WAV に整合。旧 WAV/FLAC 選択は日付付き履歴に残す |
| specs/residual-compliance.md、adr/residual-issue-boundaries.md、designs/residual-fixes.md、test-specs/residual-fixes.md | 現在判断を今回spec/ADRへ参照。旧 FLAC defer を歴史的判断として区別 |
| tasks/spec-residual-decisions.md | status doneを保持。旧完了記録を書き換えず、今回変更を追記して新taskへ参照 |
| 新規plan/task/test-spec、impl recordと各index | 今回適用・AC検証・今回review証拠を最小1件ずつに集約 |

## Data and Control Flow

候補抽出と候補データ保存の既存境界を保持。StageKind を追加せず、
候補0件でも events/candidates が存在し、未実行時の欠落と区別する。
Resolution.sample_rate は解析 PCM レートの必須 u32 を維持。
Float64 WAV は量子化しない既存保存を維持し、FLAC encode は追加しない。

## Verification Design

文書検証は上表の現在契約・旧履歴の区別、関連リンクとindex、diff check。
維持契約の動的確認は既存 tests/expensive.rs（候補・trigger/非破壊性）、
cheap::windowed_records_persist_the_decoded_sample_rate、dataset record serialization、
locate::residual_child_reopens_the_exact_subtracted_signal_and_lineage を再実行する。
空候補の既存検証が不足すれば、コードの静的確認を動的保証と混同せず不足を報告し、
必要な最小の契約テストを別review対象にする。docs-only変更を証明するための実装模倣テストは追加しない。

## Risks and Trade-offs

旧 decision history を現在の契約と誤読しないよう、旧記述に日付・新契約リンクを付ける。
音声全体では decoded.f64le も使われるため、WAVのみは dataset の残差・中間音声
出力形式に限定し、#56 の正本 PCM artifact を WAV へ置換する意味にしない。

## Graph Focus Boundary

今回 discovery/spec/ADR/design/plan/task を一意なchainにする。旧 overview と roadmap は
references で履歴・対象を保持する。旧lineage全体を refines に含めると旧複数planに
接続され focus-required となるため、本 chain が旧chainの実行状態を取り込まない。

## Future Work

候補only実行は実需要発生時、圧縮は容量・保存時間・I/O実測発生時に別Issueで判断。
浮動小数 PCM 精度・量子化損失を将来の圧縮形式の採用条件とする。

## 2026-10-14 Design Review

独立 Reviewer A/B の候補は各0、Validator の追加 findings は0、diff check と契約関係確認を経て
Judge は ACCEPT。coordinator は今回設計のみ承認と通知した。動的 AC は未検証。
spec/ADR の専用 discovery anchor への関係調整も今回 review 範囲として確認済み。

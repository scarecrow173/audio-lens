---
id: "TASK-034dLdqYw5JFCrkMD5dSPk"
type: "task"
status: "done"
title: "spec #1 との残差適合項目: 候補検出ステージ独立性、FLAC、Resolution.sample_rate"
created: "2026-10-10"
updated: "2026-10-11"
owners: ["scarecrow173"]
relations:
  source: []
  implements:
    - "docs/plans/residual-fixes.md"
  implemented-by: []
  depends-on:
    - "docs/plans/residual-fixes.md"
    - "docs/designs/residual-fixes.md"
    - "docs/adr/residual-issue-boundaries.md"
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
    - "docs/test-specs/residual-fixes.md"
  references:
    - "https://github.com/scarecrow173/audio-lens/issues/33"
  defers: []
  deferred-by: []
---

# spec #1 との残差適合項目: 候補検出ステージ独立性、FLAC、Resolution.sample_rate

## Work

#33 の残件を以下の原文棚卸しに沿って確認する。Issue 全体を対応単位とし、修正・文書化・承認済み defer の各結果と証拠を項目ごとに残す。複数の独立修正を実施するときは承認後に task を分割し、この対応を保持する。

## Classification

- `decision-required` (2026-10-11 に三方式を承認済み。末尾の判断記録を参照)
- draft plan のレビュー用候補。依頼により先に文書化するが、通常の承認済み計画由来の実装 task ではない。

## Upstream Documents

- `docs/plans/residual-fixes.md`
- `docs/specs/residual-compliance.md` AC-33
- `docs/designs/residual-fixes.md`
- Issue #33

## Dependencies

spec/ADR はチェーンの受理可能な status を満たし、design/plan と test-spec は承認済みのため status todo。

三項目それぞれの選択肢と承認結果を先に記録する。依存追加が必要なら許可を得る。仕様改訂を選ぶ場合も利用者価値・理由・対応 AC を review し、既存仕様を黙って緩和しない。

## Affected Files

- `docs/specs/audiolens-overview.md`
- `docs/specs/analysis-dataset.md`
- `docs/specs/candidate-event-model.md`
- `crates/al-analysis/src/stage.rs`
- `crates/al-dataset/src/audio.rs`
- `crates/al-dataset/src/types.rs`

## Issue Inventory (2026-10-10)

### 概要

#1(spec)の実コード再照合で確認した、spec 文言への厳密適合上の
残差項目 3 点。いずれも機能的欠落ではなく、機会があれば対応する
低優先項目。

### 1. 候補検出が独立ステージでない

spec のパイプラインは `Reference Analysis → Candidate Detection →
Expensive Analysis` の独立段。実装では `ExpensiveAnalysisStage`
内で候補抽出(`events/candidates` テーブル)→トリガ評価を fused
実行している(`crates/al-analysis/src/stage.rs`)。

順序・成果物は正しいが、「候補抽出のみ実行/再実行」ができない。
独立 StageKind を設けるか、現状を仕様として文書化するかは要判断。

### 2. FLAC エンコーディング未実装

音声中間/残差の保存は WAV のみ(`crates/al-dataset/src/audio.rs`
`WavEncoding`)。spec は "WAV/FLAC as appropriate"。大容量残差・
中間音声を多数保存するワークロードでは FLAC 圧縮が有利。

### 3. `Resolution` に sample_rate フィールドがない

`al-dataset::types::Resolution` は window/hop のみ。sample_rate は
各 record の params や manifest 経由で永続化されており値は失われ
ないが、スキーマの第一級フィールドではない。周波数軸の再解釈時に
params を掘る必要がある。

### 参照

- 監査元: issue #1
- 実装: crates/al-analysis/src/stage.rs, crates/al-dataset/src/audio.rs,
  crates/al-dataset/src/types.rs

## Steps

1. 原文の各項目を実装と既存 spec に照合し、修正か判断かを記録する。
2. 違反する契約を固定する回帰テストを追加する。
3. 対象テストを実行し、期待する失敗を確認する。
4. 所有境界で修正する。判断項目は承認後に着手する。
5. 対象検証を再実行する。
6. 項目ごとの証拠と文書を更新する。
7. 意味のある変更単位でコミットする。

## Done When

- [x] 候補検出の独立実行、FLAC 保存、Resolution.sample_rate の各項目に、既存 spec の該当文言、実装との差、選択肢、承認結果、実装または仕様改訂先を記録する。判断前に現状を適合済みとせず、選択した契約の回帰テストを承認後に追加する。
- [x] 原文全項目に修正・文書化・承認済み defer と検証証拠の対応がある。
- [x] 関連 spec/ADR/design/test-spec が実施内容と一致する。

## Verification

- [x] `mise exec -- cargo test --workspace` が exit 0。原文項目に対応した回帰テストを含む。
- [x] `mise exec -- cargo fmt --all -- --check` と `mise exec -- cargo clippy --workspace --all-targets -- -D warnings` が exit 0。
- [x] 文書と Issue 項目の対応を review する。

対象回帰は変更前の失敗と変更後の成功を確認した。最終 workspace 検証は下記に記録する。

## 選択肢と事前承認 (2026-10-11)

Orca dispatch `ctx_5ed7abc7cd10` の coordinator への事前質問で、三項目の
以下の選択と task 分割不要について明示承認を得た。承認前の現状を
適合済みとはせず、承認後に契約テスト・実装・仕様更新を行った。

| 原文項目 / 既存 spec 文言 | 最新実装との差 | 比較した選択肢 | 承認結果 / 利用者価値と理由 | 対応先と証拠 |
| --- | --- | --- | --- | --- |
| 1: audiolens-overview の `Reference Analysis → Candidate Detection → Expensive Analysis` 独立段 | stage.rs の ExpensiveAnalysisStage が候補生成・保存・トリガ評価を fused 実行し、候補のみ独立再実行できない | 独立 StageKind 追加 / 論理段として現行順序を仕様化 / defer | **仕様改訂を承認**。観測・候補保存と計算抑制は維持し、候補のみ再実行の追加 API は要求しない。利用者が高コスト計算を抑制しても候補と来歴を得られる契約を明確化 | overview と candidate-event-model を改訂。expensive 統合テストで candidate_events の先行・cheap 非破壊・観測への provenance・抑制時保存を assertion |
| 2: overview と analysis-dataset の `WAV / FLAC` | audio store は WAV のみ。#32 は減算後残差を Float64 WAV と provenance で保持済み | FLAC 保存実装（依存・解析値保持方式の選定が必要）/ WAV のみへの仕様縮小 / FLAC を承認済み defer | **defer を承認**。新 codec 依存を追加しない。圧縮の価値は保存容量と I/O 削減だが、残差の浮動小数解析値保持を損なわない方式の review と実測が必要。FLAC 要件を削除せず、未実装を適合済みにしない | overview / analysis-dataset / storage ADR に現行 WAV と defer を明記。#32 の residual_child_reopens_the_exact_subtracted_signal_and_lineage を再実行し、再取得信号と lineage の契約を確認 |
| 3: analysis-dataset の `分解能 / window / hop` と Issue #33 の第一級 sample_rate 提案 | Resolution は window/hop のみで、解析レートは params/manifest に分散 | 第一級 u32 追加 / 既存参照方式を仕様化 / defer | **第一級 u32 実装を承認**。窓の時間換算・周波数軸再解釈に解析対象 PCM レートを直接使える。原入力レートと正規化レートを混同しない。旧成果物は pre-release 方針に従い再生成 | al-dataset::Resolution / RecordBuilder / cheap / reference / band_gain / expensive Emit と analysis-dataset spec を更新。32/44.1 kHz cheap、8/32 kHz expensive 有効・無効、原入力44.1→解析48 kHz reference/expensive、JSON round trip/欠落拒否を検証 |

対応 AC は三項目とも residual-compliance **AC-33**。candidate-event-model の
元特徴非破壊・観測リンクと analysis-dataset の生成経緯追跡も維持する。
residual-issue-boundaries ADR、residual-fixes design/test-spec に選択を記録し、
上流文書の status は変えていない。独立実装修正は sample_rate のみなので
task 分割は不要という判断も承認済み。

## 契約回帰の証拠

- 変更前: `mise exec -- cargo test -p al-analysis windowed_records_persist_the_decoded_sample_rate`
  は `level.trajectory` の sample_rate が Null（期待32000）で失敗。
- 変更前: `mise exec -- cargo test -p al-analysis --test reference normalized_alignment_survives_process_boundary`
  は `ref.band_gain_m2` の sample_rate が Null（期待48000）で失敗。
- 変更後: 上記の両対象テストが exit 0。原入力レートではなく解析 PCM の
  正規化後レートを保存する。全生成箇所は同じ Resolution 型を使う。
- 候補段の統合テスト 6 件と #32 の残差再取得テスト 1 件は着手時に exit 0。
  承認後、候補段には先行 artifact と provenance の assertion を追加した。
- `feature_record_round_trips` は sample_rate の保持と必須フィールド欠落の
  deserialize エラーを固定し、旧成果物を黙って補完しない。

## FLAC defer の再検討条件と残る制約

大量の残差/中間音声保存で容量・I/O が支配的と実測された時に再検討する。
その際は Float64 残差の解析値保持（量子化するなら別 artifact と明示）、
codec 依存、read/write 境界と来歴を review し、依存追加の明示承認を得る。
現行 WAV は RIFF サイズ上限を超えるとエラーになる。FLAC や大容量
コンテナへの暗黙 fallback は追加していない。候補のみの再実行が必要なら
独立 StageKind を別途提案する。これらは本 task の承認済み判断により残す
制約であり、本 task で FLAC 実装完了を主張しない。

## 最終検証 (2026-10-11)

- `mise exec -- cargo test --workspace`: exit 0。追加・更新した契約テストを
  含め workspace 全体と doc-tests が成功した。
- `mise exec -- cargo fmt --all -- --check`: exit 0。
- `mise exec -- cargo clippy --workspace --all-targets -- -D warnings`: exit 0。
- `git diff --check`: exit 0。
- 変更文書のローカル Markdown リンク存在確認: 成功。
- 原文三項目と選択肢・承認・実装/仕様改訂/defer・回帰証拠を対応表で review。
  task status と tasks/README.md index を done に同期した。

新 codec 依存や Cargo.lock の変更はない。#32 の既存残差 PCM 永続化を
前提として再検証し、音声 store 自体の再実装は行っていない。

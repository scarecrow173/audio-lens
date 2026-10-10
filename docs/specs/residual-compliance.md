---
id: "SPEC-034DlyJKMTXtsFFjEVd9w7"
type: "spec"
status: "proposed"
title: "残存レビュー指摘と仕様適合の追跡"
created: "2026-10-10"
updated: "2026-10-11"
owners: ["scarecrow173"]
relations:
  source:
    - "https://github.com/scarecrow173/audio-lens/issues/28"
    - "https://github.com/scarecrow173/audio-lens/issues/29"
    - "https://github.com/scarecrow173/audio-lens/issues/30"
    - "https://github.com/scarecrow173/audio-lens/issues/31"
    - "https://github.com/scarecrow173/audio-lens/issues/32"
    - "https://github.com/scarecrow173/audio-lens/issues/33"
  implements: []
  implemented-by: []
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/adr/residual-issue-boundaries.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/adr/residual-issue-boundaries.md"
  derived-by:
    - "docs/designs/residual-fixes.md"
  verifies: []
  verified-by: []
  references:
    - "docs/specs/audiolens-overview.md"
    - "docs/specs/media-ingest.md"
    - "docs/specs/expensive-analysis.md"
    - "docs/specs/video-av-timing.md"
    - "docs/specs/locate-tracking.md"
    - "docs/specs/analysis-dataset.md"
    - "docs/specs/candidate-event-model.md"
    - "docs/specs/validation-strategy.md"
  defers: []
  deferred-by: []
---

# 残存レビュー指摘と仕様適合の追跡

## What Should Be Built

レビューと仕様監査で残った #28〜#33 を、観測・来歴・妥当性の契約に対応付け、実装修正と仕様判断を追跡する。

## Why Now

部分 ingest、expensive family、fixture のレビュー指摘が Issue に留まり、#31 の marker 入力と #32 の再聴取可能な残差も未完了である。#33 は機能欠落と仕様表現の差を区別して判断する必要がある。

## Users and Value

- 解析者: 利用不能と有効な測定を区別し、時刻・チャネル・残差の証拠を追跡する。
- 実装者とレビュアー: 各 Issue の未解決項目、責務、検証と承認待ちを辿る。

## Scope

### In Scope

#28〜#33 の項目棚卸し、契約に沿った修正、#31/#32 の欠落経路、#33 の仕様判断。

### Out of Scope

SIMD 高速化(#37/#40/#42/#43)、デコーダ拡張(#36)、ROI(#39)、経路校正(#41)。新しい依存導入や未承認の仕様変更も本提案では決定しない。

## Requirements

既存仕様を正本として修正境界を定める。同一概念の表現は責務を持つ境界で集約する。外部入力を境界で検証し、下流で情報を補償・捏造しない。未計測は Valid な 0 に置き換えない。

## Acceptance Criteria

- [ ] AC-28: #28: 未整列 AVI 音声パケットの時刻が累積有理時計と一致する。MP4 の非デコードトラックにも同じテーブル検証が適用される。音声が unavailable でも入力ハッシュと利用不能理由が残る。luma の再利用、非フレーム時刻、dwStart、OpenDML、integrity 名称、二次音声、欠落 timeline の扱いを各境界ケースで検証する。
- [ ] AC-29: #29: 不均一チャネル長で panic とチャネル誤ラベルが起きない。ill-conditioned 帯域の係数にも妥当性が反映され、支持数ゼロや候補なしの測定を NaN/0 + Valid として出さない。IR の窓、ペア summary のチャネル意味、暗黙スキップとコメントの整合を fixture で検証する。
- [ ] AC-30: #30: 奇数 LIST、stream 99/100、index フラグ、DIB stride と rcFrame が生成するメディアの契約と一致する。周期 fixture は全拒否で成功しない。失敗時も一時ディレクトリが回収され、合成 source hash の意味と defer 項目の受け皿が明記される。
- [ ] AC-31: #31: ユーザー供給 visual marker が avsync のペア出力に反映される。未指定時の検出イベント経路と出力は変化しない。有限時刻・クロック・入力来歴を検証し、ゲーム固有 detector を core に追加しない。
- [ ] AC-32: #32: two-source 探索の残差 PCM を保存して再読出し・再聴取できる。search JSON から音声 artifact と親 source/alignment、混合行列の来歴を辿れ、保存前後のサンプル・エネルギーが一致する。
- [x] AC-33: #33: 候補検出の独立実行、FLAC 保存、Resolution.sample_rate の各項目に、既存 spec の該当文言、実装との差、選択肢、承認結果、実装または仕様改訂先を記録する。判断前に現状を適合済みとせず、選択した契約の回帰テストを承認後に追加する。

## Existing Spec Alignment

#33 の候補検出順序は audiolens-overview の独立段に由来し、候補イベントの情報保存は candidate-event-model に従う。WAV/FLAC は audiolens-overview と analysis-dataset に明記される。sample_rate の第一級化は Issue #33 の提案であり、analysis-dataset の分解能要件を補足する判断が必要。本 spec はこれらを削除・弱めず、実装か仕様改訂かの承認を要求する。

#31 は video-av-timing のユーザーマーカー例を具体化する。#32 は locate-tracking の減算来歴と analysis-dataset の中間物保持を補足する。#30 の既存 defer は今回勝手に実装対象へ戻さず、項目・理由・再検討条件を対応付ける。

## Deferred Design Concerns

#33 の方式選定と #30 の defer 再開はレビュアーの判断を要する。対応 task で記録し、判断後に正本 spec/ADR/design を更新してから実装する。

## Sources and Relations

2026-10-10 に取得した open Issue #28〜#33 が棚卸しの証拠。既存仕様への references と残件処理 ADR への related を記録する。status proposed は提案提出であり承認を意味しない。

## AC-33 の実施判断 (2026-10-11)

三方式は coordinator の事前承認を得て、候補検出の論理段/fused 実行の
仕様改訂、FLAC 保存の承認済み defer、Resolution.sample_rate の第一級 u32
実装に対応させた。FLAC を実装済み適合とは扱わない。
[task の項目別判断と証拠](../tasks/spec-residual-decisions.md) を正本として参照する。

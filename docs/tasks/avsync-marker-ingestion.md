---
id: "TASK-034dLdqYq4GSy3E3kSqOzG"
type: "task"
status: "done"
title: "[Video] ユーザー指定 visual marker の取り込み経路(issue #11 の残存)"
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
    - "docs/tasks/container-review-edge-cases.md"
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
    - "https://github.com/scarecrow173/audio-lens/issues/31"
  defers: []
  deferred-by: []
---

# [Video] ユーザー指定 visual marker の取り込み経路(issue #11 の残存)

## Work

#31 の残件を以下の原文棚卸しに沿って確認する。Issue 全体を対応単位とし、修正・文書化・承認済み defer の各結果と証拠を項目ごとに残す。複数の独立修正を実施するときは承認後に task を分割し、この対応を保持する。

## Classification

- `new-feature`
- 実装依頼と coordinator による入力位置・時計契約・CLI 補完の承認に基づき実装済み。

## Upstream Documents

- `docs/plans/residual-fixes.md`
- `docs/specs/residual-compliance.md` AC-31
- `docs/designs/residual-fixes.md`
- Issue #31

## Dependencies

spec/ADR はチェーンの受理可能な status を満たし、design/plan と test-spec は承認済みのため status todo。

#28 の時計・dwStart 意味論を確認後にイベント差分を検証する。#39 ROI task は本 marker 入力の代替ではない。

## Affected Files

- `crates/al-analysis/src/avsync.rs`
- `crates/al-analysis/src/stage.rs`
- `crates/al-analysis/tests/avsync.rs`
- `crates/al-cli/src/`

## Issue Inventory (2026-10-10)

Parent: #1 / Refs #11

### 背景

issue #11 の A/V 同期要求は検出イベント(SceneChange / LuminanceStep / MotionOnset)を `avsync.match_events` に流す機構として実装済みだが、「user-provided visual marker ↔ audio event」の比較例として挙げられた**ユーザー供給マーカーの取り込み経路**は未実装。

`AvSyncConfig` は `match_window_s` のみを持ち、マーカー系列を job config や別入力から読む経路が存在しない。

### 提案

- job config(`analysis.avsync.markers` 等)または marker 入力 role から、ユーザー供給の映像側タイムスタンプ列を受け取る
- 供給されたマーカーも `VideoEventS` に変換して `match_events` に合流させる(検出イベントと同じ stream-clock 前提、または wall-clock で明示)
- スキーマは汎用のまま(ゲーム固有 detector を core に埋め込まない issue #11 の原則を維持)

### Acceptance

- [x] ユーザー指定マーカーが `avsync.*` のペア出力に反映される
- [x] マーカー未指定でも既存経路が変わらない

## Steps

1. 原文の各項目を実装と既存 spec に照合し、修正か判断かを記録する。
2. 違反する契約を固定する回帰テストを追加する。
3. 対象テストを実行し、期待する失敗を確認する。
4. 所有境界で修正する。判断項目は承認後に着手する。
5. 対象検証を再実行する。
6. 項目ごとの証拠と文書を更新する。
7. 意味のある変更単位でコミットする。

## Done When

- [x] ユーザー供給 visual marker が avsync のペア出力に反映される。未指定時の検出イベント経路と出力は変化しない。有限時刻・クロック・入力来歴を検証し、ゲーム固有 detector を core に追加しない。
- [x] 原文全項目に修正・文書化・承認済み defer と検証証拠の対応がある。
- [x] 関連 spec/ADR/design/test-spec が実施内容と一致する。

## Verification

- [x] `mise exec -- cargo test -p al-analysis --test avsync` が exit 0。原文項目に対応した回帰テストを含む。
- [x] `mise run fmt` と `mise run lint` が exit 0。
- [x] 文書と Issue 項目の対応を review する。

2026-10-11 に実装検証を実行し、すべて exit 0 を確認した。

## 実施結果と証拠 (2026-10-11)

| 原文項目 | 結果 | 実装・検証根拠 |
| --- | --- | --- |
| job config または入力 role で受け取る | 修正 | analysis.avsync.markers を採用、AvSyncConfig::from_job と CheapAnalysisStage。CLI は承認済み --input-ids で安定 ID を与える |
| VideoEventS に変換して合流、時計明示 | 修正 | clock=video_stream 必須、dwStart を含む秒から audio_start_s を引き match_events へ追加。四組の stream 原点 fixture と同時照合単体テスト |
| 汎用スキーマ | 修正 | input_id/clock/time_s/frame と user_marker 種別のみ。ゲーム固有 detector は追加しない |
| 未指定時の既存経路・出力保持 | 修正・検証 | 既存 AVI/ROI/WAV の回帰テストと、未指定/空配列の全レコード一致単体テスト |
| 有限時刻・時計・入力来歴 | 修正・検証 | config 境界の型/有限値検証、未知入力と decoded frame 時計の検証。params.marker に config_path/input_id/clock/time_s、event_frame と既存 artifact lineage を保存 |
| spec/ADR/design/test-spec との整合 | 文書化 | residual-compliance AC-31 を充足、residual-fixes design/test-spec に承認済み契約とテストを追記。ゲーム固有 detector を core に置かない既存 ADR の境界を維持 |

追加 defer はない。config 入力を選んだため新規 marker role/外部ファイルは追加しない。
承認済み schema と詳細は [採用設計](../designs/residual-fixes.md#31-の採用設計-2026-10-11) を参照する。

### 検証実績

- mise exec -- cargo test -p al-analysis --test avsync: 7 passed。
- mise exec -- cargo test -p al-analysis avsync: avsync 単体 8 passed、名前一致の ROI 統合 1 passed。
- mise exec -- cargo test -p al-cli --test cli: 14 passed(CLI マーカー経路・ID 検証を含む)。
- mise run fmt: workspace cargo fmt --all -- --check、exit 0。
- mise run lint: workspace/all-targets clippy -D warnings、exit 0。
- git diff --check: exit 0。
- 赤テスト: user_marker pair 欠落と CLI 指定 input_id の未反映を実装前に確認した。

workspace 全体の test suite は実行していない。fmt/clippy は workspace 全体を検証した。

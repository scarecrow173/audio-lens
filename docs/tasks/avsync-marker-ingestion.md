---
id: "TASK-034dLdqYq4GSy3E3kSqOzG"
type: "task"
status: "todo"
title: "[Video] ユーザー指定 visual marker の取り込み経路(issue #11 の残存)"
created: "2026-10-10"
updated: "2026-10-10"
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
- draft plan のレビュー用候補。依頼により先に文書化するが、通常の承認済み計画由来の実装 task ではない。

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

- [ ] ユーザー指定マーカーが `avsync.*` のペア出力に反映される
- [ ] マーカー未指定でも既存経路が変わらない

## Steps

1. 原文の各項目を実装と既存 spec に照合し、修正か判断かを記録する。
2. 違反する契約を固定する回帰テストを追加する。
3. 対象テストを実行し、期待する失敗を確認する。
4. 所有境界で修正する。判断項目は承認後に着手する。
5. 対象検証を再実行する。
6. 項目ごとの証拠と文書を更新する。
7. 意味のある変更単位でコミットする。

## Done When

- [ ] ユーザー供給 visual marker が avsync のペア出力に反映される。未指定時の検出イベント経路と出力は変化しない。有限時刻・クロック・入力来歴を検証し、ゲーム固有 detector を core に追加しない。
- [ ] 原文全項目に修正・文書化・承認済み defer と検証証拠の対応がある。
- [ ] 関連 spec/ADR/design/test-spec が実施内容と一致する。

## Verification

- [ ] `mise exec -- cargo test -p al-analysis --test avsync` が exit 0。原文項目に対応した回帰テストを含む。
- [ ] `mise run fmt` と `mise run lint` が exit 0。
- [ ] 文書と Issue 項目の対応を review する。

現時点では文書整備のみであり、上記実装検証は未実行。

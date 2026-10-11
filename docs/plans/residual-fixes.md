---
id: "PLAN-034dLdqZ28TeDZBM9XH3rg"
type: "plan"
status: "approved"
title: "残存 Issue #28〜#33 の修正・仕様判断計画"
created: "2026-10-10"
updated: "2026-10-10"
owners: ["scarecrow173"]
relations:
  source: []
  implements:
    - "docs/specs/residual-compliance.md"
  implemented-by:
    - "docs/tasks/container-review-edge-cases.md"
    - "docs/tasks/expensive-review-consistency.md"
    - "docs/tasks/fixture-review-fidelity.md"
    - "docs/tasks/avsync-marker-ingestion.md"
    - "docs/tasks/locate-residual-pcm.md"
    - "docs/tasks/spec-residual-decisions.md"
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related: []
  refines: []
  refined-by: []
  derives-from:
    - "docs/designs/residual-fixes.md"
  derived-by: []
  verifies: []
  verified-by:
    - "docs/test-specs/residual-fixes.md"
  references:
    - "https://github.com/scarecrow173/audio-lens/issues/28"
    - "https://github.com/scarecrow173/audio-lens/issues/29"
    - "https://github.com/scarecrow173/audio-lens/issues/30"
    - "https://github.com/scarecrow173/audio-lens/issues/31"
    - "https://github.com/scarecrow173/audio-lens/issues/32"
    - "https://github.com/scarecrow173/audio-lens/issues/33"
  defers: []
  deferred-by: []
---

# 残存 Issue #28〜#33 の修正・仕様判断計画

## Goal

#28〜#33 を棚卸しから検証まで追跡する一つの残件管理 plan。status draft とし、承認前の task を実行しない。

## Scope and Split

これは共通の残件監査・完了 review を束ねる計画であり、一括実装・一括リリースを要求しない。#28、#29、#30、#31、#32、#33 は独立 stream として別 task/commit/review に分ける。依頼により plan は 1 件とするが、承認時にさらに分割できる。

## Non-goals

コードの今回の変更、承認の代行、依存追加の決定、高速化やデコーダ拡張を含めない。

## Design Inputs

residual-compliance spec と residual-issue-boundaries ADR は proposed、residual-fixes design は draft。review-protocol の承認を受けるまで implementation-ready としない。

## File Map

| Issue | 所有ファイル/境界 | task |
| --- | --- | --- |
| #28 | `crates/al-media/src/avi.rs`, `crates/al-media/src/mp4.rs`, `crates/al-media/src/video.rs`, `crates/al-media/src/metadata.rs`, `crates/al-dataset/src/stage.rs`, `crates/al-viz/src/` | [container-review-edge-cases](../tasks/container-review-edge-cases.md) |
| #29 | `crates/al-analysis/src/expensive/`, `crates/al-analysis/tests/expensive.rs` | [expensive-review-consistency](../tasks/expensive-review-consistency.md) |
| #30 | `crates/al-testkit/src/media.rs`, `crates/al-media/tests/golden.rs`, `crates/al-locate/tests/golden.rs`, `crates/al-analysis/tests/golden.rs`, `docs/specs/expensive-analysis.md` | [fixture-review-fidelity](../tasks/fixture-review-fidelity.md) |
| #31 | `crates/al-analysis/src/avsync.rs`, `crates/al-analysis/src/stage.rs`, `crates/al-analysis/tests/avsync.rs`, `crates/al-cli/src/` | [avsync-marker-ingestion](../tasks/avsync-marker-ingestion.md) |
| #32 | `crates/al-locate/src/stage.rs`, `crates/al-locate/src/residual.rs`, `crates/al-dataset/src/audio.rs`, `crates/al-locate/tests/locate.rs` | [locate-residual-pcm](../tasks/locate-residual-pcm.md) |
| #33 | `docs/specs/audiolens-overview.md`, `docs/specs/analysis-dataset.md`, `docs/specs/candidate-event-model.md`, `crates/al-analysis/src/stage.rs`, `crates/al-dataset/src/audio.rs`, `crates/al-dataset/src/types.rs` | [spec-residual-decisions](../tasks/spec-residual-decisions.md) |

## Dependency Graph

| Stream | Depends on | 完了 review |
| --- | --- | --- |
| 共通 | spec/ADR/design/plan の承認 | AC と Issue 全項目の照合 |
| #28 #29 #30 #32 | 共通承認 | 独立検証して順次提出 |
| #31 | 共通承認、#28 時計契約 | marker の指定/未指定回帰 |
| #33 | 共通承認、三つの方式判断 | 採用した仕様と回帰の照合 |

critical path は承認 → #28 時計契約 → #31 → 残件完了監査。他 stream は独立に実施できる。#33 の結果が別 stream の schema を変える場合は着手前に関係と設計を更新する。

## Implementation Sequence

1. spec/ADR と design/plan を review し、未解決の入力クロック・#33 方式を決定する。
2. 各 task 内で契約テストを追加する。
3. 各対象テストの失敗を確認する。
4. 各所有境界を修正する。
5. 各対象テストを再実行する。
6. AC と Issue 全項目に修正・文書化・承認済み defer の証拠を記録する。
7. 意味のある境界でコミットする。
8. 残件全体の仕様整合を最終 review する。

## Risk Register

| Risk | Likelihood | Impact | Mitigation |
| --- | --- | --- | --- |
| 残差保存失敗による信号/来歴の不整合 | med | high | artifact 登録と読み戻しテストを #32 で確認 |
| 時計/schema の変更による既存出力の差 | med | high | 既存 golden と境界 fixture を比較し、成果物を再生成 |
| FLAC 等の外部依存が利用不能 | med | med | #33 の承認前に依存追加を決めず、選択肢を review |
| fixture 全拒否による偽の成功 | med | high | #30 で正例・負例の両方を固定 |

## Rollback Strategy

現在の変更は文書のみでデータ変換を行わない。将来の実装で信号・時刻・来歴の回帰が出た場合は該当論理 commit を revert し、同じ入力から dataset を再生成する。対象検証と golden の一致を確認する。形式変更に互換層や migration を設けない。

## Verification Matrix

| Step | Verification | Pass condition |
| --- | --- | --- |
| 1 | spec/ADR/design/plan review | 明示承認と方式決定が記録される |
| 2 | 回帰テストと AC review | 契約を観測する assertion がある |
| 3 | 対象コマンド | 原因に対応した失敗が確認される |
| 4–5 | 下記対象コマンド | exit 0、回帰成功 |
| 6 | AC-28〜AC-33 と task inventory 照合 | 未対応項目がない |
| 7 | git show / git status | 論理境界の commit、無関係変更なし |
| 8 | mise run test / fmt / lint と文書監査 | exit 0、承認した仕様と一致 |

- #28: `mise exec -- cargo test -p al-media -p al-dataset -p al-viz`
- #29: `mise exec -- cargo test -p al-analysis`
- #30: `mise exec -- cargo test -p al-testkit -p al-media -p al-locate -p al-analysis`
- #31: `mise exec -- cargo test -p al-analysis --test avsync`
- #32: `mise exec -- cargo test -p al-locate -p al-dataset`
- #33: `mise run test`

## Self-Review

各 AC を同番号の task と test-spec に対応付けた。#31 は専用 task がなく #39 に言及だけだったため新設した。#33 と #30 の defer を無断の仕様緩和で閉じない。design には spec/ADR 各 1 件のみを直接接続した。

## Review and Implementation Handoff

reviewer は spec/ADR の提案と design/plan の draft を確認し、#31 入力クロック、#33 三方式、#30 defer を決定する。承認後に task を必要な実装単位へ分割し blocked を解除する。独立 stream の委譲実装は推奨するが、実行前に利用者の承認を得て環境内の実装/レビュー/検証能力を選ぶ。今回コードと実装テストは変更・実行しない。

## Documentation Verification (2026-10-10)

- `mise install --locked` は global config の未 lock ツールで失敗した。リポジトリの toolchain を明示した `mise install --locked python uv pipx:apm-cli pipx:serena-agent rust` と `mise run setup` は成功した。
- spec/ADR/design は生成スクリプトを使用した。draft design に対する plan 生成と draft plan に対する task 生成は通常ゲートで拒否されるため、今回の明示依頼に基づきレビュー用文書を先に記述した。承認状態は引き上げていない。
- `mise exec -- node .agents/skills/doc-driven-dev-graph/scripts/inspect_graph.js --focus docs/specs/residual-compliance.md --format json`: briefing pass、Artifact Graph issues 0、hardBlockers 0。
- `mise exec -- node .agents/skills/doc-driven-dev-graph/scripts/build_task_graph.js --plan docs/plans/residual-fixes.md --json`: 6 tasks、issues 0、runnable 0。
- 現行 Graph の chain 候補は design=approved、plan=approved/in-progress/completed に限定される。draft plan focus では upstream chain を選ばないため briefing/design/planning の fail は文書の関係欠落を意味しない。全実行ゲート通過はレビュー承認後に再検証する。
- 文書全体監査には既存の手書き ID と test-spec の plan 参照に 13 findings が残る。今回の追加・変更文書に findings はない。既存の関係ない文書は修正しない。
- コードは変更せず、Rust 実装テストは未実行。今回の検証は文書関係・index・DAG・Issue 対応と差分検査に限定する。

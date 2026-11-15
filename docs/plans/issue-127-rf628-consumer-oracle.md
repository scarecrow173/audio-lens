---
id: "PLAN-034Y19owSjMFxXkwkts0L2"
type: "plan"
status: "approved"
title: "Issue #127 RF#628 consumer oracle: 実装計画"
created: "2026-11-15"
updated: "2026-11-15"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/audio-lens/issues/127"
    - "https://github.com/scarecrow173/ruviform/issues/628"
  changes:
    added:
      - type: "file"
        path: "Cargo.toml"
      - type: "file"
        path: "Cargo.lock"
      - type: "file"
        path: "crates/al-analysis/Cargo.toml"
      - type: "file"
        path: "crates/al-analysis/tests/rf628_oracle.rs"
      - type: "file"
        path: "docs/plans/issue-127-rf628-consumer-oracle.md"
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated:
      - type: "file"
        path: "ui/vendor/ruviform"
  implements:
    - "docs/specs/issue-127-rf628-consumer-oracle.md"
  implemented-by: []
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related: []
  refines: []
  refined-by: []
  derives-from:
    - "docs/specs/issue-127-rf628-consumer-oracle.md"
    - "docs/designs/issue-127-rf628-consumer-oracle.md"
  derived-by: []
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# Issue #127 RF#628 consumer oracle: 実装計画 Implementation Plan

## Goal

`ruviform-audio-test` の deterministic PCM fixture/oracle を使い、
AudioLens file-based 解析（f64 fidelity・長尺非回帰・gap/Validity）
の独立 consumer oracle テストを `crates/al-analysis` に実装し、
ruviform pin を `abb9f372` へ lockfile-primary で追従する。

## Scope

単一 implementation stream。AL-1/AL-2/AL-3 は同一テストファイル・
同一 fixture→artifact→stage 経路を共有し、分割すると helper が
重複するため 1 plan とする。rev-watch 文書は同一 pin bump の
記録として本 plan に含める。

## Upstream Documents

- `docs/specs/issue-127-rf628-consumer-oracle.md`
- `docs/designs/issue-127-rf628-consumer-oracle.md`
- `docs/test-specs/issue-127-rf628-consumer-oracle.md`

## Design Inputs

- `docs/designs/issue-127-rf628-consumer-oracle.md`

## File Map

- `Cargo.toml`（root）- `[workspace.dependencies]` に
  `ruviform-audio-test` git dep（`rev` なし）追加
- `crates/al-analysis/Cargo.toml` - `[dev-dependencies]` に
  `ruviform-audio-test.workspace = true` と
  `ruviform-audio-core.workspace = true`（`SampleRate` 用）追加
- `Cargo.lock` - `tools/update-ruviform-rev.mjs` による再生成
  （手編集禁止）
- `crates/al-analysis/tests/rf628_oracle.rs` - AL-1/AL-2/AL-3 の
  統合テスト新設（fixture→f64 widen→pcm_f64le→Manifest→stage
  駆動→独立期待値照合）
- `docs/specs|designs|plans|tasks|test-specs|impl/ir/
  issue-127-rf628-consumer-oracle.md` - 文書チェーン
- `docs/{specs,designs,tasks,test-specs,impl/ir}/
  ruviform-abb9f372-rev-watch.md` - rev-watch 5 点セット
- `docs/impl/exp/` - 実験・証拠ログ（必要に応じ `.jsonl`）
- `docs/<type>/README.md`・`docs/impl/{ir,exp}/README.md` -
  index 再生成（生成スクリプト経由、手編集禁止）

## Implementation Sequence

- [x] ruviform main 再解決（`abb9f372d3ee95ee824b3c4f7dc68e8083ce2774`、
  `src/pipeline/` 存在確認）と doc chain 起票
- [x] manifest へ `ruviform-audio-test` 追加 → `update-ruviform-rev.mjs`
  → `sync-ui-deps` → `check-ruviform-lock`/`check-ruviform-rev` Pass
- [ ] `ruviform-audio-core` を al-analysis dev-deps に追加
  （`SampleRate` 構築に必要）
- [ ] `tests/rf628_oracle.rs` skeleton: fixture→f64→pcm_f64le→
  Manifest→StageContext→CheapAnalysisStage helper + oracle 照合 helper
- [ ] AL-1 テスト実装（Constant/Sine/Noise/Square/ステレオ Noise）
  と実測による許容差確定
- [ ] AL-2 長尺 scenario テスト実装（規模実測→test-spec 記録）
- [ ] AL-3 gap/NaN validity テスト実装（期待 validity map 独立導出）
- [ ] `mise run test al-analysis` で全テスト Pass 確認
- [ ] rev-watch 5 点セット + impl/ir + impl/exp 記録
- [ ] index 再生成（doc-maintenance index apply + sync_impl_index）
- [ ] 全検証 lane 実行（fmt/check/lint/test/lock/rev/headless/prek）
- [ ] review-protocol レビュー → blocker 解消
- [ ] commit → push → PR（`Refs #127`）→ worker_done

## Dependencies and Risks

- ruviform private repo への system git 認証（`git-fetch-with-cli`）—
  `cargo fetch` が必要とする既存前提。
- `ruviform-audio-test` の推移的依存で lockfile package 数が
  20→26 に増加。dev-dep 専用で製品影響なし。
- Sine/Sweep の oracle（libm）vs generator（poly+固定小数位相）の
  サンプル差 → feature 許容差で吸収、fixture 検証は AbsEpsilon。
- 長尺テストの debug 実行時間 → 実測で規模決定。
- NaN gap の spectral 縮退（`argmax` NaN 規則）→ 観測 pin +
  PR 発見事項。

## Task Breakdown

- dep/pin（完了）
- テスト実装（AL-1→AL-2→AL-3、同一ファイル）
- 文書・index・検証・PR

## Verification

- [ ] `mise run check-ruviform-lock` / `check-ruviform-rev`: Pass
- [ ] `mise run fmt`: Pass
- [ ] `mise run check al-analysis`: Pass
- [ ] `mise run lint al-analysis`: Pass
- [ ] `mise run test al-analysis`: Pass（新規 oracle テスト含む）
- [ ] `mise run check-cli-headless`: Pass
- [ ] `mise exec -- prek run --all-files`: Pass
- [ ] `mise exec -- cargo fetch --locked`: Pass（補助証拠）

## Open Questions / Gaps

- AL-2 の長尺規模（秒数）は実測後に確定し test-spec に記録する。
- gap 入力での `noise.tonal_peak_hz` 縮退値の正確な観測値は
  テスト実行時に確定する（argmax NaN 規則の帰結を pin）。

## Self-Review

- [x] Every spec requirement maps to a task.
- [x] No placeholders remain（長尺規模は実測決定として明示）。
- [x] File paths, relations, status values, and terminology are consistent.

## Implementation Handoff

- Recommended mode: inline execution（dispatched worker が直接実装。
  helper worker なし）
- Independent work streams: none
- User approval required before dispatch: no（worker contract により
  review-protocol レビューを承認代替とする）

## Review Handoff

- 長尺規模の最終値・gap 縮退の観測値・許容差の根拠を reviewer が
  test-spec/PR と照合すること。

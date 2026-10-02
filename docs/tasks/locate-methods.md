---
id: "TASK-034z8kAawdMoi8AxPYZjrB"
type: "task"
status: "todo"
title: "locate-methods 4手法 + 候補/証拠型"
created: "2026-10-02"
updated: "2026-10-02"
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
  implements:
    - "docs/plans/locate-and-tracking.md"
  implemented-by: []
  depends-on:
    - "docs/plans/locate-and-tracking.md"
    - "docs/tasks/locate-skeleton-dsp.md"
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
    - "docs/test-specs/locate-method-consensus.md"
  references: []
  defers: []
  deferred-by: []
---

# locate-methods 4手法 + 候補/証拠型

## Work

`LocateMethod` trait と 4 実装(full_ncc / band_ncc / gcc_phat / spectral_envelope)を実装し、候補位置・スコア・ピーク対背景比・ピーク幅・第1/第2ピーク比・validity・棄却理由を生成する。

## Classification

- `normal-plan-task`

## Upstream Documents

- `docs/plans/locate-and-tracking.md`
- `docs/specs/source-localization.md`

## Dependencies

- `docs/tasks/locate-skeleton-dsp.md`

## Affected Files

- `crates/al-locate/src/methods.rs`
- `crates/al-locate/src/candidate.rs`(`LocateCandidate` / `MethodEvidence`)

## Done When

- [ ] 4 手法が独立に `MethodEvidence` を生成
- [ ] スペクトル包絡法は `sample_format` を見て比較前に正規化
- [ ] 無音/低情報窓がエラーではなく validity + 棄却理由になる

## Verification

- [ ] 合成信号で各手法が既知オフセット近傍にピークを返す
- [ ] `cargo test -p al-locate` パス

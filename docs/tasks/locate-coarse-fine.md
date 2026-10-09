---
id: "TASK-035gHnMnKlJhGfDsAqWe"
type: "task"
status: "todo"
title: "locate: 帯域キャッシュ + coarse-to-fine 間引き探索"
created: "2026-10-09"
updated: "2026-10-09"
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
    - "docs/plans/simd-dsp-foundation.md"
  implemented-by: []
  depends-on:
    - "docs/tasks/al-math-correlation.md"
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
    - "docs/test-specs/simd-dsp-foundation.md"
  references:
    - "https://github.com/scarecrow173/audio-lens/issues/40"
  defers: []
  deferred-by: []
---

# locate: 帯域キャッシュ + coarse-to-fine 間引き探索

## Work

`BandNcc` がペアごとにキャプチャ全区間に bandpass を掛けている
重複を、入力単位のキャッシュに移す。あわせて長尺ソースの探索を
間引き信号での coarse→fine に分割する。

- bandpass 結果を入力(capture)単位でキャッシュし、
  メソッド間・ペア間で共有(ステージループ外側に保持)
- locate 入口で入力ごとに間引き版信号(1/4 相当)を生成し、
  粗探索は間引き信号、精密化は原レートで行う
- 間引きは #37 のリサンプル正規化インフラと共用できる構造に
- 長尺ソース(数十秒)で効果を確認

## Classification

- `normal-plan-task`

## Upstream Documents

- `docs/plans/simd-dsp-foundation.md`

## Dependencies

- `docs/tasks/al-math-correlation.md`

## Affected Files

- `crates/al-locate/src/stage.rs`(キャッシュ導入・間引き経路)
- `crates/al-locate/src/methods.rs`(coarse-to-fine)
- `crates/al-math/src/`(間引き用リサンプラ借用)

## Done When

- [ ] BandNcc が入力単位のフィルタ済み信号を再利用
- [ ] 長尺ソースで粗探索が間引き信号で動作
- [ ] 既存 golden が全てパス

## Verification

- [ ] `cargo test -p al-locate`

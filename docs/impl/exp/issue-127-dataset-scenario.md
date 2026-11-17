---
id: "EXP-034Z9kQmDatasetScen06"
type: "impl"
status: "completed"
title: "Dataset tab→workspace scenario の実験記録"
created: "2026-11-18"
updated: "2026-11-18"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/audio-lens/issues/127"
  changes:
    added: []
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/tasks/issue-127-dataset-scenario.md"
  implemented-by: []
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related: []
---

# Dataset tab→workspace scenario の実験記録

実験イベントは `issue-127-dataset-scenario-exp.jsonl` を参照。

## 記録した判断

- oracle を spec 投影検査に置いた理由: `SemanticGuiTest` snapshot は
  table 行を持たず、port 経由では typed cell を観測できない。spec の
  cell は ViewHost がそのまま mount する投影なので、spec 側の
  不一致は mount 後表示にも現れる — これが port 非依存で最も正直な
  oracle 位置。
- tab 巡回順を `[1,2,0]` にした理由: index 0 は spec 初期 active
  であり、3 tab 全ての `tab-select` intent を発行するには巡回が必要。
- window-close case を catalog 末尾に置いた理由: window を破壊する
  ため後続 case の前提を壊さない。
- `CloseRequested` 未観測を `Fail` でなく `NotRun` にした理由:
  settle 予算内のイベント未到達は観測前提の不成立であり、観測された
  契約違反（veto で破棄/approve で残留）のみを `Fail` とする。

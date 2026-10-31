---
id: "TASK-034P3OZEY2OrqNMPwTpQnL"
type: "task"
status: "todo"
title: "Ruviform GUI アプリケーション構築(AL#105)"
created: "2026-10-30"
updated: "2026-10-30"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/audio-lens/issues/105"
  changes:
    added: []
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/plans/ruviform-foundation-migration.md"
  implemented-by: []
  depends-on:
    - "docs/plans/ruviform-foundation-migration.md"
    - "docs/tasks/ruviform-dependency-ci.md"
    - "docs/tasks/ruviform-job-dataset-regression.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related: []
  refines: []
  refined-by: []
  derives-from: []
  derived-by: []
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# Ruviform GUI アプリケーション構築(AL#105)

## Work

Ruviform GUI host(#156)・表示 primitive(#157)・headless
基盤(#155)を使い、AudioLens の GUI アプリケーション(仮称
`al-gui`)と UI 非依存の内部解析サービス(仮称 `al-service`)を
構築する。Issue
[#105](https://github.com/scarecrow173/audio-lens/issues/105) を正本とする。

- `al-service`: run/resume/cancel/progress/inspect のジョブ操作と
  解析データ query/data port を提供し、GUI/CLI の唯一の解析
  入口とする
- `al-gui`: 画面構成・操作意味・AudioLens の
  FeatureRecord/イベント/Provenance 表示を担う薄い
  プレゼンテーション層。表示 cache は実行時のみ
- `al-cli` を al-service 経由に再配線し、CLI が WebView/GPU/
  ウィンドウに依存しないことを維持
- GUI 依存が headless 側へ逆流しない

## Classification

- `normal-plan-task`

## Upstream Documents

- `docs/plans/ruviform-foundation-migration.md`
- `docs/designs/ruviform-migration-responsibility-map.md` §7
- Issue [#105](https://github.com/scarecrow173/audio-lens/issues/105)

## Dependencies

- `docs/tasks/ruviform-dependency-ci.md`
- `docs/tasks/ruviform-job-dataset-regression.md`
- 外部依存: Ruviform [#155](https://github.com/scarecrow173/ruviform/issues/155)
  (CLI/headless)、[#156](https://github.com/scarecrow173/ruviform/issues/156)
  (GUI host)、[#157](https://github.com/scarecrow173/ruviform/issues/157)
  (表示 primitive)

## Affected Files

- `al-service`(新設・仮称。`crates/al-service` 等)
- `al-gui`(新設・仮称。`crates/al-gui` 等)
- `crates/al-cli`(al-service 経由への再配線)
- `crates/al-dataset`(query/data port 供給側)

## Done When

- [ ] GUI が稼働し、GUI/CLI が同一 Job・同一結果を返す
- [ ] GUI が永続 Analysis Dataset を再解析なしで閲覧できる
- [ ] 長尺データの応答性の目標基準を定義・記録し、計測結果が
      その基準を満たす(#106 への移行判断は実使用者/レビュアに
      よる明示的な受入確認をゲートとする)
- [ ] #105 の完了条件を満たす

## Verification

- [ ] GUI/CLI の同一結果検証
- [ ] GUI 依存なしの headless ビルド・テストが通る
- [ ] 長尺データでの応答性・メモリ計測

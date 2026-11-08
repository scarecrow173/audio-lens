---
id: "TASK-rD6fg03HkkWxkiNSjqpxrS"
type: "task"
status: "in-progress"
title: "canvas paint 是正のタスク"
created: "2026-11-08"
updated: "2026-11-08"
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
    - "docs/plans/canvas-paint-empty.md"
  implemented-by:
    - "docs/impl/ir/normalize-waveform-source-samples-to-unit-range.md"
  depends-on:
    - "docs/plans/canvas-paint-empty.md"
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
    - "docs/test-specs/canvas-paint-empty.md"
  references: []
  defers: []
  deferred-by: []
---

## タスク

- [x] AL#179 fixture で `source_screen` の projection が非空で
      あることを確認(切り分け)。
- [x] `DatasetWaveform` が生 PCM スケールを返していることを
      特定し、`format_scale` による [-1,1] 正規化を実装。
- [x] `waveform_source_yields_samples_normalized_to_unit_range`
      回帰テストを追加(red → green)。
- [x] Windows 実窓で Dataset → Source / Spectrogram の pixel
      解析証拠を `.devin/evidence/al179-canvas-paint/` に保存。
- [x] `docs/impl/ir/normalize-waveform-source-samples-to-unit-range.md`
      に実装記録を残し、`docs/impl/ir/README.md` を更新。
- [ ] `mise run fmt/check/lint/test` および ui 系 task で
      検証し、`origin/main` を merge してから commit/push/PR。

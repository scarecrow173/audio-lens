---
id: "IMPL-034Z9kQmDatasetRevWatch05"
type: "impl"
status: "completed"
title: "ruviform b6d0cc6a pin 追従評価の実施記録"
created: "2026-11-18"
updated: "2026-11-18"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/audio-lens/issues/127"
  changes:
    added:
      - type: "code"
        path: "docs/specs/ruviform-b6d0cc6a-rev-watch.md"
      - type: "code"
        path: "docs/designs/ruviform-b6d0cc6a-rev-watch.md"
      - type: "code"
        path: "docs/tasks/ruviform-b6d0cc6a-rev-watch.md"
      - type: "code"
        path: "docs/test-specs/ruviform-b6d0cc6a-rev-watch.md"
      - type: "code"
        path: "docs/impl/ir/ruviform-b6d0cc6a-rev-watch.md"
      - type: "code"
        path: "evidence/al127/b6d0cc6a-verification.md"
    modified:
      - type: "code"
        path: "Cargo.lock"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/tasks/ruviform-b6d0cc6a-rev-watch.md"
  implemented-by: []
  depends-on:
    - "docs/tasks/ruviform-b6d0cc6a-rev-watch.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related: []
---

# ruviform b6d0cc6a pin 追従の実施記録

## 実施内容

1. candidate 再解決: upstream `main` = `b6d0cc6a`（61 commits ahead of
   `cbcc8343`）。
2. 差分実測: AL 依存木内は `ruviform-ui-gpui`（window-close/DPI 観測面）、
   `ruviform-exec`（JobExecutor）、`ruviform-audio-core`（MIDI mock）、
   `ruviform-audio-test`（EnvironmentKind/MIDI fault/virtual-endpoint）、
   `ruviform-io-file`（README のみ）。依存木外に script/luau/plugin 系
   新規 crate。
3. `update-ruviform-rev.mjs` → `sync-ui-deps` → `cargo fetch --locked`
   → `check-ruviform-lock`/`check-ruviform-rev`/`check-ui`/`test-ui`/
   `verify-toolchain` を実行。
4. 判定: **採用**。全検証 lane exit 0、rollback 不要
   （`evidence/al127/b6d0cc6a-verification.md` 参照）。

## 残件

- upstream main の covering CI 証拠の個別確認は本 slice では NotRun —
  merge 後対象 SHA の CI 結果が統合検証の証拠（Coordinator 確認）。
- `b6d0cc6a` 以降の upstream 進行は別 rev-watch slice の責務。

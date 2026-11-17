---
id: "TASK-034Z9kQmDatasetRevWatch03"
type: "task"
status: "done"
title: "ruviform b6d0cc6a pin 追従評価タスク"
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
    - "docs/designs/ruviform-b6d0cc6a-rev-watch.md"
  implemented-by: []
  depends-on:
    - "docs/designs/ruviform-b6d0cc6a-rev-watch.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related: []
---

# ruviform b6d0cc6a pin 追従タスク

- [x] upstream main を `git ls-remote` / local clone で再解決し
      candidate `b6d0cc6a` を確定
- [x] `cbcc8343..b6d0cc6a` の差分を local clone(read-only)で実測し
      AL 依存木内/外を仕分け
- [x] `node tools/update-ruviform-rev.mjs b6d0cc6a...` で lockfile 更新
- [x] `mise run sync-ui-deps` → `cargo fetch --locked` →
      `check-ruviform-lock`/`check-ruviform-rev`/`check-ui`/`test-ui`/
      `verify-toolchain`
- [x] 採用/保留/rollback 判定を spec/design に記録
- [x] `evidence/al127/b6d0cc6a-verification.md` に検証証拠を記録

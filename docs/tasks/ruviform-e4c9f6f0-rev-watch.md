---
id: "TASK-ueCr6an23J49bQuGKAM6bx"
type: "task"
status: "done"
title: "ruviform e4c9f6f0 pin 追従評価タスク"
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
    - "docs/designs/ruviform-e4c9f6f0-rev-watch.md"
  implemented-by: []
  depends-on:
    - "docs/designs/ruviform-e4c9f6f0-rev-watch.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related: []
---

# ruviform e4c9f6f0 pin 追従タスク

- [x] upstream main を `git ls-remote` / local clone で再解決し
      candidate `e4c9f6f0715da3ac479f8c437d21a021327e51da` を確定
      （drift なし）
- [x] candidate の upstream covering CI を `gh run list --commit` で
      確認 — run なし、NotRun として記録
- [x] `b6d0cc6a..e4c9f6f0` の差分を local clone(read-only)で実測し
      AL 依存木内/外を仕分け
- [x] `node tools/update-ruviform-rev.mjs e4c9f6f0...` で lockfile 更新
- [x] `mise run sync-ui-deps` → `cargo fetch --locked` →
      `check-ruviform-lock`/`check-ruviform-rev`/`check-ui`/`test-ui`/
      `verify-toolchain`
- [x] standing 回帰 lane（`check al-gui`/`lint al-gui`/`test al-gui`/
      `check-ui-gpui`/`lint-ui-gpui`/`test-ui-gpui`/`build-ui-gpui`/
      `test-ui-gpui-real-headless`/`check-cli-headless`/
      `check-ui-webview`/`lint-ui-webview`/`test-ui-webview`/
      `test al-analysis`/`fmt`/`prek run --all-files`）を実走
- [x] 採用/保留/rollback 判定と新 API の採用/見送りを
      spec/design に記録
- [x] `evidence/al127/e4c9f6f0-verification.md` に検証証拠を記録

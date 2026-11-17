---
id: "IMPL-sdkjbhBGWPFajNDZPGXdrB"
type: "impl"
status: "completed"
title: "ruviform e4c9f6f0 pin 追従評価の実施記録"
created: "2026-11-18"
updated: "2026-11-18"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/audio-lens/issues/127"
  changes:
    added:
      - type: "code"
        path: "docs/specs/ruviform-e4c9f6f0-rev-watch.md"
      - type: "code"
        path: "docs/designs/ruviform-e4c9f6f0-rev-watch.md"
      - type: "code"
        path: "docs/tasks/ruviform-e4c9f6f0-rev-watch.md"
      - type: "code"
        path: "docs/test-specs/ruviform-e4c9f6f0-rev-watch.md"
      - type: "code"
        path: "docs/impl/ir/ruviform-e4c9f6f0-rev-watch.md"
      - type: "code"
        path: "evidence/al127/e4c9f6f0-verification.md"
    modified:
      - type: "code"
        path: "Cargo.lock"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/tasks/ruviform-e4c9f6f0-rev-watch.md"
  implemented-by: []
  depends-on:
    - "docs/tasks/ruviform-e4c9f6f0-rev-watch.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related: []
metadata:
  experiments:
    adopted: []
    rejected: []
---

# ruviform e4c9f6f0 pin 追従の実施記録

## 実施内容

1. candidate 再解決: upstream `main` =
   `e4c9f6f0715da3ac479f8c437d21a021327e51da`
   （`git ls-remote` 実測、探索時点から drift なし、
   `b6d0cc6a` から 5 commits 先行）。
2. upstream covering CI 確認: `gh run list --repo
   scarecrow173/ruviform --commit e4c9f6f0...` で run なし —
   **NotRun** として記録。旧 pin `b6d0cc6a` にも covering run なし。
   直近 covering は祖先 `7124f0b2` の ci/daily-ci success
   （consumer-verify は同 SHA で一時 failure→後続 success）。
3. 差分実測: local clone（read-only）で `b6d0cc6a..e4c9f6f0` =
   5 commit・78 file。AL 依存木内は `ruviform-audio-test`
   （`RunMetadata::for_environment` additive + dev-dep
   audio-device/midi-device + `tests/os_endpoint.rs`）、
   `ruviform-plugin-core`（`GuiSize` + default 実装付き verbs）、
   `ruviform-plugin-host`（`HostCommand::SetGui*` + dispatch +
   `EditorSessionOps` 実装化）。依存木外は plugin-{clap,dock-test,
   test-vst3,vst3} と docs/evidence 系。
4. `update-ruviform-rev.mjs` → `sync-ui-deps` → `cargo fetch
   --locked` → `check-ruviform-lock`/`check-ruviform-rev`/`check-ui`/
   `test-ui`/`verify-toolchain` を実行。`Cargo.lock` 差分は
   source fragment 26 行のみ（新規 package 混入なし）。
5. standing 回帰 lane 全実走 — 全て exit 0
   （`evidence/al127/e4c9f6f0-verification.md` 参照）。
6. 判定: **採用**。rollback 不要。

## candidate tuple

- ruviform SHA: `e4c9f6f0715da3ac479f8c437d21a021327e51da`
- AL commit（pin bump）: `f08977ede7ca6affa0e74a046c38017819e5f40c`
- resolved SHA（`Cargo.lock`）: `e4c9f6f0715da3ac479f8c437d21a021327e51da`

## 残件

- upstream `e4c9f6f0` の covering CI run は存在しない — merge 後
  AL 対象 SHA の CI 結果が統合検証の証拠（Coordinator 確認）。
- `e4c9f6f0` 以降の upstream 進行は別 rev-watch slice の責務。
- additive 差分（`RunMetadata::for_environment`・L0 editor verbs・
  os_endpoint/endpoint probe）の AL 側消費は本 slice 対象外 —
  見送り/NotApplicable として記録済み。

---
id: "TASK-034WQNI8ouT229zjnxOOZl"
type: "task"
status: "done"
title: "OS-L1 GPUI Desktop CI 統合(AL#206): gpui-verify lane + 構造契約更新"
created: "2026-11-13"
updated: "2026-11-13"
owners:
  - "scarecrow173"
relations:
  source:
    - "https://github.com/scarecrow173/audio-lens/issues/206"
  changes:
    added:
      - type: "file"
        path: ".github/actions/gpui-verify/action.yml"
      - type: "file"
        path: "docs/specs/ci-os-l1-gpui-desktop.md"
      - type: "file"
        path: "docs/designs/ci-os-l1-gpui-desktop.md"
      - type: "file"
        path: "docs/plans/implement-ci-os-l1-gpui-desktop.md"
      - type: "file"
        path: "docs/test-specs/ci-os-l1-gpui-desktop.md"
      - type: "file"
        path: "docs/tasks/ci-os-l1-gpui-desktop.md"
    modified:
      - type: "file"
        path: ".github/workflows/ci.yml"
      - type: "file"
        path: ".github/actions/verify/action.yml"
      - type: "file"
        path: "tools/check-ci-workflow.mjs"
      - type: "file"
        path: "mise.toml"
      - type: "file"
        path: "docs/specs/README.md"
      - type: "file"
        path: "docs/designs/README.md"
      - type: "file"
        path: "docs/plans/README.md"
      - type: "file"
        path: "docs/tasks/README.md"
      - type: "file"
        path: "docs/test-specs/README.md"
    deleted: []
    renamed: []
    moved: []
    generated:
      - type: "file"
        path: "docs/specs/README.md"
      - type: "file"
        path: "docs/designs/README.md"
      - type: "file"
        path: "docs/plans/README.md"
      - type: "file"
        path: "docs/tasks/README.md"
      - type: "file"
        path: "docs/test-specs/README.md"
  implements:
    - "../plans/implement-ci-os-l1-gpui-desktop.md"
  implemented-by: []
  depends-on:
    - "../plans/implement-ci-os-l1-gpui-desktop.md"
    - "../designs/ci-os-l1-gpui-desktop.md"
    - "../test-specs/ci-os-l1-gpui-desktop.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "https://github.com/scarecrow173/audio-lens/issues/189"
    - "../tasks/full-ci-ci-summary-issue-158.md"
  refines: []
  refined-by: []
  derives-from:
    - "../plans/implement-ci-os-l1-gpui-desktop.md"
  derived-by: []
  verifies: []
  verified-by:
    - "../test-specs/ci-os-l1-gpui-desktop.md"
  references: []
  defers: []
  deferred-by: []
---

# OS-L1 GPUI Desktop CI 統合(AL#206): gpui-verify lane + 構造契約更新

## Work

Issue #206 の OS-L1 GPUI Desktop CI 統合を実施する。
`.github/actions/gpui-verify/action.yml` を新設し、
`ci.yml` に `gpui-verify`(ubuntu/windows)/`gpui-verify-macos`
job を追加、`ci-summary`/`summary` の needs・証跡・
fail-closed 集約を更新、`tools/check-ci-workflow.mjs` の
構造契約を 14 job へ更新、`mise.toml` に `build-ui-gpui`
task を追加、`verify` action から `check-full` step を撤去
する(lint-full の clippy が型診断を包含 — 計測証拠は
design doc)。doc-chain(spec/design/test-spec/plan/task)と
index を整備し、手動 Full CI で 3 OS OS-L1 を実測する。

## Classification

- `normal-plan-task`

## Upstream Documents

- `docs/plans/implement-ci-os-l1-gpui-desktop.md`
- `docs/specs/ci-os-l1-gpui-desktop.md`
- `docs/designs/ci-os-l1-gpui-desktop.md`
- `docs/test-specs/ci-os-l1-gpui-desktop.md`

## Dependencies

- `docs/plans/implement-ci-os-l1-gpui-desktop.md`(approved)
- Issue #189(旧 WebView 撤去)とは独立 — webview lane は
  維持する。

## Affected Files

- `.github/actions/gpui-verify/action.yml`(新規)
- `.github/workflows/ci.yml`
- `.github/actions/verify/action.yml`
- `tools/check-ci-workflow.mjs`
- `mise.toml`
- `docs/{specs,designs,plans,tasks,test-specs}/ci-os-l1-gpui-desktop.md` + 各 README index

## Done When

- [x] `gpui-verify`/`gpui-verify-macos` が ci.yml に存在し
  `ci-summary`/`summary` が正しく集約する
- [x] `check-ui-gpui`/`lint-ui-gpui`/`test-ui-gpui`/
  `build-ui-gpui` が 3 OS で実行される lane が接続済み
- [x] `check-ci-workflow` が新契約で exit 0
- [x] 手動 Full CI で 3 OS OS-L1 Pass の証拠がある
  (run 43422571711, SHA 7e5412d: gpui-verify ubuntu/windows/
  macos 全 success、ci-summary/summary success)

## Verification

- [x] `mise exec -- actionlint` exits 0
- [x] `mise run check-ci-workflow` exits 0
- [x] `mise run check-ci-tracker-workflows` exits 0
- [x] `gh workflow run ci.yml --ref <branch> -f expected_sha=<sha>`
  の run で gpui-verify 系・ci-summary・summary が success
  (run 43422571711)

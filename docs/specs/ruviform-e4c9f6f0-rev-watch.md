---
id: "SPEC-8cAy4DVGaJhpVS3KXAB999"
type: "spec"
status: "approved"
title: "ruviform pin e4c9f6f0 追従評価（AL#127 pin 追従 slice）"
created: "2026-11-18"
updated: "2026-11-18"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/audio-lens/issues/127"
    - "https://github.com/scarecrow173/ruviform/issues/629"
    - "https://github.com/scarecrow173/ruviform/issues/389"
    - "https://github.com/scarecrow173/ruviform/issues/754"
  changes:
    added:
      - type: "file"
        path: "Cargo.lock"
      - type: "file"
        path: "docs/specs/ruviform-e4c9f6f0-rev-watch.md"
      - type: "file"
        path: "docs/designs/ruviform-e4c9f6f0-rev-watch.md"
      - type: "file"
        path: "docs/tasks/ruviform-e4c9f6f0-rev-watch.md"
      - type: "file"
        path: "docs/test-specs/ruviform-e4c9f6f0-rev-watch.md"
      - type: "file"
        path: "docs/impl/ir/ruviform-e4c9f6f0-rev-watch.md"
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements: []
  implemented-by: []
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/specs/ruviform-b6d0cc6a-rev-watch.md"
  derives-from:
    - "docs/specs/ruviform-lockfile-source-of-truth.md"
  derived-by: []
  verifies: []
  verified-by:
    - "docs/test-specs/ruviform-e4c9f6f0-rev-watch.md"
---

# ruviform pin `e4c9f6f0` 追従評価

## 背景

AL#127 常設 tracker の headless pin 追従 slice。現行 pin
`b6d0cc6a2e14f47e5324061d8dcac3d905e4abd5`（PR #226 統合済み）に対し、
upstream ruviform main HEAD `e4c9f6f0715da3ac479f8c437d21a021327e51da`
への bump を実施し、差分影響と採用/保留/rollback 判断を記録する。
`git ls-remote` / local clone `origin/main` で 2026-11-18 に再解決し、
探索時点 candidate からの drift はなかった。

## 候補

- 採用 candidate: `e4c9f6f0715da3ac479f8c437d21a021327e51da`
  （upstream main HEAD、`b6d0cc6a` から 5 commits 先行）
- 旧 pin: `b6d0cc6a2e14f47e5324061d8dcac3d905e4abd5`
- 差分: 5 commits・78 files。AL 依存木内の変更は additive のみ
  （後述）。

## 採用/保留/rollback 判定

- **採用**: `e4c9f6f0` — AL 依存木内差分は全て additive
  （`RunMetadata::for_environment` 追加・`GuiSize` と default 実装付き
  editor verbs・plugin-host verb 配線）。AL 側コード変更不要で
  check/lint/test・standing oracle・GPUI semantic lane が全て非回帰。
- **保留**: なし。
- **rollback 条件**: 本 slice の検証 lane で ruviform 起因の Fail が
  確定した場合、`node tools/update-ruviform-rev.mjs
  b6d0cc6a2e14f47e5324061d8dcac3d905e4abd5` で旧 SHA へ復帰する
  （lockfile-primary のため manifest 変更は不要）。

## 影響範囲

`b6d0cc6a..e4c9f6f0` の crates/ 差分のうち AL 依存木
（`Cargo.lock` の ruviform 26 package と照合）に入るのは:

- `ruviform-audio-test`（`27c53bd5` / upstream `scarecrow173/ruviform#629`
  AC3）: `src/metadata.rs` +47（`RunMetadata::for_environment`
  コンストラクタ追加 — additive、既存 `deterministic` は doc comment
  のみ変更）、`tests/os_endpoint.rs` 新規 +1,616（upstream 自身の
  test target で AL build graph には入らない）、`Cargo.toml` +12
  （dev-dep として `ruviform-audio-device`/`ruviform-midi-device`
  追加 — dev-dep のため consumer の依存解決に波及しない。
  `Cargo.lock` 差分で同 package の混入がないことを実測確認）。
- `ruviform-plugin-core`（`0450f92f` / upstream `scarecrow173/ruviform#389`・
  `scarecrow173/ruviform#754`）: `boundary.rs` +15（`GuiSize` 追加）、
  `plugin.rs` +55（`Plugin` trait に default 実装付き
  `set_gui_size`/`set_gui_visibility`/`set_gui_parent`、`ErasedPlugin`
  に forwarding method 追加）、`lib.rs` export 更新。
- `ruviform-plugin-host`（同 commit）: `command.rs` +35
  （`HostCommand::SetGuiSize`/`SetGuiVisibility`/`SetGuiParent`
  追加）、`composite.rs` +41（L0-only verb の OutOfProcess 拒否配線）、
  `host.rs` +68（`InProcessHost` dispatch）、`editor_session.rs` +59
  （`EditorSessionOps::set_size`/`set_visibility`/`reattach` を
  旧 `Unsupported` refusal から実 verb 実行へ実装化 — 呼出し側が
  いなければ影響なし）。

AL 依存木外（`Cargo.lock` に現れない）: `ruviform-plugin-clap`/
`ruviform-plugin-dock-test`/`ruviform-plugin-test-vst3`/
`ruviform-plugin-vst3`（VST3/CLAP adapter 系 — AL は Plugin Host を
製品提供しないため NotApplicable 方針どおり）と docs index/
evidence 系 commit（`e55b6cbc`・`e4c9f6f0`）。
`ruviform-ui`/`ruviform-ui-test`/`ruviform-ui-theme`/
`ruviform-ui-gpui`/`ruviform-ui-webview`/`ruviform-dsp`/
`ruviform-viz`/`ruviform-audio-analysis`/`ruviform-rt`/`ruviform-math`/
`ruviform-numeric`/media/IO 系は差分なし。

## 新 API の採用/見送り

- `RunMetadata::for_environment`（rf#629 AC3）: **見送り** — AL は
  `ruviform-audio-test` の `oracle`/`pcm` module のみ消費し
  `RunMetadata` を未使用。消費が必要になれば別 slice で評価。
- L0 editor verbs（`set_gui_size`/`set_gui_visibility`/
  `set_gui_parent`・`HostCommand::SetGui*`・`EditorSessionOps`
  実装化）: **見送り** — AL は plugin host を製品・test のいずれでも
  消費しない。device/plugin boundary は AL 非提供のため原則見送り。
- os_endpoint / endpoint probe（`tests/os_endpoint.rs`）: **NotApplicable**
  — device boundary（audio-device/midi-device）は AL が提供しない
  EXT-L3 領域。AL への波及は dev-dep のみで依存解決に出ないことを
  `check-cli-headless` と lockfile 差分で実測確認。

## 独立検証

`evidence/al127/e4c9f6f0-verification.md` に検証 lane の実測を記録。
upstream `e4c9f6f0` の covering CI（ci/daily-ci/consumer-verify）は
run が存在しないため **NotRun** として記録（`gh run list
--commit` で実測。直近の covering run は祖先 `7124f0b2` の
ci/daily-ci success — 旧 pin `b6d0cc6a` 自体にも covering run なし）。

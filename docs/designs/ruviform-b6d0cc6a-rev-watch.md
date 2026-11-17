---
id: "DESIGN-034Z9kQmDatasetRevWatch02"
type: "design"
status: "approved"
title: "ruviform b6d0cc6a pin 追従の設計"
created: "2026-11-18"
updated: "2026-11-18"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/audio-lens/issues/127"
  changes:
    added:
      - type: "file"
        path: "Cargo.lock"
      - type: "file"
        path: "docs/designs/ruviform-b6d0cc6a-rev-watch.md"
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/specs/ruviform-b6d0cc6a-rev-watch.md"
  implemented-by: []
  depends-on:
    - "docs/specs/ruviform-b6d0cc6a-rev-watch.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related: []
---

# ruviform b6d0cc6a pin 追従の設計

## 方針

lockfile-primary 方式を維持する — manifest に `rev` を書かず、
`node tools/update-ruviform-rev.mjs <sha>` が全 `ruviform-*` package を
単一 resolved SHA に原子解決する（失敗時 lockfile 不変の fail-closed）。
`ui/vendor/ruviform/` は `mise run sync-ui-deps` の生成物で commit しない。

## 手順

1. `git ls-remote https://github.com/scarecrow173/ruviform.git refs/heads/main`
   で candidate を再解決 → `b6d0cc6a2e14f47e5324061d8dcac3d905e4abd5`。
2. `node tools/update-ruviform-rev.mjs b6d0cc6a2e14f47e5324061d8dcac3d905e4abd5`
   → `cargo update -p ruviform-* --precise <sha>` で `Cargo.lock` を更新。
3. `mise run sync-ui-deps` で vendor 追随（REV 一致確認・commit しない）。
4. `mise exec -- cargo fetch --locked`。
5. 検証: `check-ruviform-lock` / `check-ruviform-rev` / `check-ui` /
   `test-ui` / `verify-toolchain` + slice 共通 lane 全て。

## 互換性判断

- `cbcc8343..b6d0cc6a` の AL 依存木内差分は additive（executor 追加・
  lifecycle 観測面・audio-test 語彙拡張・MIDI mock）。破壊的 API 変更なし。
- `SemanticGuiTest`/`FakeGuiTest`/`GpuiGuiTest`/`ScenarioReport` 系の
  contract は pin 済み API のまま — scenario helper は無変更で動作。
- `RealOsGuiTest` に `window_events`/`request_close`/`arm_close_response`/
  `respond_close`/`window_geometry` が追加 — 既存呼出しへの影響なし。
- `CloseDecision`(`Approve`/`Veto`/`Defer`)は `ruviform_ui` から export。
- feature 排他契約（`gpui-test` × `gpui-real` compile_error）は不変。

## window-close/DPI case の解放

`scarecrow173/audio-lens#219` 設計書 Deferred 行を本 slice で実装する:
`al.real-os.window`/`close-veto-approve` case を `al-gui-real-scenario`
catalog 末尾に追加。`window_geometry`（physical 寸法 + scale factor）を
観測証拠として記録し、armed `Veto` → `CloseRequested` で window 生存、
armed `Approve` → `Destroyed` で window 退役を oracle 判定する。
観測面の前提不成立・イベント未観測は `NotRun`、観測された契約違反のみ
`Fail` — headless/semantic 証拠を実機 Pass に昇格しない。

## リスク

- upstream main は今後も進む — 本 bump は `b6d0cc6a` 時点の snapshot で
  あり、以降の追従は別 rev-watch で扱う。
- `ruviform-exec` の新 `JobExecutor` は AL 未使用 — 将来の Job 基盤
  slice の選択肢として記録のみ。

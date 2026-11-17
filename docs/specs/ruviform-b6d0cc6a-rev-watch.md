---
id: "SPEC-034Z9kQmDatasetRevWatch01"
type: "spec"
status: "approved"
title: "ruviform pin b6d0cc6a 追従評価（AL#127 dataset-scenario slice）"
created: "2026-11-18"
updated: "2026-11-18"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/audio-lens/issues/127"
    - "https://github.com/scarecrow173/ruviform/issues/631"
    - "https://github.com/scarecrow173/ruviform/issues/709"
    - "https://github.com/scarecrow173/ruviform/issues/744"
  changes:
    added:
      - type: "file"
        path: "Cargo.lock"
      - type: "file"
        path: "docs/specs/ruviform-b6d0cc6a-rev-watch.md"
      - type: "file"
        path: "docs/designs/ruviform-b6d0cc6a-rev-watch.md"
      - type: "file"
        path: "docs/tasks/ruviform-b6d0cc6a-rev-watch.md"
      - type: "file"
        path: "docs/test-specs/ruviform-b6d0cc6a-rev-watch.md"
      - type: "file"
        path: "docs/impl/ir/ruviform-b6d0cc6a-rev-watch.md"
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
    - "docs/specs/ruviform-cbcc8343-rev-watch.md"
---

# ruviform pin `b6d0cc6a` 追従評価

## 背景

AL#127 dataset-scenario slice の冒頭で、ruviform pin
`cbcc83437616fdd7418da37183c6be66b6cfe225` → 最新 main への bump を
本 slice に含めるかを判定した。Coordinator 決定により **bump を含める**
ことが確定した — `scarecrow173/ruviform#709` の
RealOsGuiTest window-close/DPI 観測面が `cbcc8343` 後着であり、
`scarecrow173/audio-lens#219` 設計書の Deferred 行
（window close/DPI case）を本 slice で解放するため pin bump が必須。

## 候補

- 採用 candidate: `b6d0cc6a2e14f47e5324061d8dcac3d905e4abd5`
  （`git ls-remote` / local clone `origin/main` で再解決、2026-11-18 時点の
  upstream main HEAD）
- 旧 pin: `cbcc83437616fdd7418da37183c6be66b6cfe225`
- 差分: 61 commits、AL 依存木内の変更は additive のみ（後述）。

## 採用/保留/rollback 判定

- **採用**: `b6d0cc6a` — window-close/DPI 観測面
  （`5f0cd43f` feat(ruviform-ui-gpui): RealOsGuiTest window-close/DPI
  観測面）を含有し、AL 側の既存 API 利用は compile・test で非回帰を
  確認済み。
- **保留**: なし。
- **rollback 条件**: 本 slice の検証 lane で ruviform 起因の Fail が
  確定した場合、`cbcc8343` へ `update-ruviform-rev.mjs` で戻し、
  window-close/DPI case は Deferred に戻す。

## 影響範囲

`cbcc8343..b6d0cc6a` の crates/ 差分のうち AL 依存木に入るのは:

- `ruviform-ui-gpui`: `real_window.rs` +312（lifecycle/DPI 観測面）、
  `lib.rs` export 更新、`tests/real_window_lifecycle.rs` 新規。
- `ruviform-exec`: `executor.rs` 新規（bounded 非 RT JobExecutor、
  `76472c79` / upstream #744-#745）。
- `ruviform-audio-core`: `midi_input/mock.rs` 変更（MIDI mock —
  AL は MIDI 非提供、影響なし）。
- `ruviform-audio-test`: `metadata.rs`・test 追加（EnvironmentKind
  serde 語彙 #725、MIDI fault #707/#713、virtual-endpoint #629/#680）。
- `ruviform-io-file`: README のみ。

AL 依存木外（script/luau/plugin/device 系新規 crate 群）は lockfile
には現れるが AL の feature 木からは到達しない。`ruviform-ui` /
`ruviform-ui-test` / `ruviform-ui-theme` / `ruviform-dsp` /
`ruviform-viz` / `ruviform-audio-analysis` / `ruviform-rt` /
`ruviform-math` / `ruviform-numeric` / media/IO 系は差分なし。

## 独立検証

`evidence/al127/b6d0cc6a-verification.md` に検証 lane の実測を記録。
upstream main の covering CI 証拠は merge 前確認事項として rev-watch
記録に残す（本 slice では upstream CI run の個別確認は NotRun）。

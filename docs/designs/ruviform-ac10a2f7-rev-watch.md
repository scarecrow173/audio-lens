---
id: "DESIGN-034XLUnMxIuBKdxEIBpIL6"
type: "design"
status: "approved"
title: "ruviform ac10a2f7 pin 追従の設計"
created: "2026-11-15"
updated: "2026-11-15"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/audio-lens/issues/127"
  changes:
    added:
      - type: "file"
        path: "Cargo.lock"
      - type: "file"
        path: "ui/vendor/ruviform/REV"
      - type: "file"
        path: "docs/designs/ruviform-ac10a2f7-rev-watch.md"
    modified:
      - type: "file"
        path: "ui/vendor/ruviform"
    deleted: []
    renamed: []
    moved: []
    generated:
      - type: "file"
        path: "ui/vendor/ruviform"
  implements:
    - "../specs/ruviform-ac10a2f7-rev-watch.md"
  implemented-by: []
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/designs/ruviform-c6e4f334-rev-watch.md"
    - "docs/designs/ruviform-fd1d4d5f-rev-watch.md"
    - "docs/designs/ruviform-b02942c2-rev-watch.md"
  refines: []
  refined-by: []
  derives-from:
    - "../specs/ruviform-ac10a2f7-rev-watch.md"
  derived-by: []
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# ruviform ac10a2f7 pin 追従の設計

## Context

AudioLens は ruviform を Git 依存（manifest に `rev` を書かない
lockfile-primary 契約）で利用し、`Cargo.lock` の `source` fragment
が解決 SHA の正本。`tools/update-ruviform-rev.mjs` が
`cargo update -p <ruviform-package> --precise <sha>` を実行し、
`tools/sync-ruviform-ui.mjs`（`mise run sync-ui-deps`）が
`ui/vendor/ruviform` を lockfile SHA から再生成して `REV` を書く。
`tools/check-ruviform-rev.mjs` が lockfile/manifest/vendor の
一致を fail-closed に検査する。

候補 `ac10a2f7761c08db73aebd861c0a5c1f53f269b7` は ruviform main
HEAD（`git ls-remote` で refs/heads/main を確認、local clone
`G:/repositry/audio-products/ruviform` の `origin/main` HEAD と一致）。
前 pin `c6e4f334` から 14 commit・148 file・+22,339/−223 行の
差分。内訳は docs/ 87 file・+11,733/−37、evidence/ 18 file・
+1,510、crates/ 40 file・+9,049/−184、root `Cargo.toml`/
`Cargo.lock`/`mise.toml` +47/−2。`.github/` と `ui/` の変更は
ない。

## Scope

### In Scope

- `Cargo.lock` の ruviform 20 package を `ac10a2f7` へ更新し、
  vendor tree を再生成する。
- 差分の AudioLens 影響評価（API/Feature/OS）と検証 lane 実行。
- 採用/保留判断と rollback 情報の記録。

### Out of Scope

- 製品コード変更、ruviform 新 API/feature の採用、他 repo 変更、
  実機/実 Pixel/macOS/Linux/cross-consumer gate の実施。

## Components and Boundaries

- `tools/update-ruviform-rev.mjs` / `tools/sync-ruviform-ui.mjs` /
  `tools/check-ruviform-rev.mjs`: pin 追従と整合検査の既存機構。
  今回は機構自体を変更しない。
- `crates/al-gui`: `ruviform-ui`/`ruviform-ui-test`/
  `ruviform-ui-gpui`/`ruviform-ui-theme`/`ruviform-ui-webview` を
  消費する唯一の GUI crate。headless 既定、`webview`/`gpui`/
  `gpui-test` feature で adapter を切替。`ruviform-ui-test` の
  `ScenarioDriver::PluginHostProbe` variant は additive であり、
  AL 側は `ScenarioDriver` を `assert_eq!` 比較にのみ使用し
  exhaustive match を持たないため compile 影響なし。
- `ui/`（TypeScript）: `ui/vendor/ruviform` の vendored source を
  `binding`/`guitest` 経由で利用。vendor は生成物で直接編集しない。
  本範囲の ruviform 差分に `ui/*` の変更はなく、vendor 内容は
  REV 以外不変。
- headless crate 群（`al-core`/`al-media`/`al-dsp`/`al-audio-*`/
  `al-analysis`/`al-job`/`al-dataset`/`al-service`/`al-cli` 等）:
  `ruviform-media-*`/`io-*`/`observability`/`audio-core`/`numeric`
  等を消費。`ruviform-numeric` の変更は `mod tests` 内の型注記と
  test allocator の `#[cfg(debug_assertions)]` のみで、製品コード・
  公開 API は不変。

## Data and Control Flow

1. `node tools/update-ruviform-rev.mjs
   ac10a2f7761c08db73aebd861c0a5c1f53f269b7` で `Cargo.lock` を更新
   （全 20 package が同一 SHA に解決。差分は SHA fragment 20 行のみで
   推移的依存の変動・新規 package 追加なし）。
2. `mise run sync-ui-deps` で `ui/vendor/ruviform` を再生成し
   `REV` に SHA を記録。
3. `check-ruviform-lock`/`check-ruviform-rev` で lockfile-primary
   契約（単一 SHA・vendor 一致）を検査。
4. 各検証 lane（下記 Verification）で対象 revision の
   compile/test/lint を実行。
5. 影響評価と採用/保留判断を記録し、PR を作成する。

## 差分影響評価（c6e4f334..ac10a2f7）

14 commit の主な領域と AudioLens への影響:

| 軸 | 主な変更 | AL 影響評価 |
| --- | --- | --- |
| f64 精度 / DSP / numeric | `ruviform-numeric`（依存木内、`scarecrow173/ruviform#646` `#649`）: `src/corr.rs` は `mod tests` 内の `assert_eq!` 型注記（`[] as [f64; 0]`）のみ、`src/lib.rs` は `mod rt_tests` の `#[global_allocator]` に `#[cfg(debug_assertions)]` 付与（bench profile コンパイル不能の修正）。製品コード・公開 API の変更なし。`ruviform-math`/`dsp`/`media-*`/`io-avi`/`io-symphonia`/`viz`/`exec`/`rt`/`video-core`/`audio-core`/`audio-analysis`/`ui`/`ui-theme`/`ui-gpui`/`ui-webview`/`observability`/`io-file` に変更なし | 影響なし。f64 kernel・FFT・locate/analysis 系の数値経路は不変。`check-full`/`test-full` Pass で確認 |
| CLI / headless | 差分なし（CLI が消費する crate 群に製品コード変更なし） | `check-cli-headless` Pass。headless 境界の維持を確認 |
| GUI（`ruviform-ui-gpui`） | 差分なし | 影響なし。`check/lint/test/build-ui-gpui` Pass |
| scenario（`ruviform-ui-test`） | `scarecrow173/ruviform#631`（`#647`）: additive +201/−0。`scenario/plugin_dock.rs` 新規 module（`PLUGIN_DOCK_SCENARIO`/`PLUGIN_DOCK_SLOT_KEY`/`PluginArtifactEvidence`/`plugin_dock_spec`/`plugin_dock_view_spec` — `NativeSlot` を予約する declarative ViewSpec と `NativePluginDock`+`VisualCapture` required capability の ScenarioSpec）、`ScenarioDriver::PluginHostProbe` variant 追加（in-process plugin-host probe driver。ABI/host evidence のみで実 OS window/input/pixel の証明ではないと明記）、`lib.rs`/`scenario.rs` の export・module 追加、`tests/scenario_contract.rs` に contract test 追加。`Cargo.toml` の依存追加なし | AL は dev-dependency として公開 API を使用。enum variant 追加は additive で、AL 側に `ScenarioDriver` の exhaustive match はなく `assert_eq!` 比較のみのため compile/test 影響なし。`check-ui`/`test-ui` Pass（113 tests） |
| feature / IO 契約 | 依存木内に `ruviform-io-file`/`ruviform-io-avi`/`ruviform-io-symphonia`/`ruviform-media-io` の変更なし。依存木外の `ruviform-io-e2e`（`scarecrow173/ruviform#306` `#645`）は bench/test のみ: `benches/io_composition.rs` +646、`tests/large_staged.rs` +487（G01 実 4GiB opt-in lane・G05 性能 baseline lane）、`Cargo.toml` dev-dep/`[[bench]]` 追加、`lib.rs` +12 | 影響なし。AL の staged publish 契約（AL#143）は `ruviform-io-file` 経由で不変 |
| plugin 系（依存木外） | 新規 `ruviform-plugin-test-vst3`（`scarecrow173/ruviform#630` first slice `#641`+rustfmt `#643`）: VST3 `test_gain` fixture cdylib+rlib（`block.rs`/`component.rs`/`controller.rs`/`factory.rs`/`gain.rs`/`strings.rs`/`lib.rs` + host_path test）。新規 `ruviform-plugin-dock-test`（`scarecrow173/ruviform#631` `#647`+`#652` `#389` session-lifecycle cases）: test-only plugin-dock scenario harness（`artifact.rs`/`probe.rs`/`scenario.rs` 1,632 行/`bin/plugin-dock-scenario.rs`/tests）。`ruviform-plugin-core`（`event.rs` +226）・`ruviform-audio-plugin-bridge`（`input.rs`/`node.rs`/`status.rs`/`tests/midi_delivery.rs`）に V9+V11+V12 MIDI event contract 修正（`scarecrow173/ruviform#253` `#644`） | AL 依存木外（lockfile 20 package に非含有、AL は Plugin Host を提供しない）。root `Cargo.toml` の member 追加コメントに「本番クレートからは誰も依存しない」と明記。影響なし |
| device / audio-test（依存木外） | `ruviform-audio-device`/`ruviform-midi-device`（`scarecrow173/ruviform#629` `#640`）: `examples/endpoint_probe.rs` +667・`examples/midi_endpoint_probe.rs` +395 — Windows 仮想 Audio/MIDI Endpoint capability probe first slice。`ruviform-audio-test`（`#631` commit `#647` 内）: `metadata.rs` で `#[non_exhaustive]` な `EnvironmentKind` に `RealAbiTestPlugin`/`RealWindow` variant と `as_str()` を追加（plugin-dock-test 等の consumer harness が実環境実行を主張するための語彙）+ `lib.rs` で metadata 型の crate root re-export。enum は non_exhaustive のため variant 追加は非破壊 | AL 依存木外。probe は example binary で library API ではない。影響なし |
| CI/mise/policy（ruviform 側） | `mise.toml` +14（`scarecrow173/ruviform#650` `#651`）: `bench-smoke` lane 対象を `ruviform-numeric`/`ruviform-io-e2e` へ拡張 + 新規 manual/opt-in `test-ui-plugin-dock` lane（`rf631.plugin-dock` cases を driver-neutral scenario contract 経由で実行し `evidence/rf631/` へ JSONL 記録。CI-required gate ではないと明記）。`.github/` 変更なし。docs/ は `scarecrow173/ruviform#192`/`#193`/`#275` の Virtual-first Owner remap（`#653`/`#654`/`#655`）と `#389` verification-status reconcile（`#652`）、crates index rows（`#630`/`#631`） | AL 側 CI 定義・manifest への直接影響なし。`check-ci-workflow`/`check-ci-tracker-workflows` Pass。virtual-first 契約は AL の POLICIES.md #212 と同方向 |

結論: 破壊的 API/Feature/OS 変更は検出されず、AL の全検証 lane が
対象 revision で pass。新 API/feature（`ruviform-plugin-dock-test`・
`ruviform-plugin-test-vst3`・`ruviform-ui-test` の plugin_dock
scenario/`PluginHostProbe`・endpoint probe examples・`io-e2e`
opt-in lanes・plugin-core V9+V11+V12 MIDI event contract）の採用は
本 slice では行わない（test-only/opt-in/依存木外であり、AL 側の
採用 gate・需要が未定義のため）。採用判断は「採用」（pin 追従
自体。詳細は task doc と PR 本文）。

## Non-Functional Constraints

- lockfile-primary 契約の維持（manifest `rev` 固定への回帰禁止）。
- fail-closed 検査（check-ruviform-lock/rev）の有効性維持。
- 検証は正規 mise task 経由。cargo 直実行は補助証拠として区別。
- 未実行 gate を Pass にしない（NotRun/Unsupported 語彙の厳守）。

## Risks and Trade-offs

- upstream に plugin-dock scenario harness・VST3 test fixture・
  endpoint probe・io-e2e opt-in lane・MIDI event contract 修正など
  新機能が入ったが AL lane では未踏の領域 → 採用は別 slice の
  判断とし、本 slice では pin 追従の非回帰のみを検証。
- `ruviform-ui-test` の `ScenarioDriver` enum に variant が追加
  されたが additive であり、AL 側に exhaustive match は存在しない
  ことを確認済み。将来 AL が `ScenarioDriver` を match する場合は
  新 variant の扱いが必要になる（記録のみ）。
- rollback は `Cargo.lock` revert + `sync-ui-deps` 再実行で可能
  （生成物のみの変更、製品コード差分なし）。

## Deferred Design Concerns

- `ruviform-plugin-dock-test`・`ruviform-plugin-test-vst3`・
  `ruviform-ui-test` の plugin_dock scenario/`PluginHostProbe` の
  AL 採用設計 | AL#127 の GUI-L2/EXT-L3 要件と接続するが採用
  slice・独立 Oracle 接続が未定義 | 採用判断 slice の spec 起票時
  | 実機相当の証拠なしに semantic/headless pass を実機 acceptance
  と誤認 | 本 slice では NotRun 記録のみ
- `ruviform-{audio,midi}-device` endpoint probe・
  `ruviform-io-e2e` opt-in lanes・plugin-core MIDI event contract
  の AL 採用 | AL は本番 OS API/Plugin Host を提供せず需要未定 |
  採用 Issue 起票時 | upstream 機能の重複実装リスク | 影響評価に
  記録済み

## Verification Notes

- [x] `mise run check-ruviform-lock` / `check-ruviform-rev`:
  単一 SHA・vendor 一致（fail-closed）
- [x] `mise run sync-ui-deps` → `check-ui` → `test-ui`:
  対象 SHA で Pass（113 tests）
- [x] `mise run fmt`/`check-full`/`lint-full`/`test-full`/
  `build-full`: workspace 全体 Pass
- [x] `mise run check-cli-headless`: headless 境界 Pass
- [x] `mise run check-ui-gpui`/`lint-ui-gpui`/`test-ui-gpui`/
  `build-ui-gpui`: GPUI feature lane Pass
- [x] `mise run check-ui-webview`/`lint-ui-webview`/
  `test-ui-webview`: Legacy WebView lane Pass
- [x] `mise run check-ci-workflow`/`check-ci-tracker-workflows`/
  `verify-toolchain`/`cargo fetch --locked`: Pass

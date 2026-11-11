---
id: "DESIGN-034VFT8jbz528XiW8ZauIH"
type: "design"
status: "approved"
title: "ruviform fd1d4d5f pin 追従の設計"
created: "2026-11-11"
updated: "2026-11-11"
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
        path: "docs/designs/ruviform-fd1d4d5f-rev-watch.md"
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
    - "../specs/ruviform-fd1d4d5f-rev-watch.md"
  implemented-by: []
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/designs/ruviform-b02942c2-rev-watch.md"
  refines: []
  refined-by: []
  derives-from:
    - "../specs/ruviform-fd1d4d5f-rev-watch.md"
  derived-by: []
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# ruviform fd1d4d5f pin 追従の設計

## Context

AudioLens は ruviform を Git 依存（manifest に `rev` を書かない
lockfile-primary 契約）で利用し、`Cargo.lock` の `source` fragment
が解決 SHA の正本。`tools/update-ruviform-rev.mjs` が
`cargo update -p <ruviform-package> --precise <sha>` を実行し、
`tools/sync-ruviform-ui.mjs`（`mise run sync-ui-deps`）が
`ui/vendor/ruviform` を lockfile SHA から再生成して `REV` を書く。
`tools/check-ruviform-rev.mjs` が lockfile/manifest/vendor の
一致を fail-closed に検査する。

候補 `fd1d4d5f4de57092b96ad1cd309b85fe47cbef38` は ruviform main
HEAD（`git ls-remote` で refs/heads/main・HEAD と一致を確認済み、
local clone `F:/repositry/audio-products/ruviform` の
`origin/main` HEAD と一致）。前 pin `b02942c2` から 7 commit・
110 file・+12,674/−301 行の差分（大半は ruviform 側
docs/evidence で、製品コード差分は限定的）。

## Scope

### In Scope

- `Cargo.lock` の ruviform 20 package を `fd1d4d5f` へ更新し、
  vendor tree を再生成する。
- 差分の AudioLens 影響評価（API/Feature/OS）と検証 lane 実行。
- 採用/保留判断と rollback 情報の記録。

### Out of Scope

- 製品コード変更、ruviform 新 API の採用、他 repo 変更、
  実機/実 Pixel/macOS/Linux/cross-consumer gate の実施。

## Components and Boundaries

- `tools/update-ruviform-rev.mjs` / `tools/sync-ruviform-ui.mjs` /
  `tools/check-ruviform-rev.mjs`: pin 追従と整合検査の既存機構。
  今回は機構自体を変更しない。
- `crates/al-gui`: `ruviform-ui`/`ruviform-ui-test`/
  `ruviform-ui-gpui`/`ruviform-ui-theme`/`ruviform-ui-webview` を
  消費する唯一の GUI crate。headless 既定、`webview`/`gpui`/
  `gpui-test` feature で adapter を切替。`ruviform-ui-gpui` の
  `real-scenario` feature は AL から有効化しない（real-window
  OS input ops は `#[cfg(feature = "real-scenario")]` 配下で
  AL ビルドにはコンパイルされない）。
- `ui/`（TypeScript）: `ui/vendor/ruviform` の vendored source を
  `binding`/`guitest` 経由で利用。vendor は生成物で直接編集しない。
  本範囲の ruviform 差分に `ui/*` の変更はなく、vendor 内容は
  REV 以外不変。
- headless crate 群（`al-core`/`al-media`/`al-dsp`/`al-audio-*`/
  `al-analysis`/`al-job`/`al-dataset`/`al-service`/`al-cli` 等）:
  `ruviform-media-*`/`io-*`/`observability`/`audio-core` 等を消費。
  `ruviform-audio-core` の `midi_input` mock additive は AL が
  MIDI を使わないため非該当。

## Data and Control Flow

1. `node tools/update-ruviform-rev.mjs fd1d4d5f4de57092b96ad1cd309b85fe47cbef38`
   で `Cargo.lock` を更新（全 20 package が同一 SHA に解決。
   差分は SHA fragment 20 行＋`ruviform-ui-gpui` の依存に
   `x11rb` が 1 行追加）。
2. `mise run sync-ui-deps` で `ui/vendor/ruviform` を再生成し
   `REV` に SHA を記録。
3. `check-ruviform-lock`/`check-ruviform-rev` で lockfile-primary
   契約（単一 SHA・vendor 一致）を検査。
4. 各検証 lane（下記 Verification）で対象 revision の
   compile/test/lint を実行。
5. 影響評価と採用/保留判断を記録し、PR を作成する。

## 差分影響評価（b02942c2..fd1d4d5f）

7 commit（ruviform PR #564/#567/#566/#568/#569/#570/#571）の
主な領域と AudioLens への影響:

| 軸 | 主な変更 | AL 影響評価 |
| --- | --- | --- |
| f64 精度 / DSP / numeric | 差分なし（`ruviform-math`/`numeric`/`dsp`/`media-*`/`io-*`/`viz`/`observability`/`exec`/`rt`/`video-core`/`audio-analysis`/`ui`/`ui-theme`/`ui-webview`/`ui-test` に変更なし） | 影響なし。f64 kernel・FFT・locate/analysis 系の数値経路は不変 |
| CLI / headless | 差分なし（CLI が消費する crate 群に変更なし） | `check-cli-headless` Pass。headless 境界の維持を確認 |
| GUI（`ruviform-ui-gpui`） | `capture.rs`: OS-window pixel capture backend の追加（Windows `PrintWindow`/`BitBlt` + Linux X11 `capture_client_area`/`client_geometry`、いずれも `pub(super)`）。`os_input.rs`: `OsInputOutcome` enum（`#[non_exhaustive]`）と `families()` の additive。`real_window.rs`: `os_input_preflight`/`window_focused`/`request_focus`/`os_press_key`/`os_pointer_move_to`/`os_mouse_down`/`os_mouse_up` の追加（同 module は `#[cfg(feature = "real-scenario")]` 配下）。`surface.rs`: `raw_parent_of`（`pub(super)`）。`Cargo.toml`: `gpui` feature の dep に `x11rb` 追加（`cfg(target_os = "linux")` target-gated optional dep）。既存公開 API の削除・変更なし | AL は `gpui`/`gpui-test` feature のみ使用し `real-scenario` を有効化しないため、real-window OS input ops は AL ビルドにコンパイルされない。`gpui` 配下の additive（`OsInputOutcome` 等）は公開 API 追加のみで既存呼び出しに影響なし。`x11rb` は linux-target optional dep で Windows ビルドでは解決のみ・コンパイル対象外（既に `gpui`/`winit` 由来で lockfile に存在）。`check/lint/test-ui-gpui` Pass |
| scenario（`ruviform-ui-test`） | 差分なし | AL の SemanticGuiTest/VisualGuiTest 利用面に変更なし。`check-ui`/`test-ui` Pass |
| feature / audio-core | `ruviform-audio-core` の `midi_input` mock に additive: `MockPort::inject_unsupported`（受信済みだが channel-voice 語彙に翻訳不能なメッセージを `MidiReadStats::unsupported` 経由で表面化する script API）。`MidiReadStats::new`/`with_unsupported` 契約自体は前範囲で導入済みで変更なし | AL は MIDI を一切使用せず（`crates/` 内に midi 参照なし）。mock のみの additive で影響なし |
| audio-device（依存木外） | `ruviform-audio-device` に `wasapi_input.rs` 新規: WASAPI-native input path で `frame_position` を供給（RF#565）。WASAPI callback timestamp 修正と Stage W 実機 lane closeout（RF#251） | AL 依存木外（`Cargo.lock` の 20 package に非含有）。影響なし |
| plugin 系（依存木外） | `ruviform-plugin-core` の `wire.rs` に L1 report verb `drain_output_reports` 実装、`ruviform-plugin-export`/`plugin-host` の runner/composite/l1 配線（RF#253）。`ruviform-audio-plugin-bridge` の `input.rs`/`status.rs` で unsupported 計数を bridge status に透過（RF#253 residual） | AL 依存木外。影響なし |
| midi-device（依存木外） | `ruviform-midi-device` の test・README 更新のみ | AL 依存木外。影響なし |
| CI/mise（ruviform 側） | ruviform `.github/workflows/ci.yml`・`ci/summary`・`mise.toml`: GPUI lane の CI 収載・Linux offscreen golden lane・opt-in 実機 lane scaffold | AL 側 CI 定義への直接影響なし。`check-ci`/`lint-ci`/`test-ci` Pass |

結論: 破壊的 API/Feature/OS 変更は検出されず、AL の全検証 lane が
対象 revision で pass。新 API（`real-scenario` 配下の OS input
ops・`OsInputOutcome`・SendInput preflight session cache・Linux
offscreen golden・L1 wire report verb・WASAPI-native input・
bridge unsupported 透過）の採用は本 slice では行わない
（opt-in/別ドメインであり、AL 側の採用 gate・需要が未定義の
ため）。採用判断は「採用」（pin 追従自体。詳細は task doc と
PR 本文）。

## Non-Functional Constraints

- lockfile-primary 契約の維持（manifest `rev` 固定への回帰禁止）。
- fail-closed 検査（check-ruviform-lock/rev）の有効性維持。
- 検証は正規 mise task 経由。cargo 直実行は補助証拠として区別。
- 未実行 gate を Pass にしない（NotRun/Unsupported 語彙の厳守）。

## Risks and Trade-offs

- upstream に real OS window/pixel capture・OS input・WASAPI
  input・plugin report・MIDI bridge の新機能が入ったが AL lane
  では未踏の領域 → 採用は別 slice の判断とし、本 slice では
  pin 追従の非回帰のみを検証。
- `proc-macro-error2 v2.0.1` の future-incompatibility warning が
  lint 時に出力される（依存由来、抑制せず記録のみ）。
- rollback は `Cargo.lock` revert + `sync-ui-deps` 再実行で可能
  （生成物のみの変更、製品コード差分なし）。

## Deferred Design Concerns

- `real-scenario` feature・OS input ops（RealOsPointerOrKeyboard）
  の AL 採用設計 | AL の独立精度 gate・採用 slice が未定義 |
  採用判断 slice の spec 起票時 | 実機相当の証拠なしに semantic
  pass を実機 acceptance と誤認 | 本 slice では NotRun 記録のみ
- WASAPI-native input frame_position / L1 wire report verb /
  bridge unsupported 透過の AL 採用 | AL の依存木・製品要件に
  未接続 | 各採用 Issue 起票時 | upstream 機能の重複実装
  リスク | 影響評価に記録済み

## Verification Notes

- [x] `mise run check-ruviform-lock` / `check-ruviform-rev`:
  単一 SHA・vendor 一致（fail-closed）
- [x] `mise run sync-ui-deps` → `check-ui` → `test-ui`:
  対象 SHA で Pass（113 tests）
- [x] `mise run fmt`/`check`/`lint`/`test`/`build`/
  `check-cli-headless`: workspace 全体 Pass
- [x] `mise run check-ui-gpui`/`lint-ui-gpui`/`test-ui-gpui`:
  GPUI feature lane Pass
- [x] `mise run check-ui-webview`/`lint-ui-webview`/
  `test-ui-webview`: Legacy WebView lane Pass
- [x] `mise run check-ci`/`lint-ci`/`test-ci`: CI profile Pass

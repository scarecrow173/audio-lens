---
id: "DESIGN-034XCQM1h8KbyVH2VDHQNl"
type: "design"
status: "approved"
title: "ruviform c6e4f334 pin 追従の設計"
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
        path: "docs/designs/ruviform-c6e4f334-rev-watch.md"
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
    - "../specs/ruviform-c6e4f334-rev-watch.md"
  implemented-by: []
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/designs/ruviform-fd1d4d5f-rev-watch.md"
    - "docs/designs/ruviform-b02942c2-rev-watch.md"
  refines: []
  refined-by: []
  derives-from:
    - "../specs/ruviform-c6e4f334-rev-watch.md"
  derived-by: []
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# ruviform c6e4f334 pin 追従の設計

## Context

AudioLens は ruviform を Git 依存（manifest に `rev` を書かない
lockfile-primary 契約）で利用し、`Cargo.lock` の `source` fragment
が解決 SHA の正本。`tools/update-ruviform-rev.mjs` が
`cargo update -p <ruviform-package> --precise <sha>` を実行し、
`tools/sync-ruviform-ui.mjs`（`mise run sync-ui-deps`）が
`ui/vendor/ruviform` を lockfile SHA から再生成して `REV` を書く。
`tools/check-ruviform-rev.mjs` が lockfile/manifest/vendor の
一致を fail-closed に検査する。

候補 `c6e4f334242567dbd0e0d8b7713d9d480a6aab66` は ruviform main
HEAD（`git ls-remote` で refs/heads/main を確認、local clone
`G:/repositry/audio-products/ruviform` の `origin/main` HEAD と一致）。
前 pin `fd1d4d5f` から 114 commit・892 file・+303,172/−2,270 行の
差分（大半は ruviform 側 docs/evidence/poc/ffi-poc で、`crates/` 配下は
75 file・+9,161/−475 行）。

## Scope

### In Scope

- `Cargo.lock` の ruviform 20 package を `c6e4f334` へ更新し、
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
  `gpui-test` feature で adapter を切替。`ruviform-ui-gpui` の
  `real-scenario`/`uitest` feature は AL から有効化しない
  （windowed TestIo・real-window OS input ops は各 feature 配下で
  AL ビルドにはコンパイルされない）。
- `ui/`（TypeScript）: `ui/vendor/ruviform` の vendored source を
  `binding`/`guitest` 経由で利用。vendor は生成物で直接編集しない。
  本範囲の ruviform 差分に `ui/*` の変更はなく、vendor 内容は
  REV 以外不変。
- headless crate 群（`al-core`/`al-media`/`al-dsp`/`al-audio-*`/
  `al-analysis`/`al-job`/`al-dataset`/`al-service`/`al-cli` 等）:
  `ruviform-media-*`/`io-*`/`observability`/`audio-core` 等を消費。
  `ruviform-io-file` の `fault-injection` feature は非既定で AL は
  有効化しないため `mock` module の公開化は AL ビルドに影響しない。

## Data and Control Flow

1. `node tools/update-ruviform-rev.mjs
   c6e4f334242567dbd0e0d8b7713d9d480a6aab66` で `Cargo.lock` を更新
   （全 20 package が同一 SHA に解決。差分は SHA fragment 20 行のみで
   推移的依存の変動・新規 package 追加なし）。
2. `mise run sync-ui-deps` で `ui/vendor/ruviform` を再生成し
   `REV` に SHA を記録。
3. `check-ruviform-lock`/`check-ruviform-rev` で lockfile-primary
   契約（単一 SHA・vendor 一致）を検査。
4. 各検証 lane（下記 Verification）で対象 revision の
   compile/test/lint を実行。
5. 影響評価と採用/保留判断を記録し、PR を作成する。

## 差分影響評価（fd1d4d5f..c6e4f334）

114 commit の主な領域と AudioLens への影響:

| 軸 | 主な変更 | AL 影響評価 |
| --- | --- | --- |
| f64 精度 / DSP / numeric | 差分なし（`ruviform-math`/`numeric`/`dsp`/`media-*`/`io-avi`/`io-symphonia`/`viz`/`exec`/`rt`/`video-core`/`audio-core`/`audio-analysis`/`ui`/`ui-theme` に変更なし） | 影響なし。f64 kernel・FFT・locate/analysis 系の数値経路は不変 |
| CLI / headless | 差分なし（CLI が消費する crate 群に変更なし） | `check-cli-headless` Pass。headless 境界の維持を確認 |
| GUI（`ruviform-ui-gpui`） | 新 feature `uitest`（Issue #313 S2）: `gpui_driver/windowed_io.rs` 新規（657 行、windowed `TestIo` adapter）、`src/bin/windowed_test_app.rs` 新規（`rvf.uitest.v1` AUT binary）、`tests/windowed_uitest.rs` 新規。`UiJob`/`set_ui_jobs`/`drain_ui_jobs` の cfg を `real-scenario` → `any(real-scenario, uitest)` へ拡張（内部型）。`lib.rs` に `#[cfg(feature = "uitest")]` の `UiSide`/`WindowedIo` export 追加。`capture.rs`/`surface.rs` は comment 修正のみ、`render.rs` は test 関数への `#[cfg(target_os = "windows"]` 付与のみ。`Cargo.toml` に optional dep `base64`（`uitest` feature のみ）と `[[bin]] windowed_test_app`（required-features = uitest）追加 | AL は `gpui`/`gpui-test` feature のみ使用し `uitest`/`real-scenario` を有効化しないため、windowed TestIo・OS input ops は AL ビルドにコンパイルされない。公開 API は additive のみで既存呼び出しに影響なし。`check/lint/test/build-ui-gpui` Pass |
| scenario（`ruviform-ui-test`） | `lib.rs` の公開 export は不変。内部変更: `driver.rs` +178（Issue #625 hello channel-death の Handshake 分類正規化・#388 `DriverError::Timeout` coverage・stall-fixture kill contract）、`headless.rs` 再構成 177 行、`io.rs`/`journal.rs`/`fixture.rs`/`fake_driver.rs`/`agent.rs`/`dummy_app.rs` の内部更新。新 test: `window_job_headless.rs`（Issue #243 Window/Job invariant on Fake/Headless）、`e2e_process.rs`、`headless_contract.rs`/`driver_contract.rs` 更新 | AL は dev-dependency として `SemanticGuiTest`/`VisualGuiTest`/`golden`/`FakeGuiTest`/`FakeDriver` 等の公開 API を使用。公開 export に変更がなく workspace test が Pass することを確認。`check-ui`/`test-ui` Pass |
| feature / IO 契約（`ruviform-io-file`） | 新 feature `fault-injection`（Issue #306 G07）: `#[cfg(any(test, feature = "fault-injection"))]` で `mock` module を `#[doc(hidden)] pub` 化し、`staged.rs` の `with_ops` cfg を同条件へ拡張。`PlatformFileOps` 境界を script 化する test-only seam で、公開 API・default build・RT surface は不変 | AL は default feature のみ使用（`al-job` の staged publish 契約 AL#143）。feature 非有効のため `mock` 公開化は AL ビルドに影響なし。`check/test` Pass |
| observability | `tests/logging.rs` のみ: rolling file retention test の seed file 書き込みに `SEED_FILE_BIRTH_GAP`（100ms）sleep を追加（tracing-appender の birth-time 順序に起因する CI flake 修正）。製品コード変更なし | 影響なし。AL の `al-observability` 経路は不変 |
| webview（`ruviform-ui-webview`） | `src/lib.rs` に doc comment のみ追加（Issue #465: legacy/preview 分類の明記）。コード変更なし | 影響なし。`check/lint/test-ui-webview` Pass で Legacy WebView lane 維持を確認 |
| audio-test（新規・依存木外） | `crates/ruviform-audio-test` 新規 crate（Issue #628 first slice #638）: 決定論的 test runtime core — `clock.rs`（VirtualClock）・`pcm.rs`（PCM fixture）・`midi.rs`（MIDI trace）・`oracle.rs`（独立 oracle）・`metadata.rs`（run metadata）。audio-core/media-core/rt の公開契約のみ消費し、本番クレートからは誰も依存しない test-only 基盤 | AL 依存木外（lockfile の 20 package に非含有、誰も依存しない）。影響なし。AL#127 の Windows L2/L3 要件が想定する fixture/runner の upstream 側実装として記録 |
| audio-device / audio（依存木外） | `ruviform-audio-device`: `wasapi_input.rs` +15 行・`real_input_e2e.rs` +282 行（Issue #251 Stage W 系）。`ruviform-audio`: `alternate_evaluation.rs` test +1,320 行（Issue #562 RF-CB-01 headless 2候補 alternate evaluation PoC） | AL 依存木外。影響なし |
| io-e2e / plugin 系 / misc（依存木外） | `ruviform-io-e2e`: fault seam contract test・corpus replay lane（Issue #306）。`ruviform-plugin-clap`: `host.rs` +197 行 — CLAP L0 editor-attach `open_gui`/`close_gui` via `clap.ext.gui` + poc-clap-editor lane（Issue #389 #637）。`ruviform-plugin-core`/`plugin-host`/`plugin-vst3`: ParentLease 語彙移行・editor_session 配線（Issue #389）。`ruviform-midi-device`: README のみ（Issue #253）。`ruviform-poc-native-dock`: poc 更新。`ruviform-video-proc`: `icc.rs` 変更 | AL 依存木外（lockfile 非含有）。影響なし。CLAP editor-attach は AL が Plugin Host を提供しないため需要未定 |
| CI/mise/policy（ruviform 側） | `.github/workflows/ci.yml` +518・`consumer-verify.yml` 再構成（Issue #294 schema v2 #639・#598 ui-gpui-verify-macos lane・Linux lint 集約 #608・MSRV lane #594）。`mise.toml` +200（host-feature check task・mr_boxington・package-scoped task）。`POLICIES.md`/`AGENTS.md` の virtual-first 検証契約整合（Issue #634/#635）。`ffi-poc/` 新規 workspace member（RF-SCRIPT-01/02 Luau+Rhai PoC） | AL 側 CI 定義・manifest への直接影響なし。`check-ci-workflow`/`check-ci-tracker-workflows` Pass。virtual-first 契約は AL の POLICIES.md #212 と同方向 |

結論: 破壊的 API/Feature/OS 変更は検出されず、AL の全検証 lane が
対象 revision で pass。新 API/feature（`ruviform-audio-test`・
`ruviform-ui-gpui` の `uitest`/windowed TestIo・`ruviform-io-file`
の `fault-injection`・`ruviform-plugin-clap` editor-attach・
`ruviform-ui-test` の Window/Job 拡張）の採用は本 slice では行わない
（opt-in/別ドメインであり、AL 側の採用 gate・需要が未定義のため）。
採用判断は「採用」（pin 追従自体。詳細は task doc と PR 本文）。

## Non-Functional Constraints

- lockfile-primary 契約の維持（manifest `rev` 固定への回帰禁止）。
- fail-closed 検査（check-ruviform-lock/rev）の有効性維持。
- 検証は正規 mise task 経由。cargo 直実行は補助証拠として区別。
- 未実行 gate を Pass にしない（NotRun/Unsupported 語彙の厳守）。

## Risks and Trade-offs

- upstream に deterministic test runtime・windowed TestIo・
  fault-injection・CLAP editor・plugin ParentLease 移行など大規模な
  新機能が入ったが AL lane では未踏の領域 → 採用は別 slice の判断と
  し、本 slice では pin 追従の非回帰のみを検証。
- `ruviform-ui-test` の内部再構成は公開 export 不変を確認したが、
  AL の dev-dependency 経路（`SemanticGuiTest`/`VisualGuiTest`/
  `golden`）の実動作は workspace test の Pass で担保。
- rollback は `Cargo.lock` revert + `sync-ui-deps` 再実行で可能
  （生成物のみの変更、製品コード差分なし）。

## Deferred Design Concerns

- `ruviform-audio-test`・`ruviform-ui-gpui` `uitest`/
  `real-scenario`・`ruviform-io-file` `fault-injection` の AL 採用
  設計 | AL#127 の L2/L3 要件と接続するが採用 slice・独立 Oracle
  接続が未定義 | 採用判断 slice の spec 起票時 | 実機相当の証拠なしに
  semantic/headless pass を実機 acceptance と誤認 | 本 slice では
  NotRun 記録のみ
- `ruviform-plugin-clap` editor-attach・plugin ParentLease 移行の
  AL 採用 | AL は Plugin Host を提供せず需要未定 | 採用 Issue
  起票時 | upstream 機能の重複実装リスク | 影響評価に記録済み

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

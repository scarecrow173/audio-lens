---
id: "DESIGN-034YnH7oRHxqGNv9Fl5qMK"
type: "design"
status: "approved"
title: "ruviform cbcc8343 pin 追従の設計"
created: "2026-11-16"
updated: "2026-11-16"
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
        path: "docs/designs/ruviform-cbcc8343-rev-watch.md"
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
    - "../specs/ruviform-cbcc8343-rev-watch.md"
  implemented-by: []
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/designs/ruviform-eafe0a08-rev-watch.md"
    - "docs/designs/ruviform-ac10a2f7-rev-watch.md"
  refines: []
  refined-by: []
  derives-from:
    - "../specs/ruviform-cbcc8343-rev-watch.md"
  derived-by: []
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# ruviform cbcc8343 pin 追従の設計

## Context

AudioLens は ruviform を Git 依存（manifest に `rev` を書かない
lockfile-primary 契約）で利用し、`Cargo.lock` の `source` fragment
が解決 SHA の正本。`tools/update-ruviform-rev.mjs` が
`cargo update -p <ruviform-package> --precise <sha>` を実行し、
`tools/sync-ruviform-ui.mjs`（`mise run sync-ui-deps`）が
`ui/vendor/ruviform` を lockfile SHA から再生成して `REV` を書く。
`tools/check-ruviform-rev.mjs` が lockfile/manifest/vendor の
一致を fail-closed に検査する。

候補 `cbcc83437616fdd7418da37183c6be66b6cfe225` は ruviform main
HEAD（`git ls-remote https://github.com/scarecrow173/ruviform.git
refs/heads/main` と local clone `G:/repositry/audio-products/
ruviform` の `origin/main` HEAD で一致確認。Spec 起稿時点と同一
SHA で upstream の進行なし）。前 pin `eafe0a08`（PR #216 で採用）
から 6 commit・97 file・+15,611/−581 行の差分。内訳は crates/
（9 crate）・docs/・evidence/・root `Cargo.lock`・`mise.toml`。
`.github/` と ruviform 側 `ui/`（TS）の変更はない。

## Scope

### In Scope

- `Cargo.lock` の ruviform 26 package を `cbcc8343` へ更新し、
  vendor tree を再生成する。
- 差分の AudioLens 影響評価（API/Feature/OS）と検証 lane 実行。
- 依存木内変更（record codec・内部 callsite・内部リファクタ）と
  依存木外新機能（instrument fixture・endpoint probe・Test Lab
  GUI・DComp pixel regression）の採用/見送り判断と rollback 情報の
  記録。
- upstream main CI 証拠（`cbcc8343` 対象）の確認と記録。

### Out of Scope

- 製品コード変更、ruviform 新 API/feature の採用、他 repo 変更、
  実機/実 Pixel/macOS/Linux/cross-consumer gate の実施。

## Components and Boundaries

- `tools/update-ruviform-rev.mjs` / `tools/sync-ruviform-ui.mjs` /
  `tools/check-ruviform-rev.mjs`: pin 追従と整合検査の既存機構。
  今回は機構自体を変更しない。
- `crates/al-analysis`: `ruviform-audio-test`（dev-dependency、
  RF#628 consumer oracle `tests/rf628_oracle.rs` が
  `pcm`/`oracle` module を消費）、`ruviform-plugin-core`/
  `ruviform-plugin-host`（推移的依存）等を消費する変更対象
  crate。本範囲で製品コードの変更はない。
- `crates/al-gui` 他: `ruviform-ui*`/`ruviform-media-*`/`io-*` 等を
  消費するが、本範囲の ruviform 差分にこれらの crate の変更はない
  （`ruviform-ui`/`ui-gpui`/`ui-test`/`ui-theme`/`ui-webview`/
  `viz`/`media-*`/`io-*`/`numeric`/`dsp`/`math`/`exec`/`rt`/
  `observability`/`audio`/`audio-analysis`/`audio-core`/`graph`/
  `audio-plugin-bridge`/`video-core` は無変更）。
- `ui/`（TypeScript）: `ui/vendor/ruviform` の vendored source を
  `binding`/`guitest` 経由で利用。vendor は生成物で直接編集しない。
  本範囲の ruviform 差分に `ui/*` の変更はなく、vendor 内容は
  REV 以外不変。

## Data and Control Flow

1. `node tools/update-ruviform-rev.mjs
   cbcc83437616fdd7418da37183c6be66b6cfe225` で `Cargo.lock` を更新
   （全 26 package が同一 SHA に解決。差分は SHA fragment 26 行
   のみで推移的依存の version 変動・新規 package 追加・依存 edge
   変動なし — in-tree crate の `Cargo.toml` に変更がないため）。
2. `mise run sync-ui-deps` で `ui/vendor/ruviform` を再生成し
   `REV` に SHA を記録。
3. `check-ruviform-lock`/`check-ruviform-rev` で lockfile-primary
   契約（単一 SHA・vendor 一致）を検査。
4. 各検証 lane（下記 Verification）で対象 revision の
   compile/test/lint を実行。
5. 影響評価と採用/保留判断を記録し、PR を作成する。

## 差分影響評価（eafe0a08..cbcc8343）

6 commit の主な領域と AudioLens への影響:

| 軸 | 主な変更 | AL 影響評価 |
| --- | --- | --- |
| f64 精度 / DSP / numeric / media / io | 差分なし（`ruviform-dsp`/`math`/`numeric`/`media-*`/`io-*`/`viz`/`exec`/`rt`/`video-core`/`audio`/`audio-analysis`/`audio-core`/`graph`/`audio-plugin-bridge`/`observability`/`ui*` に変更なし） | 影響なし。f64 kernel・FFT・locate/analysis 系の数値経路は不変。`check`/`test al-analysis` Pass で確認 |
| CLI / headless | 差分なし（CLI が消費する crate 群に製品コード変更なし） | `check-cli-headless` Pass。headless 境界の維持を確認 |
| GUI（`ruviform-ui*`） | 差分なし | 影響なし。`check-ui`/`test-ui` Pass（113 tests） |
| `ruviform-plugin-core`（依存木内、推移的依存） | `de3ed4e8`（`scarecrow173/ruviform#630` `#676` test_instrument）: `wire.rs` +41/−2 — shm slot layout の `EVENT_WIRE_SIZE` doc 更新と、固定長 record codec `encode_event_record`/`decode_event_record` を additive 追加（`encode_event` encoding を `EVENT_WIRE_SIZE` stride に zero-pad する record 版。kind payload が stride を超える kind は slot で表現不可、非零 pad は `WireError::Malformed` で reject）。既存 `encode_event`/`decode_event`/定数は不変 | additive 公開 API 追加のみ。AL は plugin wire codec を直接消費しない（`plugin-core` は `plugin-host` 経由の推移的依存）。`check`/`lint`/`test al-analysis` Pass で非回帰を確認。採用判断: 見送り（後述） |
| `ruviform-plugin-host`（依存木内、推移的依存） | `de3ed4e8`: `l1/data.rs` +1/−1 — `RtLeg` の event 書き込み callsite を `wire::encode_event` → `wire::encode_event_record` へ更新（slot record が固定 stride であることを codec 側で保証する内部修正） | 内部 callsite のみ、公開 API 不変。AL は plugin-host L1 経路を消費しない。`check`/`test al-analysis` Pass |
| dev-dep `ruviform-audio-test`（依存木内、AL#127 RF#628 oracle の入力基盤） | `de3ed4e8`: `backend/stream.rs` +39/−17 — input callback の error delivery（fault errors・gap Xrun・StreamInvalidated）を非公開 fn `deliver_input_errors` へ抽出する内部リファクタ。公開 API・`PumpReport` 契約は不変 | `pub(crate)` 内部実装のみ。AL の `rf628_oracle.rs` は `pcm`/`oracle` module のみ消費し影響なし。`check`/`lint`/`test al-analysis` Pass（`rf628_oracle` 7 tests・長尺 scenario 実走） |
| plugin adapter / test fixture（依存木外） | `de3ed4e8`（`scarecrow173/ruviform#630` `#676` test_instrument）: `ruviform-plugin-clap` `host.rs` +277/−34（instrument 対応の host 側拡張）、`ruviform-plugin-export` `lib.rs` +4/−3、`ruviform-plugin-test-clap`（`instrument.rs` +474・`instrument_core.rs` +797・`block.rs` +326/−5・`tests/l1_real_test_plugin_e2e.rs` +932・`tests/host_path.rs` +85/−7）、`ruviform-plugin-test-vst3`（`instrument.rs` +831・`events.rs` +364 新規・`component.rs` +128/−31・`factory.rs` +128/−241 再構成・`tests/l1_real_test_plugin_e2e.rs` +935・`tests/host_path.rs` +94/−10）。`7956bfc4`（`scarecrow173/ruviform#631` `#674`）: `ruviform-plugin-dock-test` に `plugin-dock-lab` bin +1,194（GPUI Test Lab / Scenario Console GUI v1）と `Cargo.toml`/`mise.toml` lane 簿記。`b6b72dbd`（`scarecrow173/ruviform#532` `#677`）: `ospix.rs` +521・`real_dock.rs` +859/−96 — real-dcomp-pixel regression case + `GPUI_DISABLE_DIRECT_COMPOSITION` 実機証拠化 | AL 依存木外（lockfile 26 package に非含有、AL は Plugin Host を提供しない）。影響なし |
| audio-device（依存木外） | `6a481425`（`scarecrow173/ruviform#629` `#673` 4th slice）: `ruviform-audio-device` の `examples/endpoint_probe.rs` +259/−24・`tests/virtual_endpoint.rs` +1,371 新規 — WASAPI streaming verifier + endpoint_probe 拡張 | AL 依存木外（lockfile 26 package に非含有、AL は実 OS Audio Device を提供しない）。影響なし |
| CI/mise/policy（ruviform 側） | `mise.toml` +60/−9: `7956bfc4` の Test Lab lane 簿記と `de3ed4e8`/`b6b72dbd` の lane 追加。`.github/` 変更なし。docs/ は RF#629/#630/#631/#532 の文書チェーン、evidence/ は rf629/rf630/rf631 の verification 記録と DComp pixel PNG | AL 側 CI 定義・manifest への直接影響なし |

結論: 破壊的 API/Feature/OS 変更は検出されず、AL の全検証 lane が
対象 revision で pass。依存木内変更は additive 公開 API
（record codec）・内部 callsite 更新・内部リファクタのみ。
新 API/feature（record codec・plugin test_instrument fixture・
audio-device endpoint probe・Test Lab GUI・DComp pixel regression）
は opt-in/test-only/依存木外であり、AL 側の採用 gate・需要が
未定義のため本 slice では採用しない。採用判断は「採用」（pin
追従自体。詳細は task doc と PR 本文）。

### upstream main CI 証拠（cbcc8343 対象、実測）

`gh run list --commit cbcc83437616fdd7418da37183c6be66b6cfe225`
（ruviform repo、2026-11-16 確認）:

| workflow | run | 結果 | 内訳 |
| --- | --- | --- | --- |
| `consumer-verify`（workflow_dispatch） | 43783883894 | **success** | preflight / verify-synvelis / **verify-audiolens** / summary 全て success |
| `daily-ci`（schedule） | 43783844680 | **success** | decide / decide-consumer-verify（dispatcher のみ） |
| `ci`（workflow_dispatch） | 43783872777 | **failure** | `verify (windows-latest)`・`l1-real-verify (windows-latest)`・`msrv-verify`・`ui-*`・`observability-*`・`io-structured-verify`・`compression-verify`・`icc-verify`・`bench` は success。**failure**: `verify (ubuntu-latest)`・`l1-real-verify (ubuntu-latest)`（`ruviform-plugin-test-vst3` の clippy `-D warnings`: unnecessary `u32 as u32` cast ×5）、`verify-macos`・`rt-verify`（`ruviform-midi-device` test `output_ports` の compile error: `MidiGroup`/`MidiChannel`/`NoteKey`/`MidiValue`/`MidiKind` 未解決）、`ci-summary`（集約） |

失敗の切り分け: `ruviform-plugin-test-vst3` の clippy 失敗は
`de3ed4e8` で追加された instrument/events 系コードに起因する
可能性が高い（同 crate は本差分で大幅変更、unix/macos lane のみ
で顕在化 — windows lane は success）。`ruviform-midi-device` は
本差分（`eafe0a08..cbcc8343`）で未変更であり、test
`output_ports` の compile 失敗は `bff3603b`（PR #663、pin
`eafe0a08` に既に含まれる）以降の既存問題の可能性がある
（`ac10a2f7` の covering ci は `bff3603b` より前で green、
`eafe0a08` には covering ci なし）。いずれの失敗 crate も AL の
lockfile 26 package に非含有（依存木外）であり、AL 側の採用判断・
検証結果に影響しない。`scarecrow173/ruviform#625`（CI failure
tracker、Open）が main の `ci` 失敗を追跡しており、状況を
コメントで追記する。

## Non-Functional Constraints

- lockfile-primary 契約の維持（manifest `rev` 固定への回帰禁止）。
- fail-closed 検査（check-ruviform-lock/rev）の有効性維持。
- 検証は正規 mise task 経由。cargo 直実行は補助証拠として区別。
- 未実行 gate を Pass にしない（NotRun/Unsupported 語彙の厳守）。
- upstream CI の failure を隠さず、依存木内/外の切り分けを記録する。

## Risks and Trade-offs

- upstream `ci` workflow が `cbcc8343` で failure（依存木外
  crate の lint/compile 失敗）。AL 依存木・consumer-verify は
  green だが、upstream main の健全性は `scarecrow173/ruviform#625`
  で追跡が必要。将来これらの crate が依存木に入った場合に
  顕在化し得る。
- `ruviform-plugin-core` の record codec は公開 API 追加だが
  AL は消費経路を持たない。`plugin-host` の callsite 更新は
  upstream 内部の整合修正であり AL への波及はない。
- `ruviform-audio-test` の内部リファクタは `pub(crate)` に留まり
  公開契約不変。`rf628_oracle` の standing 回帰で非回帰を確認済み。
- rollback は `Cargo.lock` revert + `sync-ui-deps` 再実行で可能
  （生成物のみの変更、製品コード差分なし）。

## Deferred Design Concerns

- `encode_event_record`/`decode_event_record` の AL 採用設計 |
  plugin wire codec の shm slot 経路を AL が消費する要件は未定義
  | 採用 slice の spec 起票時 | semantic/headless pass を実機
  acceptance と誤認 | 本 slice では影響評価のみ
- plugin test_instrument fixture・audio-device endpoint probe・
  Test Lab GUI・DComp pixel regression の AL 採用 | AL#127 の
  EXT-L3/GUI-L2 要件と接続するが採用 slice・独立 Oracle 接続が
  未定義 | 採用 Issue 起票時 | upstream 機能の重複実装リスク |
  影響評価に記録済み

## Verification Notes

- [x] `mise run check-ruviform-lock` / `check-ruviform-rev`:
  単一 SHA・vendor 一致（fail-closed）
- [x] `mise run sync-ui-deps` → `check-ui` → `test-ui`:
  対象 SHA で Pass（113 tests）
- [x] `mise run fmt`/`check al-analysis`/`lint al-analysis`/
  `test al-analysis`: 対象 crate Pass（`rf628_oracle` 7 tests
  含む、長尺 scenario 実走）
- [x] `mise run check-cli-headless`: headless 境界 Pass
- [x] `mise run verify-toolchain`/`cargo fetch --locked`: Pass
- [x] upstream main CI 証拠: `consumer-verify`/`daily-ci` success、
  `ci` failure（依存木外 crate のみ、切り分け記録済み）
- [x] 補助証拠: `mise run check-full`（workspace check）
- [ ] `*-full` の lint/test/build、`check-ui-gpui`/`check-ui-webview`
  各 lane、`check-ci-workflow`/`check-ci-tracker-workflows`:
  本 slice の正規検証コマンド対象外のため未実施

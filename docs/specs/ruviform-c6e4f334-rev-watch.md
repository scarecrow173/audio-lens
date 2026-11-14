---
id: "SPEC-034XCOYQWCk6bOeIZofauh"
type: "spec"
status: "approved"
title: "ruviform pin c6e4f334 追従評価（AL#127）"
created: "2026-11-15"
updated: "2026-11-15"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/audio-lens/issues/127"
    - "https://github.com/scarecrow173/ruviform/issues/219"
    - "https://github.com/scarecrow173/ruviform/issues/294"
    - "https://github.com/scarecrow173/ruviform/issues/628"
    - "https://github.com/scarecrow173/ruviform/issues/389"
    - "https://github.com/scarecrow173/ruviform/issues/306"
    - "https://github.com/scarecrow173/ruviform/issues/313"
    - "https://github.com/scarecrow173/ruviform/issues/243"
    - "https://github.com/scarecrow173/ruviform/issues/465"
    - "https://github.com/scarecrow173/ruviform/issues/635"
  changes:
    added:
      - type: "file"
        path: "Cargo.lock"
      - type: "file"
        path: "ui/vendor/ruviform/REV"
      - type: "file"
        path: "docs/specs/ruviform-c6e4f334-rev-watch.md"
      - type: "file"
        path: "docs/designs/ruviform-c6e4f334-rev-watch.md"
      - type: "file"
        path: "docs/tasks/ruviform-c6e4f334-rev-watch.md"
      - type: "file"
        path: "docs/test-specs/ruviform-c6e4f334-rev-watch.md"
      - type: "file"
        path: "docs/impl/ir/ruviform-c6e4f334-rev-watch.md"
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated:
      - type: "file"
        path: "ui/vendor/ruviform"
  implements: []
  implemented-by: []
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/specs/ruviform-fd1d4d5f-rev-watch.md"
    - "docs/specs/ruviform-b02942c2-rev-watch.md"
  refines: []
  refined-by: []
  derives-from: []
  derived-by: []
  verifies: []
  verified-by:
    - "../test-specs/ruviform-c6e4f334-rev-watch.md"
  references: []
  defers: []
  deferred-by: []
---

# ruviform pin c6e4f334 追従評価（AL#127）

## What Should Be Built

AudioLens の ruviform 解決 pin を
`fd1d4d5f4de57092b96ad1cd309b85fe47cbef38` から候補
`c6e4f334242567dbd0e0d8b7713d9d480a6aab66`（ruviform main HEAD）へ、
lockfile-primary 手順（`tools/update-ruviform-rev.mjs` →
`mise run sync-ui-deps`）で追従する。`Cargo.lock` の 20 ruviform
package が単一 SHA に解決され、`ui/vendor/ruviform/REV` が同一 SHA を
指すことを確認する。

`fd1d4d5f..c6e4f334` の 114 commit・892 file・+303,172/−2,270 行の
差分（大半は ruviform 側 docs/evidence/poc/ffi-poc で、`crates/` 配下は
75 file・+9,161/−475 行）を調査し、AudioLens への影響を軸別
（f64 精度 / CLI / GUI / scenario / feature / IO 契約）に記録する。
新規 crate `ruviform-audio-test`（Issue #628 first slice、決定論的
test runtime）・`ruviform-plugin-clap` の CLAP L0 editor-attach
（Issue #389）・`ruviform-io-file` の opt-in `fault-injection` feature
（Issue #306）・`ruviform-ui-gpui` の `uitest` feature（Issue #313
slice S2 windowed TestIo）・`ruviform-ui-test` の内部再構成
（Issue #243/#625/#388）・consumer-verify schema v2（Issue #294）等の
採用判断は本 slice では「採用しない」と明示する。

## Why Now

Issue #127 は Ruviform 更新を AudioLens が Synvelis と対等な独立
Consumer として継続評価・導入するための常設追跡 Issue である。
候補 `c6e4f334` は `fd1d4d5f..c6e4f334` で 114 commit・892 file の
大規模差分を含み、AL 依存木内では `ruviform-ui-gpui`（`uitest`
feature・`windowed_io`・`UiJob` cfg 拡張）、`ruviform-ui-test`
（driver/headless/io/journal/fixture の内部変更と Window/Job
invariant test）、`ruviform-io-file`（opt-in `fault-injection`
feature で `mock` module を公開化）、`ruviform-ui-webview`
（legacy/preview 分類の doc 追記のみ）、`ruviform-observability`
（test のみの timing 修正）が変更されている。依存木外では新規
`ruviform-audio-test`、`ruviform-audio-device`、`ruviform-audio`、
`ruviform-io-e2e`、`ruviform-midi-device`、`ruviform-plugin-*`、
`ruviform-poc-native-dock`、`ruviform-video-proc` が変更されている。
AL が採用済みの GPUI driver 経路・UI test 契約・IO staged publish
契約に隣接する変更を含むため、candidate tuple として評価・記録する
必要がある。

## Users and Value

- AudioLens 開発者: pin 追従の検証証拠と影響範囲の記録により、
  次候補の評価コストを下げ、rollback 判断材料を残せる。
- Coordinator / Reviewer: Issue #127 の受入条件に対応する
  検証結果（実測と NotRun の区別）を PR/Issue から追跡できる。
- 将来の test runtime / windowed TestIo / fault-injection / CLAP
  editor 採用判断: upstream の新 crate・feature・capability の存在を
  記録し、採用/保留の判断材料にする。

## Scope

### In Scope

- ruviform pin `fd1d4d5f` → `c6e4f334` の lockfile-primary 追従
  （`Cargo.lock` 20 package の単一 SHA 解決、`ui/vendor/ruviform`
  の再生成と `REV` 一致）。
- `fd1d4d5f..c6e4f334` の差分調査と、AudioLens に影響する
  API/Feature/OS の変更・関連 Issue の記録。
- 正規 mise 検証 lane（fmt/check/lint/test/build、CLI headless、
  UI、GPUI、WebView、CI profile）の対象 revision での実行と結果記録。
- 採用/保留判断、rollback 手順、Issue #127 への結果記録、PR 作成。

### Out of Scope

- 製品コード（`crates/`・`al-gui/`・`ui/src/`）の機能変更。pin
  追従でコンパイル/テストが壊れた場合の adapter 修正は行わず、
  API 差分を PR 本文に記録する（今回は不要だった）。
- ruviform の新 API・新 feature（`ruviform-audio-test`、
  `ruviform-ui-gpui` の `uitest` feature・windowed TestIo、
  `ruviform-io-file` の `fault-injection` feature、
  `ruviform-plugin-clap` の editor-attach、`ruviform-ui-test` の
  Window/Job invariant 拡張）の採用。
- 実 OS GUI・実 Pixel・macOS/Linux・cross-consumer gate の実施
  （環境到達不能なものは NotRun/Unsupported として記録）。
- ruviform・synvelis リポジトリの変更。
- Issue #127 の Close。

## Requirements

- manifest の恒久固定 rev へ戻さず、lockfile の `source` fragment が
  単一 resolved SHA の正本であり続けること。
- `cargo fetch --locked`・`mise run check-ruviform-lock`・
  `mise run check-ruviform-rev` が欠落・分裂・不一致を fail-closed
  に検出すること。
- `mise run sync-ui-deps` → `check-ui` → `test-ui` が対象 SHA で
  Pass し、`Cargo.lock` と `ui/vendor/ruviform/REV` が一致すること。
- 検証不能事項・失敗ケース・許容差を結果へ明記し、未実行 gate を
  Pass に繰り上げないこと。
- 採用/保留判断と rollback 情報（旧 SHA `fd1d4d5f` への復帰手順）
  を記録すること。

## Acceptance Criteria

- [x] ruviform pin が `c6e4f334242567dbd0e0d8b7713d9d480a6aab66` に
  解決され、`mise run check-ruviform-lock` / `check-ruviform-rev`
  が pass する。
- [x] `Cargo.lock` と `ui/vendor/ruviform/REV` が同一 SHA を指し、
  `Cargo.lock` の差分が ruviform 系 SHA fragment 更新のみ（20 行、
  推移的依存の変動なし）に留まる。
- [x] `fd1d4d5f..c6e4f334` の影響 API/Feature/OS と関連 Issue を
  docs に記録し、新 API/feature の採用判断（今回は採用しない）を
  明示する。
- [x] 正規 mise 検証 lane を対象 revision で実行し、結果を
  test-spec の対応表どおり記録する。
- [x] 採用/保留判断と rollback 情報を記録する。
- [x] PR 本文に resolved SHA・candidate SHA・影響表・採用 PR リンク
  を記録し、Issue #127 に candidate tuple 評価 comment を投稿する
  （Issue は Close しない）。

## Deferred Design Concerns

- `ruviform-audio-test`（決定論的 Audio/MIDI test runtime:
  VirtualClock・PCM fixture・MIDI trace・独立 oracle・run metadata）
  の AL 採用 | AL#127 の Windows L2/L3 要件が RF#628 fixture と
  RF#631 driver-neutral runner の利用を想定するが、AL 側の採用
  slice・Oracle 接続設計が未定義 | 採用判断 slice の spec 起票時 |
  upstream fixture の存在を失念して重複実装するリスク |
  design doc の影響評価と PR 本文に記録
- `ruviform-ui-gpui` の `uitest` feature（windowed TestIo /
  `rvf.uitest.v1` over real GPUI windows）・`real-scenario` OS
  input ops の AL 採用 | AL の GUI-L2 は実 Window/入力/Focus/画素を
  要求するが、AL 側の採用 gate・独立 Oracle 接続が未定義 | 採用
  判断 slice の spec 起票時 | semantic/headless の Pass を実機
  acceptance と誤認するリスク | design doc の影響評価に記録
- `ruviform-io-file` の `fault-injection` feature・
  `ruviform-plugin-clap` editor-attach・`ruviform-ui-test` の
  Window/Job 拡張の AL 採用 | AL の Job/Resume/Cancel・Plugin
  非依存性・既存 scenario 契約との接続が未評価 | 各機能の採用
  Issue 起票時 | upstream 機能の存在を失念して重複実装する
  リスク | design doc の影響評価に記録

## Sources and Relations

- Issue #127（常設追跡 Issue、本 slice の受入条件の出典）
- ruviform#219（update/Consumer 共通契約）、ruviform#294
  （candidate tuple Consumer 検証、schema v2 へ拡張 #639）、
  ruviform#628（deterministic test runtime first slice #638）、
  ruviform#389（CLAP L0 editor-attach #637）、ruviform#306
  （io fault seam #610）、ruviform#313（windowed TestIo S2 #572）、
  ruviform#243（Window/Job headless invariant #613）、
  ruviform#625（ui-test handshake 分類 #626）、
  ruviform#465（WebView legacy 分類 #616）、
  ruviform#635/#636（virtual-first 検証契約・GPUI DComp 監査）
- 前段採用: PR #194（pin `fd1d4d5f` 追従評価）、PR #193（pin
  `b02942c2` 追従評価）、PR #190（pin `271e2d41` 追従評価）

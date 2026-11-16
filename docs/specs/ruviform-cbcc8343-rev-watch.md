---
id: "SPEC-034YnFfBgevMiuCJ5T6H98"
type: "spec"
status: "approved"
title: "ruviform pin cbcc8343 追従評価（AL#127）"
created: "2026-11-16"
updated: "2026-11-16"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/audio-lens/issues/127"
    - "https://github.com/scarecrow173/ruviform/issues/219"
    - "https://github.com/scarecrow173/ruviform/issues/294"
    - "https://github.com/scarecrow173/ruviform/issues/532"
    - "https://github.com/scarecrow173/ruviform/issues/625"
    - "https://github.com/scarecrow173/ruviform/issues/628"
    - "https://github.com/scarecrow173/ruviform/issues/629"
    - "https://github.com/scarecrow173/ruviform/issues/630"
    - "https://github.com/scarecrow173/ruviform/issues/631"
  changes:
    added:
      - type: "file"
        path: "Cargo.lock"
      - type: "file"
        path: "ui/vendor/ruviform/REV"
      - type: "file"
        path: "docs/specs/ruviform-cbcc8343-rev-watch.md"
      - type: "file"
        path: "docs/designs/ruviform-cbcc8343-rev-watch.md"
      - type: "file"
        path: "docs/tasks/ruviform-cbcc8343-rev-watch.md"
      - type: "file"
        path: "docs/test-specs/ruviform-cbcc8343-rev-watch.md"
      - type: "file"
        path: "docs/impl/ir/ruviform-cbcc8343-rev-watch.md"
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
    - "docs/specs/ruviform-eafe0a08-rev-watch.md"
    - "docs/specs/ruviform-ac10a2f7-rev-watch.md"
  refines: []
  refined-by: []
  derives-from: []
  derived-by: []
  verifies: []
  verified-by:
    - "../test-specs/ruviform-cbcc8343-rev-watch.md"
  references: []
  defers: []
  deferred-by: []
---

# ruviform pin cbcc8343 追従評価（AL#127）

## What Should Be Built

AudioLens の ruviform 解決 pin を
`eafe0a08b71bd9a74566ae6678c3e0683fce1421` から候補
`cbcc83437616fdd7418da37183c6be66b6cfe225`（ruviform main HEAD）へ、
lockfile-primary 手順（`tools/update-ruviform-rev.mjs` →
`mise run sync-ui-deps`）で追従する。`Cargo.lock` の 26 ruviform
package が単一 SHA に解決され、`ui/vendor/ruviform/REV` が同一 SHA を
指すことを確認する。

`eafe0a08..cbcc8343` の 6 commit・97 file・+15,611/−581 行の
差分を調査し、AudioLens への影響を軸別に記録する。AL 依存木内
（lockfile 26 package に含まれる crate）で変更されたのは
`de3ed4e8`（`scarecrow173/ruviform#630` test_instrument slice）
由来の 3 crate のみ: `ruviform-plugin-core` の `wire.rs` に
shm slot 用固定長 record codec `encode_event_record`/
`decode_event_record` が additive 追加（既存 `encode_event`・
`EVENT_WIRE_SIZE` 不変）、`ruviform-plugin-host` の
`l1/data.rs` が内部 callsite を `encode_event` から
`encode_event_record` へ更新（1 行）、`ruviform-audio-test` の
`backend/stream.rs` が非公開 fn `deliver_input_errors` を抽出する
内部リファクタ。in-tree crate の `Cargo.toml` 変更はなく、
AL `Cargo.lock` の差分は 26 package の SHA fragment のみ。
依存木外では `ruviform-audio-device`（`scarecrow173/ruviform#629`
WASAPI streaming verifier + endpoint_probe 拡張 +
virtual_endpoint test）、`ruviform-plugin-clap`/`plugin-export`/
`plugin-dock-test`（`scarecrow173/ruviform#630` test_instrument・
`scarecrow173/ruviform#631` Test Lab GUI・`scarecrow173/ruviform#532`
DComp pixel regression）、`ruviform-plugin-test-clap`/
`test-vst3`（instrument fixture）が変更されている。これらの
採用判断は本 slice では「採用しない」と明示する。

## Why Now

Issue #127 は Ruviform 更新を AudioLens が Synvelis と対等な独立
Consumer として継続評価・導入するための常設追跡 Issue である。
前 slice（PR #216）は pin `eafe0a08` 追従を着地させた。本 slice
はその直後の upstream main 追従であり、`eafe0a08..cbcc8343` に
`ruviform-plugin-core` の record codec・`ruviform-plugin-host` の
内部 callsite 更新・`ruviform-audio-test` の内部リファクタという
依存木内変更が含まれるため、採用/保留の判断記録と standing 回帰
（`rf628_oracle` を含む `al-analysis` の check/lint/test）の対象
revision での再実走が必要である。upstream main CI では
`cbcc8343` に対する covering run が初めて取得されており
（`consumer-verify` success・`daily-ci` success・`ci` failure）、
その証拠評価も本 slice の記録対象である。

## Users and Value

- AudioLens 開発者: pin 追従の検証証拠と影響範囲の記録により、
  次候補の評価コストを下げ、rollback 判断材料を残せる。
- Coordinator / Reviewer: Issue #127 の受入条件に対応する
  検証結果（実測と NotRun の区別）を PR/Issue から追跡できる。
- 将来の record codec・plugin instrument/fixture・audio-device
  endpoint probe・Test Lab GUI 採用判断: upstream の新 API・
  fixture の存在と upstream CI 状態を記録し、採用/保留の判断
  材料にする。

## Scope

### In Scope

- ruviform pin `eafe0a08` → `cbcc8343` の lockfile-primary 追従
  （`Cargo.lock` 26 package の単一 SHA 解決、
  `ui/vendor/ruviform` の再生成と `REV` 一致）。
- `eafe0a08..cbcc8343` の差分調査と、AudioLens に影響する
  API/Feature/OS の変更・関連 Issue の記録。
- `ruviform-plugin-core` record codec・`ruviform-plugin-host`
  内部 callsite・`ruviform-audio-test` 内部リファクタの採用/見送り
  判断の記録。
- 正規 mise 検証 lane（fmt/check/lint/test、CLI headless、
  UI vendor lane、toolchain）の対象 revision での実行と結果記録。
- upstream main CI 証拠（`cbcc8343` 対象の `ci`/`daily-ci`/
  `consumer-verify`）の確認と記録。
- 採用/保留判断、rollback 手順、Issue #127 への結果記録、PR 作成。

### Out of Scope

- 製品コード（`crates/`）の機能変更。pin 追従でコンパイル/
  テストが壊れた場合の adapter 修正は行わない（今回は不要
  だった）。
- ruviform の新 API・新 feature（record codec・plugin
  instrument/fixture・audio-device endpoint probe・Test Lab
  GUI）の採用。
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
- 依存木内変更（`ruviform-plugin-core`/`plugin-host`/
  `audio-test`）が additive・内部実装に留まり、CLI/headless
  依存木・製品 API を破壊しないことを `check-cli-headless` と
  `check`/`lint`/`test al-analysis` で確認すること。
- 検証不能事項・失敗ケース・許容差を結果へ明記し、未実行 gate を
  Pass に繰り上げないこと。
- 採用/保留判断と rollback 情報（旧 SHA `eafe0a08` への復帰手順）
  を記録すること。

## Acceptance Criteria

- [x] ruviform pin が `cbcc83437616fdd7418da37183c6be66b6cfe225` に
  解決され、`mise run check-ruviform-lock` / `check-ruviform-rev`
  が pass する。
- [x] `Cargo.lock` と `ui/vendor/ruviform/REV` が同一 SHA を指し、
  `Cargo.lock` の差分が ruviform 系 SHA fragment 更新のみである
  （新規 package 追加・version 変動・依存 edge 変動なし）。
- [x] `eafe0a08..cbcc8343` の影響 API/Feature/OS と関連 Issue を
  docs に記録し、新 API/feature の採用判断（今回は採用しない）を
  明示する。
- [x] 正規 mise 検証 lane を対象 revision で実行し、結果を
  test-spec の対応表どおり記録する。
- [x] upstream main CI 証拠（`cbcc8343` 対象の `ci`/`daily-ci`/
  `consumer-verify` の成否と失敗 job の切り分け）を記録する。
- [x] 採用/保留判断と rollback 情報を記録する。
- [x] PR 本文に resolved SHA・candidate SHA・影響表・採用判断を
  記録し、Issue #127 に candidate tuple 評価 comment を投稿する
  （Issue は Close しない）。

## Deferred Design Concerns

- `ruviform-plugin-core` の `encode_event_record`/
  `decode_event_record`（shm slot 固定長 record codec）の AL 採用
  | RF#630 test_instrument で追加された wire codec。AL は
  `plugin-host` の shm slot 経路を直接消費しないため現状不要
  | plugin host / wire codec 消費 slice の spec 起票時 |
  upstream API の存在を失念して重複実装するリスク | design doc
  の影響評価と PR 本文に記録
- `ruviform-plugin-test-{vst3,clap}` の test_instrument
  （MIDI/transport fixture + L1 real-ABI oracle）・
  `ruviform-plugin-dock-test` の Test Lab GUI / DComp pixel
  regression・`ruviform-audio-device` の WASAPI endpoint probe の
  AL 採用 | AL は Plugin Host・実 OS Audio Device を提供せず
  依存木外。EXT-L3（実 Plugin ABI/実 OS API）要件との接続は
  将来の採用 slice で評価 | 採用 Issue 起票時 | upstream
  fixture の存在を失念するリスク | design doc の影響評価に記録
- upstream `ci` workflow の `cbcc8343` failure（依存木外
  crate の lint/compile 失敗）の追跡 | `scarecrow173/ruviform#625`
  tracker 管理。AL 側の恒久 workaround は不要・禁止 | upstream
  修正または tracker 更新時 | 依存木外のため AL 受入には影響
  しないが、将来の依存木取り込み時に顕在化し得る | evidence と
  PR 本文に記録、ruviform#625 へ状況コメントを追加

## Sources and Relations

- Issue #127（常設追跡 Issue、本 slice の受入条件の出典）
- ruviform#219（update/Consumer 共通契約）、ruviform#294
  （candidate tuple Consumer 検証）、ruviform#625（CI tracker）
- ruviform#629（WASAPI endpoint probe 4th slice `#673`）、
  ruviform#630 test_instrument（`#676`）、ruviform#631 Test Lab
  GUI（`#674`）、ruviform#532 DComp pixel regression（`#677`）
- 前段採用: PR #216（pin `eafe0a08` 追従評価）、PR #215（pin
  `abb9f372` 追従 + RF#628 consumer oracle）、PR #214（pin
  `ac10a2f7` 追従評価）、PR #213（pin `c6e4f334` 追従評価）

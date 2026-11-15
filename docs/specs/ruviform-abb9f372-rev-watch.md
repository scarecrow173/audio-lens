---
id: "SPEC-034Y1A2b3C4d5E6f7G8h9"
type: "spec"
status: "approved"
title: "ruviform pin abb9f372 追従評価 + RF#628 consumer oracle（AL#127）"
created: "2026-11-15"
updated: "2026-11-15"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/audio-lens/issues/127"
    - "https://github.com/scarecrow173/ruviform/issues/628"
    - "https://github.com/scarecrow173/ruviform/issues/629"
    - "https://github.com/scarecrow173/ruviform/issues/630"
    - "https://github.com/scarecrow173/ruviform/issues/631"
    - "https://github.com/scarecrow173/ruviform/issues/253"
    - "https://github.com/scarecrow173/ruviform/issues/306"
    - "https://github.com/scarecrow173/ruviform/issues/389"
    - "https://github.com/scarecrow173/ruviform/issues/646"
    - "https://github.com/scarecrow173/ruviform/issues/650"
  changes:
    added:
      - type: "file"
        path: "Cargo.lock"
      - type: "file"
        path: "ui/vendor/ruviform/REV"
      - type: "file"
        path: "docs/specs/ruviform-abb9f372-rev-watch.md"
      - type: "file"
        path: "docs/designs/ruviform-abb9f372-rev-watch.md"
      - type: "file"
        path: "docs/tasks/ruviform-abb9f372-rev-watch.md"
      - type: "file"
        path: "docs/test-specs/ruviform-abb9f372-rev-watch.md"
      - type: "file"
        path: "docs/impl/ir/ruviform-abb9f372-rev-watch.md"
    modified:
      - type: "file"
        path: "Cargo.toml"
      - type: "file"
        path: "crates/al-analysis/Cargo.toml"
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
    - "docs/specs/ruviform-ac10a2f7-rev-watch.md"
    - "docs/specs/issue-127-rf628-consumer-oracle.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/specs/issue-127-rf628-consumer-oracle.md"
  derived-by: []
  verifies: []
  verified-by:
    - "../test-specs/ruviform-abb9f372-rev-watch.md"
  references: []
  defers: []
  deferred-by: []
---

# ruviform pin abb9f372 追従評価 + RF#628 consumer oracle（AL#127）

## What Should Be Built

AudioLens の ruviform 解決 pin を
`ac10a2f7761c08db73aebd861c0a5c1f53f269b7` から候補
`abb9f372d3ee95ee824b3c4f7dc68e8083ce2774`（ruviform main HEAD、
`crates/ruviform-audio-test/src/pipeline/` 存在確認済み）へ、
lockfile-primary 手順で追従する。同時に test-only crate
`ruviform-audio-test`（RF#628 deterministic PCM fixture/oracle）
を `al-analysis` の dev-dependency として追加し、file-based 解析の
f64 fidelity・長尺非回帰・gap/Validity を AL 独自の独立 Oracle で
判定する統合テスト `crates/al-analysis/tests/rf628_oracle.rs` を
実装する（`docs/specs/issue-127-rf628-consumer-oracle.md` が正本）。

`ac10a2f7..abb9f372` の 11 commit・182 file・+31,238/−528 行の
差分のうち、AL 依存木内で変更されたのは `ruviform-audio-core`
（additive: `midi_output` module 新設、Issue #629）、
`ruviform-ui-test`（additive: plugin-dock scenario catalog、
Issue #631）、`ruviform-ui-gpui`（additive: real_window native-slot
dock lane、Issue #631）、および新規 dev-dep として入る
`ruviform-audio-test`（`pcm`/`oracle` module は ac10a2f7 から
無変更、`pipeline`/`backend`/`midi` module が Issue #628
slice 2-3 で追加）である。依存木外の変更（`ruviform-plugin-dock-test`、
`ruviform-plugin-test-clap`/`vst3`、`ruviform-midi-device`、
`ruviform-io-e2e` lanes 等）の採用判断は本 slice では
「採用しない」と明示する。

## Why Now

Issue #127 は Ruviform 更新を AudioLens が Synvelis と対等な独立
Consumer として継続評価・導入するための常設追跡 Issue であり、
受入条件に「RF #628 の Ruviform 固定 PCM/Clock/Signal Fixture を
利用し、AudioLens 独自の f64/長尺/PTS/gap/Validity/CLI/解析GUIを
独立 Oracle で判定する」ことが明記されている。本 slice はそのうち
file-based 解析の f64 fidelity・長尺非回帰・gap/Validity の 3 脚を
担当する。`abb9f372` は RF#628 の deterministic pipeline wiring
（scripted backend を production consumer へ接続する 3rd slice、
PR #664）を含む main HEAD であり、`pipeline/` module の存在が
Coordinator 指定の採用条件である。

## Users and Value

- AudioLens 開発者: RF#628 fixture を使った独立 Oracle テストにより、
  file-based 解析の f64 精度・長尺・gap/Validity を upstream の
  self-test に依存せず検証できる。
- Coordinator / Reviewer: pin 追従の検証証拠・採用判断・rollback
  材料を PR/Issue から追跡できる。
- 将来の device-pipeline/clock consumer 採用 slice: `pipeline/`・
  `backend::*`・`clock::*` API の存在を記録し、採用判断材料にする。

## Scope

### In Scope

- ruviform pin `ac10a2f7` → `abb9f372` の lockfile-primary 追従
  （`Cargo.lock` 全 ruviform package の単一 SHA 解決、
  `ui/vendor/ruviform` の再生成と `REV` 一致）。
- `ruviform-audio-test` の `al-analysis` dev-dependency 追加と
  `ruviform-audio-core`（`SampleRate` 構築用）の dev-dep 追加。
- `crates/al-analysis/tests/rf628_oracle.rs` の新設
  （AL-1 f64 fidelity / AL-2 長尺非回帰 / AL-3 gap/Validity）。
- `ac10a2f7..abb9f372` の差分調査と影響記録、採用/保留判断、
  rollback 手順、正規 mise 検証 lane の実行と結果記録。

### Out of Scope

- 製品コード（`crates/` src）の機能変更。ingest/decode 側の
  PTS/gap 正規化、`al-analysis` src の挙動変更。
- AL-4 deterministic clock drift（live clock consumer 不在のため
  Deferred）、AL-5 device-pipeline legs（`backend::*`/`pipeline::*`/
  `midi::*`/`ScriptedBackend` — device boundary 非消費のため
  NotApplicable）。
- ruviform の新 API・新 feature（`pipeline::*`、`backend::*`、
  `midi::*`、`ruviform-plugin-dock-test`、`ruviform-plugin-test-*`、
  `ruviform-midi-device`、`ruviform-ui-test` scenario catalog、
  `ruviform-ui-gpui` native-slot lane、`ruviform-audio-core`
  `midi_output`）の製品採用。
- CLI/GUI/Export・3OS CI・cross-consumer・実機検証。
- ruviform・synvelis リポジトリの変更、Issue #127 の Close。

## Requirements

- manifest の恒久固定 rev へ戻さず、lockfile の `source` fragment が
  単一 resolved SHA の正本であり続けること。manifest に `rev`/
  branch/tag を書かないこと。
- `cargo fetch --locked`・`mise run check-ruviform-lock`・
  `mise run check-ruviform-rev` が欠落・分裂・不一致を fail-closed
  に検出すること。
- `ruviform-audio-test` は `publish = false` の test-only crate で
  あり、dev-dependency に限定し製品配布へ含めないこと。
- 期待値は AL 側解析的参照と `oracle::pcm_reference_block` の
  独立再実装から導出し、ruviform 側 self-test の結果を AL の
  Pass 証拠に転用しないこと。
- 検証不能事項・失敗ケース・許容差を結果へ明記し、未実行 gate を
  Pass に繰り上げないこと。
- 採用/保留判断と rollback 情報（旧 SHA `ac10a2f7` への復帰手順）
  を記録すること。

## Acceptance Criteria

- [x] ruviform pin が `abb9f372d3ee95ee824b3c4f7dc68e8083ce2774` に
  解決され、`mise run check-ruviform-lock` / `check-ruviform-rev`
  が pass する（26 package・単一 SHA）。
- [x] `Cargo.lock` と `ui/vendor/ruviform/REV` が同一 SHA を指す。
- [x] `tests/rf628_oracle.rs` が AL-1/AL-2/AL-3 の 3 脚を実装し、
  `cargo test -p al-analysis --test rf628_oracle` が Pass する。
- [x] `ac10a2f7..abb9f372` の影響 API/Feature/OS と関連 Issue を
  docs に記録し、新 API/feature の採用判断（今回は採用しない）を
  明示する。
- [x] 正規 mise 検証 lane を対象 revision で実行し、結果を
  test-spec の対応表どおり記録する。
- [x] 採用/保留判断と rollback 情報を記録する。
- [x] PR 本文に resolved SHA・candidate SHA・影響表・採用 PR リンク
  を記録し、Issue #127 に結果を記録する（Issue は Close しない、
  `Refs #127`）。

## Deferred Design Concerns

- AL-4 deterministic clock drift | `clock::VirtualClock*` API は
  存在するが AL に live clock consumer がない | clock consumer
  採用 slice の spec 起票時 | clock 契約の見落とし | test-spec・
  design doc に Deferred 記録
- AL-5 device-pipeline legs（`backend::*`/`pipeline::*`/`midi::*`/
  `ScriptedBackend`） | AL は file-based で device boundary を
  消費しない | device-pipeline consumer 採用 slice 時 |
  device 境界回帰の見落とし | test-spec に NotApplicable 記録
- `ruviform-plugin-dock-test`/`ruviform-plugin-test-clap`/`vst3`/
  `ruviform-midi-device`/io-e2e lanes の AL 採用 | AL は Plugin
  Host・MIDI device・io-e2e lane を製品機能として提供しておらず
  需要未定 | 採用 Issue 起票時 | upstream 機能の存在を失念する
  リスク | design doc の影響評価に記録
- spectral 系の gap 縮退（`argmax` の `total_cmp` NaN 規則）|
  既存 AL 意味論・本 slice 対象外 | spectral gap 対応 Issue
  起票時 | gap 混在入力の誤値 | PR 発見事項・test-spec に記録

## Sources and Relations

- Issue #127（常設追跡 Issue、本 slice の受入条件の出典）
- `scarecrow173/ruviform#628`（deterministic test runtime/fixtures）、
  `scarecrow173/ruviform#629`（MIDI device/output）、
  `scarecrow173/ruviform#630`（plugin test fixtures）、
  `scarecrow173/ruviform#631`（plugin-dock scenario/driver）、
  `scarecrow173/ruviform#253`（MIDI event contract）、
  `scarecrow173/ruviform#306`（io-e2e lanes）、
  `scarecrow173/ruviform#389`（verification-status reconcile）、
  `scarecrow173/ruviform#646`（numeric bench profile）、
  `scarecrow173/ruviform#650`（bench-smoke lane）
- 前段採用: PR #213（pin `ac10a2f7`）、PR #194（pin `fd1d4d5f`）、
  PR #193（pin `b02942c2`）、PR #190（pin `271e2d41`）
- 本 slice 文書チェーン: `docs/specs|designs|plans|tasks|
  test-specs|impl/ir/issue-127-rf628-consumer-oracle.md`

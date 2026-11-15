---
id: "DESIGN-034YaQQQPq20bNhuQdu0AH"
type: "design"
status: "approved"
title: "ruviform eafe0a08 pin 追従の設計"
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
        path: "docs/designs/ruviform-eafe0a08-rev-watch.md"
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
    - "../specs/ruviform-eafe0a08-rev-watch.md"
  implemented-by: []
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/designs/ruviform-ac10a2f7-rev-watch.md"
    - "docs/designs/ruviform-abb9f372-rev-watch.md"
  refines: []
  refined-by: []
  derives-from:
    - "../specs/ruviform-eafe0a08-rev-watch.md"
  derived-by: []
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# ruviform eafe0a08 pin 追従の設計

## Context

AudioLens は ruviform を Git 依存（manifest に `rev` を書かない
lockfile-primary 契約）で利用し、`Cargo.lock` の `source` fragment
が解決 SHA の正本。`tools/update-ruviform-rev.mjs` が
`cargo update -p <ruviform-package> --precise <sha>` を実行し、
`tools/sync-ruviform-ui.mjs`（`mise run sync-ui-deps`）が
`ui/vendor/ruviform` を lockfile SHA から再生成して `REV` を書く。
`tools/check-ruviform-rev.mjs` が lockfile/manifest/vendor の
一致を fail-closed に検査する。

候補 `eafe0a08b71bd9a74566ae6678c3e0683fce1421` は ruviform main
HEAD（`git ls-remote https://github.com/scarecrow173/ruviform.git
refs/heads/main` と local clone `G:/repositry/audio-products/
ruviform` の `origin/main` HEAD で一致確認。Spec 起稿時点と同一
SHA で upstream の進行なし）。前 pin `abb9f372`（PR #215 で採用、
RF#628 consumer oracle と同時着地）から 6 commit・93 file・
+13,204/−317 行の差分。内訳は crates/ 38 file・+7,442/−259、
docs/ 45 file・+5,302/−43、evidence/ 8 file・+454/−14、root
`Cargo.lock`/`mise.toml`。`.github/` と `ui/`（ruviform 側）の
変更はない。

## Scope

### In Scope

- `Cargo.lock` の ruviform 26 package を `eafe0a08` へ更新し、
  vendor tree を再生成する。
- 差分の AudioLens 影響評価（API/Feature/OS）と検証 lane 実行。
- `ruviform-audio-test` 新 API の採用/見送り判断と rollback 情報の
  記録。

### Out of Scope

- 製品コード変更、ruviform 新 API/feature の採用、他 repo 変更、
  実機/実 Pixel/macOS/Linux/cross-consumer gate の実施。

## Components and Boundaries

- `tools/update-ruviform-rev.mjs` / `tools/sync-ruviform-ui.mjs` /
  `tools/check-ruviform-rev.mjs`: pin 追従と整合検査の既存機構。
  今回は機構自体を変更しない。
- `crates/al-analysis`: `ruviform-audio-test`（dev-dependency、
  RF#628 consumer oracle `tests/rf628_oracle.rs` が
  `pcm`/`oracle` module を消費）と `ruviform-audio-core`
  （dev-dependency、`SampleRate` 構築用）、`ruviform-audio-analysis`/
  `ruviform-dsp`/`ruviform-numeric`（dependencies）を消費する
  唯一の変更対象 crate。本範囲で製品コードの変更はない。
- `crates/al-gui` 他: `ruviform-ui*`/`ruviform-media-*`/`io-*` 等を
  消費するが、本範囲の ruviform 差分にこれらの crate の変更はない
  （`ruviform-ui`/`ui-gpui`/`ui-test`/`ui-theme`/`ui-webview`/
  `viz`/`media-*`/`io-*`/`numeric`/`dsp`/`math`/`exec`/`rt`/
  `observability`/`audio*`/`graph`/`plugin-core`/`plugin-host`/
  `audio-plugin-bridge`/`video-core` は無変更）。
- `ui/`（TypeScript）: `ui/vendor/ruviform` の vendored source を
  `binding`/`guitest` 経由で利用。vendor は生成物で直接編集しない。
  本範囲の ruviform 差分に `ui/*` の変更はなく、vendor 内容は
  REV 以外不変。

## Data and Control Flow

1. `node tools/update-ruviform-rev.mjs
   eafe0a08b71bd9a74566ae6678c3e0683fce1421` で `Cargo.lock` を更新
   （全 26 package が同一 SHA に解決。差分は SHA fragment 26 行 +
   `ruviform-audio-test` の `serde`/`serde_json` 依存 edge 2 行のみで
   推移的依存の version 変動・新規 package 追加なし — 両 crate は
   既に lockfile 内に存在）。
2. `mise run sync-ui-deps` で `ui/vendor/ruviform` を再生成し
   `REV` に SHA を記録。
3. `check-ruviform-lock`/`check-ruviform-rev` で lockfile-primary
   契約（単一 SHA・vendor 一致）を検査。
4. 各検証 lane（下記 Verification）で対象 revision の
   compile/test/lint を実行。
5. 影響評価と採用/保留判断を記録し、PR を作成する。

## 差分影響評価（abb9f372..eafe0a08）

6 commit の主な領域と AudioLens への影響:

| 軸 | 主な変更 | AL 影響評価 |
| --- | --- | --- |
| f64 精度 / DSP / numeric / media / io | 差分なし（`ruviform-dsp`/`math`/`numeric`/`media-*`/`io-*`/`viz`/`exec`/`rt`/`video-core`/`audio`/`audio-analysis`/`audio-core`/`graph`/`plugin-core`/`plugin-host`/`audio-plugin-bridge`/`observability`/`ui*` に変更なし） | 影響なし。f64 kernel・FFT・locate/analysis 系の数値経路は不変。`check`/`test al-analysis` Pass で確認 |
| CLI / headless | 差分なし（CLI が消費する crate 群に製品コード変更なし） | `check-cli-headless` Pass。headless 境界の維持を確認 |
| GUI（`ruviform-ui*`） | 差分なし | 影響なし。`check-ui`/`test-ui` Pass（113 tests） |
| dev-dep `ruviform-audio-test`（依存木内、AL#127 RF#628 oracle の入力基盤） | `scarecrow173/ruviform#628` close-out `#669` + rustfmt `#672`: `metadata.rs` +92/−15 — `RunMetadata`/`RunParameters`/`CaseRequirement`/`CaseResult`/`VerificationLevel`/`EnvironmentKind` に `Serialize`/`Deserialize` derive（Case Schema v1 kebab-case vocabulary）と `RunParameters` の `Eq` derive、`RunMetadata::{to_json,from_json,save_json,load_json}` 4 非 RT API 追加。`clock.rs` +9/−4 — `JitterSpec`/`VirtualClockConfig` に serde derive。`backend.rs` +30/−1 — `#[non_exhaustive]` `StreamFault` に `Cancellation { at_callback }` variant 追加（production contract に cancel 概念がないため `Backend("cancelled")` error + abort delivery + terminate で表現）、`PumpReport.invalidated` doc 更新。`backend/stream.rs` +156/−27 — `collect_faults` を `FaultEffects` struct 化し `cancelled_pump_{output,input}` を追加。`Cargo.toml` +10/−1 — `serde`/`serde_json` 依存追加（理由記録コメント付き）。tests: `backend_faults.rs` +256、`metadata.rs` +136 新規 | AL は `pcm`/`oracle` module のみ消費（`rf628_oracle.rs`）。変更は全て additive: enum は `#[non_exhaustive]`、既存型への derive 追加は非破壊、新規 API は opt-in。`serde`/`serde_json` は dev-dep 経路に留まり CLI/headless 依存木へ波及しない（`check-cli-headless` Pass）。`check`/`lint`/`test al-analysis` Pass（`rf628_oracle` 7 tests 含む） |
| plugin adapter / test fixture（依存木外） | `ruviform-plugin-vst3`（`host.rs` +1,104/`controller.rs` +27 — Issue #630 adapter state bridge `#667`: VST3 `IBStream` state）・`ruviform-plugin-clap`（`host.rs` +78 — 同 CLAP `ext.state`）。`ruviform-plugin-test-vst3`（+約2,900: `editor.rs`/`editor_view.rs` 新規 — Issue #630 test_editor `#668` plugin-side IPlugView + real child-HWND fixture、`stateful.rs` +806 — test_stateful `#671` real state blob + L1 save/mutate/restore/save E2E）。`ruviform-plugin-test-clap`（`stateful.rs` +958 + host_path/l1 E2E tests — test_stateful `#671`）。`ruviform-plugin-dock-test`（`real_dock.rs` +796/−85 — Issue #631 real-editor-attach `#670`: test_editor real IPlugView + child HWND OS observation） | AL 依存木外（lockfile 26 package に非含有、AL は Plugin Host を提供しない）。影響なし |
| CI/mise/policy（ruviform 側） | `mise.toml` +3/−1: `test-l1-real-plugin` lane の description/run に `l1_stateful_test_plugin_e2e` test 追加。`.github/` 変更なし。docs/ は RF#628 close-out・RF#630 3 slice・RF#631 real-editor-attach の文書チェーン、evidence/ は rf628/rf630/rf631 の verification 記録 | AL 側 CI 定義・manifest への直接影響なし |

結論: 破壊的 API/Feature/OS 変更は検出されず、AL の全検証 lane が
対象 revision で pass。新 API/feature（`RunMetadata` JSON・
`StreamFault::Cancellation`・plugin state bridge・test_editor/
test_stateful fixture・plugin-dock real-editor attach）は
opt-in/test-only/依存木外であり、AL 側の採用 gate・需要が
未定義のため本 slice では採用しない。採用判断は「採用」（pin
追従自体。詳細は task doc と PR 本文）。

## Non-Functional Constraints

- lockfile-primary 契約の維持（manifest `rev` 固定への回帰禁止）。
- fail-closed 検査（check-ruviform-lock/rev）の有効性維持。
- 検証は正規 mise task 経由。cargo 直実行は補助証拠として区別。
- 未実行 gate を Pass にしない（NotRun/Unsupported 語彙の厳守）。
- `serde`/`serde_json` の追加が `ruviform-audio-test`（dev-dep）
  内に留まり、製品依存木・CLI/headless 境界に波及しないこと。

## Risks and Trade-offs

- upstream に plugin state bridge・test_editor/test_stateful
  fixture・plugin-dock real-editor attach など EXT-L3 関連の
  大規模新機能が入ったが AL lane では未踏 → 採用は別 slice の
  判断とし、本 slice では pin 追従の非回帰のみを検証。
- `ruviform-audio-test` は `#[non_exhaustive]` enum への variant
  追加と serde derive のみで非破壊だが、AL が将来
  `StreamFault`/`RunMetadata` を消費する場合は新語彙の扱いが
  必要になる（記録のみ）。
- upstream main の covering CI（`ci`/`daily-ci`/`consumer-verify`）
  は candidate `eafe0a08` に対して未実行（最新は `ac10a2f7` 対象、
  `eafe0a08` には schedule の `ci-issue-tracker-reconcile` のみ）。
  ruviform#625 tracker Open のまま。AL 側の merge 後 CI が統合
  検証の証拠となる。
- rollback は `Cargo.lock` revert + `sync-ui-deps` 再実行で可能
  （生成物のみの変更、製品コード差分なし）。

## Deferred Design Concerns

- `RunMetadata` JSON 永続化・`StreamFault::Cancellation` の AL
  採用設計 | `rf628_oracle` の証拠強化・backend fault 消費に
  接続し得るが現行 oracle は未使用 | 採用判断 slice の spec
  起票時 | semantic/headless pass を実機 acceptance と誤認 |
  本 slice では NotRun 記録のみ
- plugin state bridge・test fixture・real-editor attach の AL
  採用 | AL#127 の EXT-L3 要件と接続するが採用 slice・独立
  Oracle 接続が未定義 | 採用 Issue 起票時 | upstream 機能の
  重複実装リスク | 影響評価に記録済み

## Verification Notes

- [x] `mise run check-ruviform-lock` / `check-ruviform-rev`:
  単一 SHA・vendor 一致（fail-closed）
- [x] `mise run sync-ui-deps` → `check-ui` → `test-ui`:
  対象 SHA で Pass（113 tests）
- [x] `mise run fmt`/`check al-analysis`/`lint al-analysis`/
  `test al-analysis`: 対象 crate Pass（`rf628_oracle` 7 tests
  含む、長尺 scenario 実走）
- [x] `mise run check-cli-headless`: headless 境界 Pass
  （serde/serde_json の dev-dep 波及なしを確認）
- [x] `mise run verify-toolchain`/`cargo fetch --locked`: Pass
- [x] 補助証拠: `mise run check-full`（workspace check）
- [ ] `*-full` の lint/test/build、`check-ui-gpui`/`check-ui-webview`
  各 lane、`check-ci-workflow`/`check-ci-tracker-workflows`:
  本 slice の正規検証コマンド対象外のため未実施（変更は
  al-analysis dev-dep 経路に限定されるため）

---
id: "DESIGN-VxibNPMtkHUoSzNPFJPOgN"
type: "design"
status: "approved"
title: "ruviform eea9e209 pin 追従の設計"
created: "2026-11-28"
updated: "2026-11-28"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/audio-lens/issues/127"
  changes:
    added:
      - type: "file"
        path: "docs/designs/ruviform-eea9e209-rev-watch.md"
    modified:
      - type: "file"
        path: "Cargo.lock"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/specs/ruviform-eea9e209-rev-watch.md"
  implemented-by: []
  depends-on:
    - "docs/specs/ruviform-eea9e209-rev-watch.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/designs/ruviform-87a7ba8f-rev-watch.md"
---

# ruviform `eea9e209` pin 追従の設計

## 方針

lockfile-primary 方式を維持する。manifest に `rev`/`branch`/`tag` を書かず、
`node tools/update-ruviform-rev.mjs <sha>` が全 `ruviform-*` package を単一
resolved SHA に原子解決する。失敗時は lockfile を変更しない fail-closed
契約を維持する。`ui/vendor/ruviform/` は `mise run sync-ui-deps` の生成物で、
gitignore 済みのため commit しない。

本 slice では candidate freeze 後の upstream drift を追いかけない。dispatch
時に再解決した `eea9e2090a23e24dd5cd7a0799b25175c91cad9c` を評価対象と
する。対象を固定することで、検証中に採否対象が動くことを防ぐ。

## 実施手順

1. `git ls-remote https://github.com/scarecrow173/ruviform.git
   refs/heads/main` で candidate を再解決する。dispatch 時の実測値は
   `eea9e2090a23e24dd5cd7a0799b25175c91cad9c`。
2. `node tools/update-ruviform-rev.mjs
   eea9e2090a23e24dd5cd7a0799b25175c91cad9c` を実行し、25 個の
   ruviform package を単一 SHA に更新する。
3. `mise run sync-ui-deps` で `ui/vendor/ruviform` を追随させ、
   `ui/vendor/ruviform/REV` が candidate と一致することを確認する。
   vendor は生成物なので commit しない。
4. `mise exec -- cargo fetch --locked` で locked dependency を fetch する。
5. `mise run check-ruviform-lock`、`mise run check-ruviform-rev`、
   `mise exec -- node tools/check-ruviform-rev.mjs --require-vendor` を
   実行し、lockfile-primary・単一 SHA・vendor REV 一致を確認する。
6. canonical 回帰 lane を実行し、結果を
   `evidence/al127/eea9e209-verification.md` に記録する。

`tools/` の pin 手順スクリプト自体は変更しない。現行の案内文と fail-closed
契約は本 slice の手順と一致している。

## 互換性判断

### lockfile / transitive dependency

`Cargo.lock` の差分は 25 個の ruviform source fragment のみである。
`[[package]]` section の追加・削除はなく、無関係な version bump はない。
upstream crate manifest diff もないため、本 slice では dependency shape の
変化を採用判断へ繰り上げない。

`gpui` は前 slice で `scarecrow173/gpui` tag `v0.2.2-rf2`（peeled commit
`b3144f286b942fbb5f478da73db13a826cef054a`）へ切り替わっており、本 slice
の差分ではない。`build-ui-gpui` は同 source を使う Native Build/Link の
非回帰確認として実行する。

### transport/musical time

`TransportPlan` が tempo map と PPQN を plan-owned source として保持し、
`SpanSink::span` が `MusicalTimeSource` を受け取る変更は破壊的 API 変更で
ある。ただし AL は scheduler/plan/sink を直接利用しないため、consumer
適合コードは不要である。compile が通れば transitive dependency として採用
可能と判断する。

重要な契約は、musical transport が tempo map と PPQN の両方がある場合だけ
有効で、片方が欠けたときに固定 tempo map や PPQN 960 を補わないことで
ある。`call_transport_info` も Playing に限り musical timing を載せ、
Stopped/Paused では保持しない。この behavior は upstream plugin boundary
の正直な negative state であり、AL の product contract ではない。

### audio/test runtime

`ruviform-audio` offline render sink と `ruviform-audio-test` MIDI trace sink
は新 signature への適合である。AL は offline render driver と MIDI trace
surface を直接使わない。AL が利用する `oracle`/`pcm` API に差分がないこと
を確認し、rf628 oracle を含む `test al-analysis` で f64/長尺/PTS/gap/
Validity の非回帰を確認する。

### UI/scenario

`ruviform-ui::ipc` の decision core 化は公開 `parse_intent` の契約維持を
意図した内部再構成である。AL は op 文字列、envelope、Intent/Event types を
既存どおり利用するため、Rust/TS の semantic lane で回帰を検出する。

`ruviform-ui-test` の frame EOF 分類修正は test support の挙動修正であり、
partial frame prefix EOF を clean EOF と誤認しない。AL は `read_frame`/
`write_frame` を直接使わないが、scenario/Fake/GPUI seam を通じて制御
channel の異常系が変わっていないかを観察する。

### 依存木外

archive/structured decision core、WASAPI input xrun/stereo、virtual audio
driver、plugin format test adapter、consumer-verify workspace manifest pin
helper は、現行 AL lock 25 package に入らない、または AL の製品 dependency
tree の直接対象外である。本 slice では採用条件にしない。将来これらの
機能を AL が利用する場合は、利用 slice で独立に受入条件を定義する。

## 新 API の採用/見送り

- musical transport API: 見送り。AL 側利用要件がなく、missing half を補わ
  ない契約の consumer 設計もまだない。
- plugin bridge/core handoff: NotApplicable。AL は plugin host/device
  boundary を提供しない。
- MIDI trace/test runtime: 見送り。AL の rf628 oracle は PCM surface のみを
  使う。
- IPC decision core・frame EOF 分類: pin 追随として採用。AL 側変更は不要。
- archive/structured/audio-device/driver/consumer-verify 差分: NotApplicable
  または Deferred。AL 依存木外として記録する。

## リスク

- `SpanSink::span` の破壊的変更は、直接利用する consumer で compile error
  を起こしうる。AL では direct use がなく compile/test は非回帰だったが、
  将来 transport scheduler を導入する場合は新 signature と musical source
  contract を前提にする。
- upstream `consumer-verify` は candidate で verify-audiolens success だが
  verify-synvelis failure / summary failure である。cross-consumer tuple は
  本 slice では未解決のまま Coordinator/後続 slice へ残す。
- upstream `ci` run 46134393707 は candidate 自体で success だが、AL の
  統合受入を代行しない。AL の統合検証は merge 後対象 SHA の CI 証拠で
  判断する。
- local/headless lane の成功は interactive real window、pixel、OS input、
  実機、macOS/Linux OS-L1 の証拠ではない。

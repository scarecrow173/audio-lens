---
id: "SPEC-hDvlkUICOQWZbnaQJCKBFW"
type: "spec"
status: "approved"
title: "ruviform pin eea9e209 追従評価（AL#127 pin 追従 slice）"
created: "2026-11-28"
updated: "2026-11-28"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/audio-lens/issues/127"
  changes:
    added:
      - type: "file"
        path: "docs/specs/ruviform-eea9e209-rev-watch.md"
      - type: "file"
        path: "docs/designs/ruviform-eea9e209-rev-watch.md"
      - type: "file"
        path: "docs/test-specs/ruviform-eea9e209-rev-watch.md"
      - type: "file"
        path: "evidence/al127/eea9e209-verification.md"
    modified:
      - type: "file"
        path: "Cargo.lock"
      - type: "file"
        path: "docs/evidence/ruviform-verification.md"
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
    - "docs/specs/ruviform-87a7ba8f-rev-watch.md"
  derives-from:
    - "docs/specs/ruviform-lockfile-source-of-truth.md"
  derived-by: []
  verifies: []
  verified-by:
    - "docs/test-specs/ruviform-eea9e209-rev-watch.md"
---

# ruviform pin `eea9e209` 追従評価

## 背景

AL#127 常設 tracker の headless pin 追従 slice。現行 pin
`87a7ba8ff3fbe6d4502bea27d152d7f74c105899` に対し、dispatch 時に
`git ls-remote` で再解決した upstream ruviform main HEAD
`eea9e2090a23e24dd5cd7a0799b25175c91cad9c` への bump を実施し、
差分影響と採用/保留/rollback 判断を記録する。

dispatch 時の再解決では起稿時点 candidate からの drift はなかった。
したがって `eea9e209` を candidate freeze とする。検証中に upstream main が
進んだ場合でも、本 slice の採否対象はこの freeze SHA とし、後続分は次回
rev-watch の対象とする。

## 候補

- 採用 candidate: `eea9e2090a23e24dd5cd7a0799b25175c91cad9c`
  （dispatch 時 upstream main HEAD、`87a7ba8f` の descendant で
  69 commits 先行）
- 旧 pin: `87a7ba8ff3fbe6d4502bea27d152d7f74c105899`
- upstream 差分: 69 commits・483 files（+11,813,666/−1,509）。
  大半は upstream evidence と検証資産であり、evidence/.agent-skills を
  除く非 evidence は 54 files（+16,133/−1,480）。`crates/` 配下は
  26 files（+12,683/−1,332）。
- AL lock: 25 個の ruviform package が単一 resolved SHA
  `eea9e2090a23e24dd5cd7a0799b25175c91cad9c` に一致。

## 採用/保留/rollback 判定

- **採用**: `eea9e209` — pin 一貫性 lane、analysis/service/CLI/GUI の
  check/lint/test、GPUI semantic lane、`al-desktop` Native Build/Link、
  real-headless blocked-probe、pre-commit が全て exit 0。AL 側コード
  変更は不要だった。
- **保留**: なし。
- **rollback 条件**: ruviform 起因の Fail が後続検証で確定した場合、
  `node tools/update-ruviform-rev.mjs
  87a7ba8ff3fbe6d4502bea27d152d7f74c105899` で旧 SHA へ復帰する
  （lockfile-primary のため manifest 変更は不要）。rollback した場合も
  対象 lane・失敗理由・復帰後結果を evidence に記録する。

## 影響範囲

### `Cargo.lock` / dependency source

`Cargo.lock` の差分は 25 個の ruviform package の git source fragment が
`87a7ba8f…` から `eea9e209…` へ更新される変更のみである。
`[[package]]` section の追加・削除はなく、無関係な version bump や新規
package の混入はない。upstream crate manifest には差分がない。

`gpui` 0.2.2 は現行 lock で既に `scarecrow173/gpui` tag `v0.2.2-rf2`
（peeled commit `b3144f286b942fbb5f478da73db13a826cef054a`）へ切り替わって
おり、本 slice の新規 source switch ではない。

### AL 依存木内で production `src/` 差分がないもの

`ruviform-audio-analysis`/`ruviform-audio-core`/`ruviform-dsp`/
`ruviform-exec`/`ruviform-graph`/`ruviform-io-avi`/`ruviform-io-file`/
`ruviform-io-symphonia`/`ruviform-math`/`ruviform-media-io`/
`ruviform-numeric`/`ruviform-observability`/`ruviform-plugin-host`/
`ruviform-rt`/`ruviform-ui-gpui`/`ruviform-ui-theme`/`ruviform-video-core`/
`ruviform-viz` には production `src/` 差分がない。

### `ruviform-media-core` / `ruviform-audio` / plugin bridge/core

`ruviform-media-core` の `TransportPlan` は `musical_ppqn` を保持し、
builder は `with_musical_ppqn`/`with_musical_transport` を得る。
`SpanSink::span` は第 3 引数 `MusicalTimeSource<'_>` を取る破壊的 API
変更である。musical transport は tempo map と PPQN の両方が active plan に
ある場合だけ有効で、missing half を固定値で補わない。

`ruviform-audio` の offline render sink、`ruviform-audio-plugin-bridge` の
production graph node、`ruviform-plugin-core` の transport bridge はこの新
contract へ適合し、Playing かつ tempo map+PPQN が揃う場合だけ musical
timing を `TransportInfo` へ載せる。Stopped/Paused/missing source では
捏造・保持しない behavior fix である。

AL の `crates/`/`tools/`/`ui` には `SpanSink`、`TransportPlan`、
`TransportPlanBuilder`、`musical_ppqn`、plugin host/device boundary の直接
利用がない。したがって本 slice では transitive compile 影響として採用し、
musical transport API の消費は見送る。

### `ruviform-audio-test`

`ruviform-audio-test/src/midi.rs` の `TraceSink` が新 `SpanSink` signature
へ適合する。`tests/os_endpoint.rs`/`tests/rt_contract.rs` には render
timestamp observation、epoch classifier、OS target gate 等の upstream test
runtime 強化が入る。AL の rf628 oracle は `ruviform_audio_test::oracle` と
`ruviform_audio_test::pcm` のみを使い、`midi` surface は直接利用しない。
oracle 非回帰は `mise run test al-analysis` で確認する。

### `ruviform-ui` / `ruviform-ui-test`

`ruviform-ui::ipc` の op/envelope validation は byte-level pure decision
core へ再構成され、Kani proof が追加された。公開 `parse_intent` 契約は
維持意図であり、AL は `ruviform_ui::ipc` types を使うため semantic lane で
非回帰を確認する。

`ruviform-ui-test` の control-channel frame length codec は decision core
化され、partial frame prefix EOF が clean EOF ではなく `UnexpectedEof` /
truncation として分類される test-only behavior fix である。AL は
`read_frame`/`write_frame` を直接使わないが、scenario/Fake/GPUI seam 経由の
regression を観察する。

### 依存木外

`ruviform-audio-device` の WASAPI input forward-gap xrun mapping、virtual
cable stereo channel preservation、`drivers/rvf-virtual-audio` の deterministic
capture xrun injection、plugin format test adapter、`ruviform-io-archive` の
ZIP/TAR budget decision core と TAR metadata block budget fix、
`ruviform-io-structured` の CSV/input budget decision core、rf#1064 系
consumer-verify workspace manifest pin helper は、AL lock 25 package に入ら
ない、または AL dependency tree の直接対象外である。将来導入時の採用条件と
混同せず、本 slice では NotApplicable/依存木外へ分類する。

## 新 API の採用/見送り

- `musical_ppqn`/`with_musical_transport`/`MusicalTimeSource`: **見送り**。
  AL は transport scheduler/plan を直接消費しない。将来導入する場合は
  tempo map と PPQN を plan 単位で両方供給する contract を前提にする。
- plugin bridge/core の musical transport handoff: **NotApplicable**。
  AL は plugin host/device boundary を製品機能として提供しない。
- `ruviform-audio-test` の MIDI trace/runtime 強化: **見送り**。AL は
  PCM oracle のみを利用し、MIDI trace surface は消費しない。
- IPC decision core: **採用（pin 追随）**。公開 wire contract は変えず、
  AL 側利用変更は不要。
- control-channel EOF 分類 fix: **採用（pin 追随）**。AL から direct
  consumer はないが、test-only の正直な失敗分類として受け入れる。
- archive/structured budget API、audio-device xrun/stereo、virtual endpoint
  provisioning、consumer-verify pin helper: **NotApplicable/Deferred**。
  AL lock または製品依存木の対象外であり、本 slice の採用条件に繰り上げない。

## 独立検証

`evidence/al127/eea9e209-verification.md` に pin 手順、差分実測、canonical
回帰 lane、upstream covering CI、NotRun/Unsupported/Deferred を記録する。
candidate 自体では upstream `ci` run 46134393707 が success。
`consumer-verify` run 46144010070 は verify-audiolens success /
verify-synvelis failure / summary failure であり、verify-audiolens の success
を AL の独立 Pass へ転用しない。verify-synvelis failure は cross-consumer
gate の未解決事実として扱う。

AL 側の統合検証は merge 後の対象 SHA に対する branch/Daily CI 証拠とし、
本 slice の local/headless lane と混同しない。

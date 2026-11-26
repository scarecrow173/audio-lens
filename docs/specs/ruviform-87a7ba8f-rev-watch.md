---
id: "SPEC-wbaYvKekva0NN7wHt540LW"
type: "spec"
status: "approved"
title: "ruviform pin 87a7ba8f 追従評価（AL#127 pin 追従 slice）"
created: "2026-11-26"
updated: "2026-11-26"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/audio-lens/issues/127"
  changes:
    added:
      - type: "file"
        path: "docs/specs/ruviform-87a7ba8f-rev-watch.md"
      - type: "file"
        path: "docs/designs/ruviform-87a7ba8f-rev-watch.md"
      - type: "file"
        path: "docs/test-specs/ruviform-87a7ba8f-rev-watch.md"
      - type: "file"
        path: "evidence/al127/87a7ba8f-verification.md"
    modified:
      - type: "file"
        path: "Cargo.lock"
      - type: "file"
        path: "tools/update-ruviform-rev.mjs"
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
    - "docs/specs/ruviform-e4c9f6f0-rev-watch.md"
  derives-from:
    - "docs/specs/ruviform-lockfile-source-of-truth.md"
  derived-by: []
  verifies: []
  verified-by:
    - "docs/test-specs/ruviform-87a7ba8f-rev-watch.md"
---

# ruviform pin `87a7ba8f` 追従評価

## 背景

AL#127 常設 tracker の headless pin 追従 slice。現行 pin
`e4c9f6f0715da3ac479f8c437d21a021327e51da` に対し、dispatch 時に
`git ls-remote` で再解決した upstream ruviform main HEAD
`87a7ba8ff3fbe6d4502bea27d152d7f74c105899` への bump を実施し、
差分影響と採用/保留/rollback 判断を記録する。

dispatch 時の再解決では起稿時点 candidate からの drift はなかった。
検証後の再確認では upstream main が
`0fb97d1ba89e1fee154cdbbb72ceb2db15fdcd8c` へ進んでいたが、これは
candidate freeze 後の drift である。本 slice の採否対象は dispatch 時に
確定した `87a7ba8f` とし、`0fb97d1b` は次回 rev-watch の対象とする。
`87a7ba8f..0fb97d1b` は `ruviform-audio-device` の 1 commit で、同 package は
本 slice の `Cargo.lock` に含まれない。

## 候補

- 採用 candidate: `87a7ba8ff3fbe6d4502bea27d152d7f74c105899`
  （dispatch 時 upstream main HEAD、`e4c9f6f0` の descendant で
  281 commits 先行）
- 旧 pin: `e4c9f6f0715da3ac479f8c437d21a021327e51da`
- upstream 差分: 281 commits・3,758 files（+212,585/−247,733。
  rename detection は件数上限で省略）。`crates/` 配下は 157 files
  （+56,418/−5,897）。
- AL lock: 25 個の ruviform package が単一 resolved SHA
  `87a7ba8ff3fbe6d4502bea27d152d7f74c105899` に一致。

## 採用/保留/rollback 判定

- **採用**: `87a7ba8f` — pin 一貫性 lane、analysis/service/CLI/GUI の
  check/lint/test、GPUI semantic lane、`al-desktop` Native Build/Link、
  real-headless blocked-probe、pre-commit が全て exit 0。AL 側コード
  変更は不要だった。
- **保留**: なし。
- **rollback 条件**: ruviform 起因の Fail が後続検証で確定した場合、
  `node tools/update-ruviform-rev.mjs
  e4c9f6f0715da3ac479f8c437d21a021327e51da` で旧 SHA へ復帰する
  （lockfile-primary のため manifest 変更は不要）。rollback した場合も
  対象 lane・失敗理由・復帰後結果を evidence に記録する。

## 影響範囲

### `Cargo.lock` / dependency source

`Cargo.lock` の差分は次の 2 系統のみである。

1. 25 個の ruviform package の git source fragment が
   `e4c9f6f0…` から `87a7ba8f…` へ更新される。
2. `gpui` 0.2.2 の source が crates.io registry から
   `https://github.com/scarecrow173/gpui` tag `v0.2.2-rf2`
   （peeled commit `b3144f286b942fbb5f478da73db13a826cef054a`）へ
   切り替わる。

`[[package]]` section の追加・削除はなく、無関係な version bump や新規
package の混入はない。upstream `ruviform-ui`/`ruviform-ui-test` の
`proptest` は dev-dependency であり、AL lock では既存の 1 entry のまま
増えない。`ruviform-audio-device`/`ruviform-midi-device` は AL lock に
含まれない。gpui source switch は upstream
`ruviform-ui-gpui` の dependency 変更に伴う transitive dependency effect
であり、手編集や rollback で隠さず採用判断へ含める。

### AL 依存木内で production `src/` 差分がないもの

`ruviform-audio-analysis`/`ruviform-audio-core`/`ruviform-dsp`/
`ruviform-exec`/`ruviform-graph`/`ruviform-io-avi`/`ruviform-io-file`/
`ruviform-media-core`/`ruviform-numeric`/`ruviform-rt`/
`ruviform-ui-theme`/`ruviform-video-core` は production `src/` 差分なし。
`ruviform-io-symphonia`/`ruviform-media-io`/`ruviform-observability`/
`ruviform-viz` も production `src/` 差分はなく、確認できた差分は
README 等に限られる。`ruviform-audio-plugin-bridge` と
`ruviform-audio-test` も `src/` 差分はない。

### `ruviform-audio` / `ruviform-math`

- `ruviform-audio/src/typed/probe.rs`: ProbeDeck capture chunk の
  `chunks_out.try_push` が `debug_assert!` 内の副作用から常時実行へ修正
  された（upstream commit `7853ccd2`）。AL は `ProbeDeck` を直接参照
  しないため transitive compile のみだが、capture 系利用時には upstream
  bugfix として意味がある。
- `ruviform-math/src/dsp.rs`: 証拠文書参照コメントの更新のみで、
  production 意味論差分はない。

### plugin-core/plugin-host

`ruviform-plugin-core` と `ruviform-plugin-host` に大規模な差分がある。
主な内容は GUI floating/editor verb、plugin→host GUI request channel、
state dirty/param notice、L1 diagnostics、wire version 5→9、bus report
drain surface である。

特に `Plugin::buses` は `&[BusDescriptor]` から
`Option<&[BusDescriptor]>` へ変わる契約変更で、`None` は bus enumeration
unavailable、`Some(&[])` は declared none を意味する。`ErasedPlugin` も同じ
signature へ追随する。これは API 互換性上の破壊的変更だが、AL の
`crates/`/`tools/`/`ui` には plugin API の実装・利用がなく、compile/test
lane でも適合変更は不要だった。したがって AL では transitive compile のみの
影響として採用し、plugin host/device boundary 自体は AL 非提供のため
NotApplicable とする。

wire version bump は upstream L1 out-of-process plugin 境界の内部契約で
あり、AL の永続化形式・CLI/GUI wire 契約ではない。AL が plugin host を
利用開始する場合に限り別 slice で評価する。

### ruviform-ui / ui-test / ui-gpui

- `ruviform-ui`: `InputSpec`/`InputValueSpec`（Text・bounded Integer・
  bounded Number）、`ViewHost::submit_input`、`CommandParam`/
  `CommandParams` が追加された。`ComponentSpec::Input` は additive で、
  `CommandIntent::Action.params` の serde field 自体は旧 pin に既存であり、
  optional/default の形状も変わらない。したがって既存 command wire の
  serde 互換性は保たれる。
- `ruviform-ui-test`: out-of-process plugin host 実行制御用の `rvf_ctl`、
  `ScenarioCapability::OutOfProcessPluginHost`、transport/command param
  tests が追加された。AL は capability を新たに主張しない。
- `ruviform-ui-gpui`: Windows IMM32 IME capability 向け `os_ime.rs`、
  native slot container/surface compositing、lane provenance capability
  claim の変更を含む。`gpui` dependency が crates.io 0.2.2 から fork tag
  `v0.2.2-rf2` へ切り替わる。`gpui-test` feature の check/lint/test と
  `gpui` feature の native build は通ったが、interactive real window、
  pixel、OS input、IME の実証は本 slice では行わない。

### 依存木外

upstream 全体差分の大半は docs/evidence/skill 記録と upstream 自身の
virtual audio/driver/endpoint workflow 整備である。AL lock に現れない
`ruviform-audio-device`、plugin format adapter、virtual endpoint
provisioning、driver install、実機検証資産は AL の依存木外であり、
本 slice では NotApplicable/Deferred とする。

## 新 API の採用/見送り

- `InputSpec`/`InputValueSpec`/`submit_input`: **見送り** — AL は現行
  GUI に free-form input widget を持たず、消費は別 slice で評価する。
- `CommandParam`/`CommandParams`: **見送り** — 既存 `Action.params` の
  arbitrary JSON transport を AL が typed profile へ移行する必要は本 slice
  ではない。serde 形状は非回帰。
- `ScenarioCapability::OutOfProcessPluginHost`: **NotApplicable** — AL は
  plugin host を提供しない。capability claim へ転用しない。
- plugin GUI/editor verbs、dirty/param notices、wire v6-v9: **見送り/
  NotApplicable** — AL は plugin host API を消費しない。
- Windows IMM32 IME / native real-window capability: **Deferred** — build
  と semantic lane は通ったが、interactive desktop での実測前に Pass を
  主張しない。
- ProbeDeck capture push bugfix: **採用（pin 追随）** — AL から direct
  consumer はないが、pin に含まれる upstream bugfix として受け入れる。
  独立した AL 側利用証拠は要求しない。
- virtual endpoint/xrun injection: **NotApplicable** — candidate 後の
  upstream main または AL 依存木外の領域であり、本 slice の採用条件に
  繰り上げない。

## 独立検証

`evidence/al127/87a7ba8f-verification.md` に pin 手順、差分実測、
canonical 回帰 lane、upstream covering CI、NotRun/Unsupported/Deferred を
記録する。upstream `87a7ba8f` 自体の covering CI run は存在せず NotRun。
直近 ancestor `ab6dc99fe8935ac09082d05e2176b10e729ba263` では
`ci` run 45749815004・`daily-ci` run 45749786810 が success で、
`ab6dc99f..87a7ba8f` は docs/evidence/skill 記録のみで production code
差分はない。

`scarecrow173/ruviform#294` の cross-consumer 集約結果は本 slice では
未取得（Coordinator/後続 slice の対象）。AL 側の統合検証は merge 後の
対象 SHA に対する branch/Daily CI 証拠とし、本 slice の local/headless
lane と混同しない。

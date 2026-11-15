---
id: "DESIGN-034Y1A2b3C4d5E6f7G8h9"
type: "design"
status: "approved"
title: "ruviform pin abb9f372 追従 + consumer oracle 設計"
created: "2026-11-15"
updated: "2026-11-15"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/audio-lens/issues/127"
    - "https://github.com/scarecrow173/ruviform/issues/628"
  changes:
    added:
      - type: "file"
        path: "docs/designs/ruviform-abb9f372-rev-watch.md"
    modified: []
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
    - "docs/designs/ruviform-ac10a2f7-rev-watch.md"
    - "docs/designs/issue-127-rf628-consumer-oracle.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/specs/ruviform-abb9f372-rev-watch.md"
  derived-by: []
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# ruviform pin abb9f372 追従 + consumer oracle 設計

## Context

pin `ac10a2f7` → `abb9f372d3ee95ee824b3c4f7dc68e8083ce2774`
（ruviform main HEAD）。`ac10a2f7..abb9f372` は 11 commit・
182 file・+31,238/−528 行。AL 依存木内の crates/ 変更は
`ruviform-audio-core`（+`midi_output` module、Issue #629）、
`ruviform-ui-test`（+plugin-dock scenario catalog、Issue #631）、
`ruviform-ui-gpui`（+real_window native-slot lane、Issue #631）
の 3 crate のみで、いずれも additive・非破壊。
`ruviform-audio-test` は AL へ新規 dev-dep として入る
（`pcm.rs`/`oracle.rs` は ac10a2f7 から無変更、`pipeline/`・
`backend/`・`midi/` module が Issue #628 slice 2-3 で追加）。

lockfile の ruviform package は 20 → 26 に増加。増分は
`ruviform-audio-test` 本体とその推移的依存（`ruviform-audio`・
`ruviform-graph`・`ruviform-plugin-core`・`ruviform-plugin-host`・
`ruviform-audio-plugin-bridge`・`ruviform-io-symphonia`・
`symphonia` 系・`shared_memory` 等）で、dev-dep 経路のみ。
製品配布物への影響はない（`publish = false` の test-only crate、
`al-analysis` `[dev-dependencies]` 限定）。

## Scope

### In Scope

- pin bump（lockfile-primary）と dev-dep 追加。
- `tests/rf628_oracle.rs`（詳細設計は
  `docs/designs/issue-127-rf628-consumer-oracle.md` が正本）。
- 差分影響評価・採用判断・rollback 記録。

### Out of Scope

- spec の Out of Scope と同じ（製品コード変更・clock/device
  legs・CLI/GUI/Export・3OS/cross-consumer・Issue #127 Close）。

## Components and Boundaries

- **fixture/oracle（ruviform、被験ではない）**:
  `pcm::{PcmFixture,PcmScenario,PcmSegment,PcmPattern,PcmBlock}`、
  `oracle::{pcm_reference_block,pcm_assert_eq,PcmCompare,
  pcm_max_abs_diff,pcm_bits_eq}`。`pcm`/`oracle` は前 pin から
  無変更のため、fixture 契約の差分リスクはない。
- **新規 module（本 slice では未使用・文書参照のみ）**:
  `pipeline::{input,output,midi,common}`（scripted backend を
  production consumer へ接続する Issue #628 slice 3）、
  `backend::*`、`midi::*`、`clock::*`。AL は file-based で
  device boundary consumer ではないため AL-5 NotApplicable、
  AL-4 Deferred。
- **AL 側消費（被験）**: `crates/al-analysis/tests/rf628_oracle.rs`
  が `pcm`/`oracle` のみを使用し、`backend`/`pipeline`/`midi`/
  `clock`/`metadata` には触れない。

## 影響評価（軸別）

| 軸 | 変更 | AL 影響 | 判断 |
| --- | --- | --- | --- |
| f64 精度 | なし（解析 DSP 無変更） | なし | — |
| `ruviform-audio-core` | +`midi_output` module（additive） | なし（未使用） | 採用（pin 追従に包含） |
| `ruviform-ui-test` | +scenario catalog（additive） | なし（AL-GUI は別経路） | 採用（pin 追従に包含） |
| `ruviform-ui-gpui` | +native-slot dock lane（additive） | なし | 採用（pin 追従に包含） |
| `ruviform-audio-test` | `pcm`/`oracle` 無変更、`pipeline`/`backend`/`midi` 追加 | dev-dep のみ、pcm/oracle のみ消費 | 採用（本 slice の目的） |
| device/pipeline/midi | 新規 module 多数 | AL 非消費 | 採用しない（NotApplicable 記録） |
| plugin test fixtures | `plugin-dock-test`/`test-clap`/`test-vst3` 新規 | 依存木外 | 採用しない |
| `ruviform-midi-device` | MIDI loopback verifier 等 | 依存木外 | 採用しない |
| io-e2e lanes | G01/G05 等 | 依存木外 | 採用しない |

## Risks and Trade-offs

- dev-dep 推移的依存で lockfile +6 package・`--all-targets`
  check のコンパイル対象が増える → test-only・製品影響なし。
  `check-ruviform-lock` で単一 SHA を確認済み。
- `pipeline/`/`backend/` API は今後の AL device consumer slice で
  再評価が必要 → rev-watch に存在を記録。
- rollback: `git checkout -- Cargo.lock` + manifest から
  `ruviform-audio-test`/`ruviform-audio-core` dev-dep 削除 +
  `node tools/update-ruviform-rev.mjs ac10a2f7...` +
  `mise run sync-ui-deps` で旧 pin へ復帰（生成物のみ）。

## Verification Notes

- [ ] `mise run check-ruviform-lock`/`check-ruviform-rev`: Pass
- [ ] `mise run sync-ui-deps` → `ui/vendor/ruviform/REV` 一致
- [ ] `cargo test -p al-analysis --test rf628_oracle`: Pass
- [ ] `mise run test al-analysis` 他 lane: 結果記録

---
id: "TASK-034H79j4zxpqdARj0xtWcM"
type: "task"
status: "todo"
title: "Streaming Bandpass 実装 (issue #71 P2)"
created: "2026-10-16"
updated: "2026-10-16"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/audio-lens/issues/71"
  changes:
    added: []
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "../designs/streaming-bandpass-issue-71-p2.md"
  implemented-by: []
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "./valid-fft-ncc-plan-issue-71-p1.md"
  refines: []
  refined-by: []
  derives-from:
    - "../designs/streaming-bandpass-issue-71-p2.md"
    - "../specs/streaming-bandpass-issue-71-p2.md"
    - "../adr/bandfilter-inputcache.md"
  derived-by: []
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# Streaming Bandpass 実装 (issue #71 P2)

## Work

`band_gain` の帯域波形を全帯域同時保持から帯域ごと逐次生成へ変える。
al-math に `BandFilter`(forward スペクトル保持 + per-band
mask/IFFT)を追加し、`bandpass_many` はそれへ委譲。
`band_gain::analyze` は `InputCache` を使わず `BandFilter` で
帯域ループを回す。

## Classification

- `normal-plan-task`

## Upstream Documents

- `docs/specs/streaming-bandpass-issue-71-p2.md`
- `docs/adr/bandfilter-inputcache.md`
- `docs/designs/streaming-bandpass-issue-71-p2.md`

## Dependencies

- P1(Valid FFT NCC)のマージ済みコードに依存
  (`CorrelationPlan::new_ncc`)。

## Affected Files

- `crates/al-math/src/filter.rs` — `BandFilter` 追加、
  `Bandpass` カーネルとの共通内部関数化
- `crates/al-locate/src/dsp.rs` — `BandFilter` の re-export
- `crates/al-analysis/src/band_gain.rs` — `analyze` の逐次生成化、
  `InputCache` import 除去
- `crates/al-math/tests/` または al-analysis tests — メモリ計測
  (counting allocator 先例 `al-math/tests/ncc_workspace.rs`)

## Done When

- [ ] `BandFilter::new`/`band` が `bandpass_many` と同一仕様
      (mask・切り戻し・NaN hi・空入力)で動作する
- [ ] `bandpass_many` が `BandFilter` 経路と同じ内部実装を共有する
- [ ] `band_gain::analyze` が `InputCache` 非経由で帯域逐次生成する
- [ ] `prepare_with_bands`/`PreparedChannel`/`InputCache` が無変更
      (locate 契約維持)
- [ ] 全テスト・clippy・doc audit が通る
- [ ] 実サンプル run で `analysis::reference` busy とメモリを記録し、
      discovery doc に反映する

## Verification

- [ ] `cargo test -p al-math -p al-locate -p al-analysis`
- [ ] `cargo clippy --workspace --all-targets`
- [ ] `node .agents/skills/doc-status/scripts/audit_docs.js --type all`
- [ ] counting allocator テストで帯域同時生存が 2 信号以内であること
- [ ] 実サンプル run で busy/RSS の before/after を記録

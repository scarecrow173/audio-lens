---
id: "IMPL-034Y19C4d5E6f7G8h9J0k1"
type: "impl"
status: "completed"
title: "Issue #127 RF#628 consumer oracle 実装記録"
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
        path: "crates/al-analysis/tests/rf628_oracle.rs"
      - type: "file"
        path: "docs/impl/ir/issue-127-rf628-consumer-oracle.md"
    modified:
      - type: "file"
        path: "Cargo.toml"
      - type: "file"
        path: "Cargo.lock"
      - type: "file"
        path: "crates/al-analysis/Cargo.toml"
    deleted: []
    renamed: []
    moved: []
    generated:
      - type: "file"
        path: "ui/vendor/ruviform"
  implements:
    - "docs/tasks/issue-127-rf628-consumer-oracle.md"
  implemented-by: []
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/impl/ir/ruviform-abb9f372-rev-watch.md"
  refines: []
  refined-by: []
  derives-from: []
  derived-by: []
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
metadata:
  experiments:
    adopted: []
    rejected: []
---

# Issue #127 RF#628 consumer oracle 実装記録

## Decisions

- 単一テストファイル `crates/al-analysis/tests/rf628_oracle.rs` に
  AL-1/AL-2/AL-3 を集約（helper 共有のため分割しない）。
- fixture→artifact 経路は `render_fixture`/`render_scenario`
  （`PcmBlock.data` planar f32 → `f64::from` exact widen →
  channel 別 Vec → `signal::interleave` → `pcm_f64le`）で、
  golden.rs の `fixture()`/`ctx()` パターンを踏襲。
- 期待値は (a) `oracle::pcm_reference_block` 出力の f64 widen
  統計量（`level_expect` で dc/peak/rms/4x true-peak を再実装）
  と (b) 解析的参照（constant 厳密値・sine/square tonal 周波数・
  segment 幾何由来 validity map）の 2 系統。`pcm_assert_eq` は
  fixture 入力データの妥当性確認に限定し AL Pass 判定に使わない。
- AL-2 は 30s（3×10s segment、Noise→Sine→Square）に決定。
  実測: 60s→150s、45s→130s、30s→~85s（debug）。支配コストは
  locate 相互相関でほぼ線形。tensor chunk 境界・trajectory 長は
  30s で十分に複数 chunk を踏む。
- AL-3 gap は `render_scenario` が未カバー frame を `f64::NAN`
  で埋め、`decode_input` の plain decoded_pcm 経路（non-finite
  拒否は normalized_pcm のみ）経由で NaN を channel_samples へ
  届かせる。期待 NaN window 集合は segment `[start,start+len)`
  と window/hop=960/480 の幾何から独立導出。

## Code Changes

- `crates/al-analysis/tests/rf628_oracle.rs`: 新規 ~730 行、
  7 test（fixture oracle 照合・f64 fidelity 4 本・長尺・gap）。
- `crates/al-analysis/Cargo.toml`: dev-deps に
  `ruviform-audio-test` + `ruviform-audio-core`。
- `Cargo.toml`/`Cargo.lock`: pin `abb9f372` + git dep。

## Evidence

- `cargo test --locked -p al-analysis --test rf628_oracle`:
  7/7 Pass（Windows debug）。内訳: fixture oracle・constant・
  noise・stereo・periodic(sine/square)・gap ≈ 各 1-2s、
  long_scenario ≈ 85s（cheap 6.8s + locate/reference 77.3s）。
- `cargo check --locked --all-targets -p al-analysis`: Pass。
- `node tools/check-ruviform-lock.mjs`/`ruviform-lock-rev.mjs`:
  26 package 単一 SHA Pass。
- その他 lane は task doc/PR 本文に実測値を記録。

## Observed Semantics（pin した既存挙動、変更なし）

- `level.dc_offset`: NaN 伝播で JSON `null`（record validity は
  `Valid` のまま — 値レベルの欠測として観測可能）。
- `level.sample_peak`/`true_peak_est`: `f64::max` の NaN 無視で
  有限（valid 区間の最大）。
- `level.rms_dbfs`/`integrated_dbfs`: NaN → `db()` floor -240.0。
- `level.trajectory` tensor: NaN を含む window で NaN、
  robust summary record（`rms_median/mad/p05/p95_dbfs`）の
  `support_count` は有限窓数（gap テストで 398/449 を確認）。
- `level.crest_factor`: rms_all NaN で 0.0。
- `dig.*`: NaN サンプルは閾値比較をすり抜けるが分母に含まれる。
  `quant_step_est` は NaN 差分を除外して有限値。
- 全 record の `params.ruviform` = `{rev: <lockfile SHA>,
  precision: "f64", crates: [...]}`。

## Unresolved / Limitations

- `noise.tonal_peak_hz` は NaN STFT フレームで `argmax`
  （`total_cmp`）が NaN を最大扱いする既存規則の影響を受け得る
  （gap テストでは record 存在を確認したが値の縮退は pin して
  いない — 有限フレームが残る本ケースでは実害なし）。改善は
  別 Issue の判断。
- AL-4 Deferred・AL-5 NotApplicable（spec/test-spec 参照）。
- 実験ログは `docs/impl/exp/` に記録。

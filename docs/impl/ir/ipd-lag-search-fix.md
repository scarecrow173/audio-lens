---
id: "IMPL-034FQsvZu2ciTHdGMozQAf"
type: "impl"
status: "completed"
title: "IPD lag search fix"
created: "2026-10-14"
updated: "2026-10-14"
owners: ["scarecrow173"]
relations:
  source: []
  changes:
    added: []
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/tasks/ipd-lag-search-fix.md"
  implemented-by: []
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related: []
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

# IPD lag search fix

## Summary

IPD caller のfull-margin検索窓とsigned原点を修正した。NCC合法clamp/FFT APIは変更なし。
今回IPD sliceは独立Judge ACCEPT。fixを独立commitし、次にfix後性能baselineを取得する。reuseは未実装。

## Context

#52 approvedplanのSlice1。nccに同長blockを渡すと合法clamp0になり、index0から
max_lagを引くと固定偽lag、drift0になる。違反境界はcheap callerのshape/座標。

## Implementation

ipd_lagsで両channel長のminを使いblock+2marginを完全に持つ窓だけ分析。
ipd_block_lagはNCC offsetから検索原点を引く。signed-correlation max/tieは従来を維持。
first pair/100ms、max0/negativeconfigのusize飽和を維持。exactblockは1観測として含める。

## Related Experiments

探索実験なし。承認されたcaller境界fixのred/greenを実行。

## Validation

最初のtest記述はFeatureRecordのsupport field名/型で2回compile errorとなった。
これはred証拠とは扱わない。修正後、product変更前に以下2件の実挙動redを確認:

- `ipd_lag_drift_reports_negative_to_positive_change_for_first_pair`: Some(0) vs expected Some(8)。
- `ipd_lag_zero_width_exact_block_is_observed`: mod.lag_drift_samples record欠落。

`mise exec -- cargo test -p al-analysis ipd_lag` は2failed/exit1、その後callerfixで2passed。
境界4回帰を追加し `mise exec -- cargo test -p al-analysis --lib cheap::tests` は17passed。
signed -8/-3/0/+5/+8、constantlag、negativecfg、双方の完全margin/short/巨大幅を確認。
`mise exec -- cargo fmt --all -- --check` exit0。
`mise exec -- cargo test -p al-analysis` は99unit+7avsync+8expensive+19golden+12reference、計145passed、doc0。
`mise exec -- cargo clippy -p al-analysis --all-targets -- -D warnings` exit0。
IR全72文書audit findingsなし、diffcheck成功。

fixture/artifact: `target/ncc-reuse-evidence/ipd-red-green/verification.json` と2channel f64le。

| Fixture | SHA256 |
| --- | --- |
| channel0.f64le | 207422e994a86079e6abbb56038aac0a7e9f7fb6c1075a6eeff64ac98bd1eaa5 |
| channel1.f64le | bfc787fd34c2008237a30c3649f84c3a2481738519c30a6bb6d4ffd1fb2e11e4 |

静的baselineはf6273917bd7ccd8637828b934676d7b31a7038c6、性能baselineではない。
性能計測はreview/独立IPD commit後に取得するためまだspeedupを主張しない。

## Risks

exactblock/右端を含む観測数が意図的に修正される。短入力でlag観測を作らない。
NCC empty helperは内部trusted呼出境界でfullshapeを保証し、public NCC契約は変更しない。

## Follow-ups

独立slice review→IPDfixcommit→fix後baseline→math所有分離。その他tasksはtodo。

Review snapshot: HEAD f6273917bd7ccd8637828b934676d7b31a7038c6、cheap SHA256
FB9BD788AEAAA436A3C1F99EC3F6C03D84CF15BC77214082BF60ABC183F69B4D、
patch SHA256 F036A6E04814D791ADAC3C19007B6D05B166F2333AE7F3695BC67D98A1C3543F。

## 2026-10-15 Review

A/B/Validator追加0、Judge ACCEPT。独立cheap17/IPD6検証とsourcehash一致を確認。
callerfixのみをmath/reuseと独立commitする。

## Fix後の相関microbenchmark Baseline

IPD独立commit f12e9203be8bfda20b6075be9099db3f62318b81でfreeonly baselineを取得。
artifact target/ncc-reuse-evidence/baseline-f12e920/manifest.json、SHA256 d5cd1e7a88da22c6a87cf6a84a58a61fe631a1b1f6c3884de796b25c7ceb7267。
binary SHA256 77ae18fab7c337f82f75c6ad10ad9c940973e75718edaff8f1b297b057b4dab4。
6case×raw3000call samples、cold、p50/p95、phaseRSS/PID/UTCinterval、host/toolchain/ISA、
fixturebytes/harness/outputhashを保存。ISA probeは同hostのpulp::Arch::new()。

| shape/mode | p50 ms | p95 ms | cold ms |
| --- | --- | --- |
| band-ncc | 0.321200 | 0.642300 | 0.717300 |
| band-phat | 0.321800 | 0.589800 | 0.710800 |
| ipd-ncc | 0.320500 | 0.632900 | 0.733500 |
| ipd-phat | 0.319500 | 0.603800 | 0.687900 |
| large-ncc | 10.449900 | 13.042300 | 12.030800 |
| large-phat | 10.752800 | 16.246900 | 13.877900 |

一時harness/probeは自作ファイルだけ除去。製品依存追加なし。これは相関freecallの基準であり
2site実caller e2eのbaselineではない。後続slice3でcaller計測を別途確認し、速度改善はまだ主張しない。
phaseRSSはcurrent working set、peakはprocess lifetimeでありphasepeakではない。

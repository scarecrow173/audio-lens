---
id: "PLAN-034FQfhDbOOCKmh4QhnP51"
type: "plan"
status: "in-progress"
title: "NCC plan/workspaceを分離し2サイトを再利用へ移す"
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
    - "docs/specs/ncc-plan-workspace-reuse.md"
  implemented-by:
    - "../tasks/ipd-lag-search-fix.md"
    - "../tasks/ncc-plan-workspace-foundation.md"
    - "../tasks/ncc-hot-caller-reuse-verification.md"
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related: []
  refines: []
  refined-by: []
  derives-from:
    - "docs/specs/ncc-plan-workspace-reuse.md"
    - "docs/designs/ncc-plan-workspace-reuse.md"
  derived-by: []
  verifies: []
  verified-by:
    - "../test-specs/ncc-plan-workspace-reuse.md"
  references: []
  defers: []
  deferred-by: []
---

# NCC plan/workspaceを分離し2サイトを再利用へ移す

## Goal and Scope

approved spec/accepted ADR/approved designの所有境界・数値契約を実装する。
IPD独立fix→fix後baseline→math分離→2site移行→検証/測定の1stream。
FFT kernel交換、TLS pool、全caller移行、al-locate移行、新依存、manifest/schema変更はない。
今回designのP1は設計上resolvedであり、firstcold allocation証拠は実装で必ず確認する。

## File Map and Ownership

| パス | 所有する変更 |
| --- | --- |
| crates/al-analysis/src/cheap.rs | signed IPD検索窓/helper・回帰、次にblock外reuse |
| crates/al-analysis/src/band_gain.rs | windows反復外plan/ws、既存tie/polarity/guard維持、回帰/計測 |
| crates/al-math/src/fft.rs | RealFftPlan/Workspace、packing+large stage所有、同ISA共通stage primitive、既存RealFft委譲 |
| crates/al-math/src/correlation.rs | exactshape plan/ws、runtime NCC/PHAT、borrowed curve、freewrapper統合 |
| crates/al-math/tests/ncc_workspace.rs | ownership、shape、連続state、数値oracle、freshthread allocation/drop harness |
| crates/al-math/examples/bench_ncc_reuse.rs | 固定fixture cold/steady比較、phase/shape/alloc/p50/p95出力 |
| docs/test-specs/ncc-plan-workspace-reuse.md、docs/tasks/{ipd-lag-search-fix,ncc-plan-workspace-foundation,ncc-hot-caller-reuse-verification}.md | 下記同義test契約と3slices |
| docs/impl/irの対応3件と関連index | 実行したred/green/基準commit/計測/完成reviewの証拠 |
| target/ncc-reuse-evidence/ | ignored run artifacts（環境、fixture、baseline/post計測結果） |

既存 Fft::transform TLSは維持。legacyIR補修や#33は所有外で変更しない。
Cargo.toml/Cargo.lockは変更しない。benchmark/testは標準libraryと既存依存だけで実装する。

## Ordered Task Proposals

### Slice 1: ipd-lag-search-fix

1. cheap内に非周期wideband 2区間lag -3/+5の独立回帰を先に追加し、現実装drift0でredを保存する。
2. NCC合法clampを変えずfull-margin検索窓・signed origin helperを修正する。
3. signed helper、固定lag、max0、負設定の既存usize飽和、短入力、exactblock、第一channel pair境界を確認する。
4. targeted cheap回帰green→IPD fix/回帰/irを独立commit。plan APIをこのcommitに混ぜない。
5. fixture/benchmark harnessを用意し、このfix後commitをpinしてbaselineを取得する。
   harness追加は必要なら独立commit。bug修正によるworkload差をreuse効果と混同しない。

### Slice 2: ncc-plan-workspace-foundation (depends on Slice 1)

1. plan Send+Sync、shape/pow2不一致、borrowed output、既存oracle等価性とfreshthread allocation/dropのテストを作る。
2. immutable FFT係数とpacked scratch/large skew stageを分離しconstructorで全確保する。
3. StageRun本体を共通generic primitiveへ抽出し、explicit pathは外側dispatchのsimdを直接渡す。
   RealTransformの2方向はexplicit pathへ、legacy Fft TLS pathは既存dispatchを維持する。
4. CorrelationPlan/Workspaceを実装し、NCC/PHAT共通kernelにfreewrapperを統合する。
5. 下記matrixで数値/境界/firstcall/dropを検証、math変更と対応証拠を論理commitする。

### Slice 3: ncc-hot-caller-reuse-verification (depends on Slice 2)

1. band_gain windowsとIPD helperのcurve生成だけreuseへ移し、loop外所有・loop内borrow消費を確認する。
2. 2site既存挙動の回帰、loop内plan/scratch確保0、逐次/worker別ws結果一致を確認する。
3. 同fixture/toolchain/host/設定でbaselineとpostのcold/steady p50/p95/RSSを比較する。
4. targeted tests、workspace fmt/clippy、docs audit/diffcheck、独立完成reviewを行う。
5. 修正可能な非Blockerを直し、移行/測定/対応irをcommitしてGraph commit gateを確認する。

3slicesは依存順のみ、並列実装しない。独立reviewと必要な読取調査は並列可。

## Bundled Test-Spec Contract Proposal

この節はplan reviewに含める同義test-spec/task案であり、draft planから実ファイルを生成しない。
plan承認後に1test-specと3tasksを忠実に生成する。追加判断/scope変更は再review。

- Rule 1: IPDは合法full-margin窓のsignedlag変化を報告し、NCCclamp/pair/100ms契約を維持。
  Example: noiseの-3/+5→drift8、constant lag→0、max0、短/片channel不足、exactblock。
- Rule 2: same kernelのNCC/PHAT/freewrapperは独立oracleと既存許容誤差で一致し、極性/lag方向を保持。
  Example: positive/negative signal、silent、n1/2、oddlen、large、empty/short、clamp、max変化連続reuse。
- Rule 3: 共有immutablePlanとworker別exclusiveWorkspaceは逐次と同じ結果。
  shape不一致は処理前panic、借用curveの間にwsを再使用できない。
- Rule 4: explicit RealFFT/correlationはlargeもTLSを使わずconstructor全確保。
  freshthreadのwarmupなしfirstcall、反復callともalloc/realloc0、dropで所有領域解放。
- Rule 5: 2siteのloop内再構築/確保を除去し、実測値とfixture/hash/environmentを保存。
  未計測のspeedup・RSS効果を主張しない。#42判定はここで行わない。

## Verification Matrix and Commands

| 保証 | 実行/合格条件 |
| --- | --- |
| IPD red/green | `mise exec -- cargo test -p al-analysis ipd_lag`（独立fixtureのredを先に保存）、既存cheap回帰もgreen |
| FFT/NCC数値 | `mise exec -- cargo test -p al-math`、小shape独立oracle既存abs1e-10、large既存golden誤差を維持 |
| firstcold/drop | `mise exec -- cargo test -p al-math --test ncc_workspace -- --test-threads=1`。構築/計数器init後のfreshthread first forward/inverse/NCC/PHATにalloc/realloc0、workspace drop解放bytesを確認 |
| threshold | RealFFT N16384/32768/65536/262144、m8192/16384/32768/131072、large NCC/PHAT FFT N262144 |
| real callers | `mise exec -- cargo test -p al-analysis band_gain` / `ipd_lag`、既存reference/golden関連regression。windows/cheap owner内のignored計測testは10秒stereo noise @48kを使う |
| numerical sharing | Arc plan/scoped threads別ws、sequentialとcurve/peak一致。workspace不一致panicとstate連続reuse |
| quality | `mise exec -- cargo fmt --all -- --check`、`mise exec -- cargo clippy -p al-math -p al-analysis --all-targets -- -D warnings`、変更文書audit、`git diff --check` |

freshthreadでは入力/output/plan/ws/計数器を構築後、測定開始。assert/出力/他thread確保を除外。
allocation計測のdiagnostic instrumentationはproduction TLS cacheを作らない。
所有buffer bytes/deallocをRSS低下と混同しない。scalar/SIMD数値閾値を通すため緩和しない。

## Benchmark Recipe

`mise exec -- cargo run --release -p al-math --example bench_ncc_reuse` のハーネスを使用する。
noise seed0x52、band window4800/max240(signal5280)、IPD block4800/max192(signal5184)、
large template96000/signal100800（FFT262144）をNCC/PHAT双方で測る。
入力生成はtimed scope外、coldはplan+ws+firstcall、steadyは100call×30batchの各call durationを
3000点としてp50/p95集計する。baselineのowned freecall、post freecall、post reuseを区別する。
RSS/peakはWindows標準Get-Process等でphaseとprocess IDを紐付け採取し依存を追加しない。

2site benchmarkはowner内のignored testで同じ10秒stereo fixtureを実際windows/IPD処理へ渡し、
計測phaseとcurve構築回数を出力する。全cheap pipelineを含む計測なら他DSP費用も含むと明記。
同host/toolchain/ISA、baseline fix後commitとpost commit、fixture/config hash、run IDを保存。
他process干渉があればその限界も記録する。3x/10%を未承認gateとして追加しない。
正しさ/loop内確保0は必須、再現性能回帰は原因調査してspeedup主張をしない。
ベンチ長時間は60秒以内のprogressと非blocking yieldで監視する。

## Risks, Rollback and Handoff

hidden TLS stage見落とし、buffer初期化漏れ、shapeoverflow、借用期限、measurementノイズが主risk。
firstcold/threshold/drop・独立oracle・stage順の静的reviewと実測で防ぐ。
reuseだけrevertしてもIPD修正が残るcommit境界を維持する。破壊的Git操作は不要。
ユーザーはsubagent実作業委譲とreviewによる承認を指示済み、追加permission質問は不要。
既存dirty/他担当legacyIR/#33をrevertしない。sharedindex更新はこの担当だけが行う。

## Self Review

specの全8ACを上記task/rules/matrixへ対応。専用focusedchain1件、計測結果は未実施として扱う。
plan reviewには同義test-spec/task案を束ね、承認後の生成を新判断として再審査しない。

## 2026-10-15 Plan Review

A/B0、Validator追加0、pinnedhash一致/diffcheck0、Judge ACCEPT。
bundledtest-spec1件/task3件の上流同義内容も承認。動的実証は未実施。

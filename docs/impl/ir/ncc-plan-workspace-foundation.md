---
id: "IMPL-034FQVh1G9J5C9QK9oDZNe"
type: "impl"
status: "completed"
title: "NCC plan workspace foundation"
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
    - "docs/tasks/ncc-plan-workspace-foundation.md"
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

# NCC plan workspace foundation

## Summary

FFT係数とpacking/large stage scratchを明示所有へ分離。exactshape CorrelationPlan/Workspaceに
NCC/PHATの同一kernelを集約し、owned freewrapperを維持した。基盤Judge ACCEPT。
2hot caller移行・性能比較は未実施、deps/lock/他callerの変更なし。

## Context

approved #52 design/planのslice2。IPD独立fix f12e920の後、基準測定/graph記録69e5322からの差分。
RealFFTのhidden stage TLSはdesignレビューP1であり、explicit pathから除去する。

## Implementation

fft.rsは子worker所有、correlation.rs/test/今回docsは実行担当所有として並列実装。
全scratchはconstructorで確保、smallは既存transform、largeは同ISA run_stages/gather_permute。
Fft::transformのlegacyTLSは維持、RealFft/filterの必要crateSIMDwrapperはauthoritative処理へ委譲。

Correlation shape不一致は演算前panic、NCC/PHATは同じplanでruntime指定。
curveはws借用、padding/cumulative/norms/PHATenergyを毎回上書き。freewrapperは同じkernelのowned入口。

## Related Experiments

API導入red/greenと独立oracle/所有証明のみ。性能prototype比較はslice3へ。

## Validation

red: `mise exec -- cargo test -p al-math --test ncc_workspace --no-run` は
CorrelationPlan/Workspace、RealFftPlan/Workspace未定義E0432/exit1。新APIfeature redであり挙動redではない。
実装後、既存private scalar testの構造体callを新所有型へ同義適応（途中compile4errorを解消）。

- `mise exec -- cargo test -p al-math -- --nocapture`: unit33、correlation/filter5、FFT9、
  new workspace6、borrowcompilefaildoc1が成功。追加の独立drop1もtargeted実行で成功。
- `mise exec -- cargo clippy -p al-math --all-targets -- -D warnings`: exit0（追加drop後も確認）。
- `mise exec -- cargo fmt --all -- --check`: exit0。
- producer回帰: `cargo test -p al-analysis --lib` 99、`--test golden`19、
  `cargo test -p al-locate --test golden`16が成功。すべてmise経由。

direct滑動NCC、fullDFT PHATの小shape独立oracle、large FFT65536 NCC direct oracle。
従来scalar/SIMD deviationと許容誤差を維持。Arc共有plan/4worker別wsでNCC/PHAT一致。
empty/short/shape/mode/zeroenergy/polarity/max変化を確認、借用中reuse不能をcompilefailで確認。

freshthreadでinput/output/plan/wsとdiagnostic counterを構築し、warmupなしfirst forward/inverse/NCC/PHATと
100repeatのalloc/realloc=0を確認。計数器のconstTLSはtestだけで本番workspaceTLSではない。
RealFFT N1/2/16384/32768/65536/262144で閾値前後も確認。
N262144のworkspace4,195,328bytes、large correlation13,954,616bytesはconstructor所有で
plan存続中のworkspace dropが全bytesを解放。独立のdrop検証はplan4,194,384bytesと
workspace13,954,616bytesを別々に解放し、両者を混同しない。

artifact `target/ncc-reuse-evidence/foundation/` のmath-tests.log/ownership-drop.log/verification.jsonに
実数counts・sourcehash・検証条件を保存。baseline manifestは別artifactで保持。

## Risks

2site loopの確保/寿命・速度改善はまだ未実証。freewrapperは所有資源を毎回構築するので
postfree/baselinefree/reuseを別条件としてslice3で比較する。RSSとowned deallocationを混同しない。

## Follow-ups

基盤の独立review→論理commit→2siteだけ移行/allocatorcaller証拠→同fixture速度比較→完成review。

## 2026-10-15 Review

A/B/Validator追加0、独立workspace7pass/pinnedhash一致、Judge ACCEPT。
math基盤だけを独立commitし、actual2callerのfree-beforeを取得してから移行する。

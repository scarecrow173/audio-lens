---
id: "TSPEC-034FQrcfiATnKq0wNxx0kz"
type: "test-spec"
status: "approved"
title: "NCC 再利用と IPD signed lag の契約"
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
  implements: []
  implemented-by: []
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related: []
  refines: []
  refined-by: []
  derives-from:
    - "docs/designs/ncc-plan-workspace-reuse.md"
  derived-by: []
  verifies:
    - "docs/specs/ncc-plan-workspace-reuse.md"
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# NCC 再利用と IPD signed lag の契約

## Purpose

IPDの検索窓/原点とNCC/FFTの明示所有再利用が、正しいlag・数値・無確保契約を維持する。
上流が変われば見直す。2026-10-15 plan Judge ACCEPTの同義具体化。

## Feature

IPD signed lag 計測と共有plan/排他workspaceによる2site相関再利用。

## Rules

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


## Examples

planのfixture/matrixどおり、wideband -3/+5、empty/silent/threshold、firstcold、mode/shape切替、drop。

## Guarantees

独立oracleと既存誤差を維持。stage順/正規化/符号を変えず、firstcall/反復alloc0、所有領域解放。

## Non-goals

全caller/TLS全廃、kernel交換、依存追加、#42判断、未計測性能主張。

## Risk

共有実装比較だけでoracleを代替しない。hidden large-stage TLSとwarmupが見落としやすいため
freshthread構築後のfirstcall計数を必須とし、RSSとdeallocation証拠を区別する。

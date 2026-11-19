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

IPD の検索窓/原点と NCC/FFT の明示所有再利用が、正しい lag・数値・無確保契約を維持する。
上流が変われば見直す。2026-10-15 plan Judge ACCEPT の同義具体化。

## Feature

IPD signed lag 計測と共有 plan/排他 workspace による 2 site 相関再利用。

## Rules

- Rule 1: IPD は合法 full-margin 窓の signed lag 変化を報告し、NCC clamp/pair/100ms 契約を維持。
  Example: noise の -3/+5 → drift 8、constant lag → 0、max 0、短/片 channel 不足、exact block。
- Rule 2: same kernel の NCC/PHAT/free wrapper は独立 oracle と既存許容誤差で一致し、極性/lag 方向を保持。
  Example: positive/negative signal、silent、n1/2、odd len、large、empty/short、clamp、max 変化・連続 reuse。
- Rule 3: 共有 immutable plan と worker 別 exclusive workspace は逐次と同じ結果。
  shape 不一致は処理前 panic、借用 curve の間は ws を再使用できない。
- Rule 4: explicit RealFFT/correlation は large も TLS を使わず constructor で全確保。
  fresh thread の warmup なし first call、反復 call とも alloc/realloc 0、drop で所有領域解放。
- Rule 5: 2 site の loop 内再構築/確保を除去し、実測値と fixture/hash/environment を保存。
  未計測の speedup・RSS 効果を主張しない。#42 判定はここで行わない。


## Examples

plan の fixture/matrix どおり、wideband -3/+5、empty/silent/threshold、first cold、mode/shape 切替、drop。

## Guarantees

独立 oracle と既存誤差を維持。stage 順/正規化/符号を変えず、first call/反復 alloc 0、所有領域解放。

## Non-goals

全 caller/TLS 全廃、kernel 交換、依存追加、#42 判断、未計測性能主張。

## Risk

共有実装比較だけで oracle を代替しない。hidden large-stage TLS と warmup が見落としやすいため
fresh thread 構築後の first call 計数を必須とし、RSS と deallocation 証拠を区別する。

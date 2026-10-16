---
id: "DESIGN-034H5ozhtFnMNl0ZABihBS"
type: "design"
status: "draft"
title: "Valid FFT NCC plan 設計 (issue #71 P1)"
created: "2026-10-16"
updated: "2026-10-17"
owners: []
relations:
  source:
    - "../specs/valid-fft-ncc-plan-issue-71-p1.md"
    - "../adr/ncc-valid-fft-plan-gcc-phat-plan.md"
  changes:
    added: []
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "../specs/valid-fft-ncc-plan-issue-71-p1.md"
    - "../adr/ncc-valid-fft-plan-gcc-phat-plan.md"
  implemented-by: []
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "../discovery/issue-40-locate-pair-plan-cache.md"
    - "./streaming-bandpass-issue-71-p2.md"
  refines: []
  refined-by: []
  derives-from:
    - "../specs/valid-fft-ncc-plan-issue-71-p1.md"
    - "../adr/ncc-valid-fft-plan-gcc-phat-plan.md"
  derived-by: []
  verifies: []
  verified-by: []
  references:
    - "../research/issue-71-band-correlation/algorithm-research-report.md"
  defers: []
  deferred-by: []
---

# Valid FFT NCC plan 設計 (issue #71 P1)

## Context

`CorrelationPlan` は FFT 長 `n = nextpow2(S+L−1)` を持ち、NCC(`ncc`)
と GCC-PHAT(`gcc_phat`)を同一 plan で実行する。NCC は有効ラグ
`lag ∈ [0, S−L]`(出力 `real[..=max_lag]`、`max_lag` は `S−L` に
clamp)のみを返すため、`n = nextpow2(S)` で返却範囲の値は数学的に
同一になる:

- 線形相関 `c[k]` の index 域は `k ∈ [−(L−1), S−1]`。長さ `n` の
  巡回 IFFT 出力 `k` は `c[k] + c[k−n] + c[k+n]` となる。
- `k ∈ [0, S−L]` では `k+n ≥ n ≥ S` より `c[k+n]=0`、また
  `k−n ≤ S−L−n < −(L−1)`(`n ≥ S` なら `n−L+1 > S−L ≥ k`)より
  `c[k−n]=0`。よって返却域は wraparound 無汚染。
- 汚染は `k > S−L` の領域にのみ現れ、返却されない。
- `n = nextpow2(S) ≥ S` なので signal 読込みバッファ(`real` サイズ
  `n`)も不足しない。

GCC-PHAT は bin ごとの位相正規化が FFT 長依存(白色化した系列は時間
領域で帯域制限されないため巡回エイリアスが全 index に効く)なので
短縮不可 — 従来 FFT 長を維持する。

## Scope

### In Scope

- `al-math/src/correlation.rs`: `CorrelationPlan` に mode を追加し、
  NCC-valid コンストラクタを追加。`gcc_phat` は valid plan で panic。
- `al-locate/src/pool.rs`: pool キーに mode を追加。
- `al-math` フリー関数 `ncc`: valid plan 化(`gcc_phat` は不変)。
- `al-analysis/src/band_gain.rs`、`al-analysis/src/cheap.rs` の
  NCC 専用 plan 呼出を valid コンストラクタへ切替。

### Out of Scope

- `Correlation::run`・`CorrelationWorkspace`・FFT/SIMD カーネルの変更
- `OverlapCorrPlan`(local_lag、別契約)
- GCC-PHAT の FFT 長変更
- P2/P3 の構造最適化

## Components and Boundaries

- `CorrelationPlan`(`al-math`): shape + FFT 長の所有。mode を
  `enum PlanMode { Full, NccValid }`(private)で保持。
  - `CorrelationPlan::new(template_len, signal_len)`: 従来通り
    `n = nextpow2(S+L−1)`、mode=Full。PHAT 呼出可能な唯一の plan。
  - `CorrelationPlan::new_ncc(template_len, signal_len)`(名称は実装時に
    `new_ncc`/`new_valid` から確定): `n = nextpow2(S)`、
    mode=NccValid。`S==0 || S<L` は従来通り `n=0`。
  - `gcc_phat`: `mode != Full` で panic(契約違反の早期検出)。
  - `ncc`: 両 mode で使用可能(Full plan での NCC も従来通り正しい)。
- `CorrelationWorkspace`: 無変更(`n` 由来のサイズは plan から取得済み、
  valid plan では bins/real が小さくなるだけ)。
- `CorrelationPool`(`al-locate`): キーを
  `(template_len, signal_len, PoolMode)` に拡張。
  `PoolMode::Ncc → new_ncc`、`PoolMode::Phat → new`。
  `MAX_SHAPES` による bounded 挙動は維持。
- フリー関数(`al-math`): `ncc` は `new_ncc` 経由、`gcc_phat` は
  `new` 経由。pool 経由と plan 一致で結果がビット一致する。

## Data and Control Flow

1. 呼出側は用途で plan を生成: NCC 専用なら `new_ncc`、PHAT を含むなら
   `new`。
2. `CorrelationWorkspace::new(&plan)`: plan の `n` から real/bins を確保
   (valid plan では半分以下のメモリ)。
3. `plan.ncc(...)`: 従来通り `run(..., phat=false)`。FFT 長だけが小さい。
4. `plan.gcc_phat(...)`: mode 検査 → `run(..., phat=true)`(従来 FFT 長)。

## Non-Functional Constraints

- 数値契約: 返却域 `[0, max_lag]` の値は scalar↔dispatch の既存許容
  基準で従来 plan と一致。FFT 長が異なるため厳密なビット一致ではなく
  丸め誤差範囲の一致(両経路とも同じ valid plan 同士は一致)。
- `nextpow2(S) == nextpow2(S+L−1)` の形状(短 template、48kHz
  band_gain 等)では同一 FFT 長となり性能退行なし。
- メモリ: valid plan の workspace は `n` 由来バッファが縮小
  (band_ncc では real 64MB→32MB、bins 類も半減)。

## Risks and Trade-offs

- **リスク**: valid plan で `gcc_phat` を誤用しても FFT 自体は動く
  (出力が壊れる)。→ mode フィールドで実行時 panic 検出 + テスト。
- **リスク**: `dsp::ncc`(フリー関数)と pool.ncc の plan が乖離して
  結果の丸めがずれる。→ 両者とも `new_ncc` を使い一致を維持
  (pool テストが `assert_eq!` で検証)。
- **トレードオフ**: ncc/phat 同一 shape で plan が 2 本になり pool の
  plan 構築コストが倍増する場合があるが、band_ncc/full_ncc/gcc_phat は
  別メソッドとして実行されるため 1 メソッドあたり 1 plan で変わらない。
- **トレードオフ**: `new` を残すため、新規 NCC 呼出が誤って Full plan
  を選ぶ余地が残る。→ `new` の doc に「PHAT を含む場合のみ」旨を明記。

## Deferred Design Concerns

- P2 Streaming Bandpass: `band_gain::prepare_with_bands` の帯域波形を
  逐次生成へ。`InputCache` の責務分割が前提 | P1 完了後に独立設計。
- P3 適応 dispatch(direct/Valid FFT/Blocked FFT): mode に
  第三選択肢を足す可能性 | 形状ベンチ後に設計。
- `CorrelationPlan::new` の rename(責務明確化): pre-release なので
  後方互換不要だが、本変更の差分を小さく保つため今回は実施しない。

## Verification Notes

- [ ] `cargo test -p al-math`: valid/Full の NCC 全ラグ一致(境界:
      L=1、L=S、K=1、max_lag<S−L、非 pow2 各種)、`new_ncc`+
      `gcc_phat` が panic、scalar↔dispatch 同等
- [ ] `cargo test -p al-locate`: pool で同一 shape の ncc/phat が別
      plan(builds=2)、MAX_SHAPES 境界不変、`dsp` 一致
- [ ] `cargo test -p al-analysis`: band_gain / ipd 出力不変の既存回帰
- [ ] 実サンプルで band_ncc / band_gain busy 比較記録
- [ ] `cargo clippy --workspace` / `audit_docs` clean

---
id: "DESIGN-034HiHXIY2zsfaMpUtzj8x"
type: "design"
status: "draft"
title: "shape-adaptive NCC dispatch (issue #71 P3)"
created: "2026-10-17"
updated: "2026-10-17"
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
  implements: []
  implemented-by: []
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "./streaming-bandpass-issue-71-p2.md"
  refines: []
  refined-by: []
  derives-from:
    - "../specs/adaptive-ncc-dispatch-issue-71-p3.md"
    - "../adr/ncc-plan-time-dispatch-issue-71-p3.md"
  derived-by: []
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# shape-adaptive NCC dispatch (issue #71 P3)

## Context

`CorrelationPlan` は P1 で `PlanMode::{Full, NccValid}` に分離済み。
全 NCC 呼出は `max_lag = signal_len − template_len` であり、
`K = S − L` は plan 構築引数から一意に決まる。P3 はここに第 3 の
mode `NccDirect` を追加し、分子の計算方式だけを plan 内部で
切り替える。正規化パイプ(prefix-sum → `normalize`)は全経路で
共通のまま。

## Scope

### In Scope

- `PlanMode::NccDirect` 追加と `new_ncc` のコストモデル選択
- `Correlation::run` の direct 分岐(分子の axpy 蓄積)
- `CorrelationWorkspace::new` の mode 別バッファ確保
- mode 選択 pin・経路間一致・scalar↔dispatch のテスト

### Out of Scope

- 呼出側・pool API の変更、Blocked FFT、#76 の探索構造変更

## Components and Boundaries

- `PlanMode`: `Full`(GCC-PHAT 対応・全ラグ FFT)、`NccValid`
  (有効ラグ NCC・`nextpow2(S)`)、`NccDirect`(有効ラグ NCC・
  直接法)の 3 値。`ncc` は NccValid/NccDirect を受け付け、
  `gcc_phat` は Full のみ(現行 assert がそのまま direct を拒否)
- `CorrelationPlan::new_ncc`: shape → mode 選択の唯一の主体
- `CorrelationWorkspace::new`: `NccDirect` plan では FFT 関連
  バッファ(`fft`/`real`/`a_re`/`a_im`/`b_re`/`b_im`)を確保
  しない。prefix-sum 用の `template_squares`/`signal_squares`/
  `cumulative`/`norms` と分子蓄積用バッファは両経路で使う
- `Correlation::run`: mode で分子経路を分岐し、以降は共通

## Data and Control Flow

- `new_ncc(L, S)` → `n = nextpow2(S)`、`K = S−L` →
  `L·(K+1) ≤ C·n·log2(n)` なら `NccDirect`、else `NccValid`
  (C はベンチ校正定数 ≈ 8、実装時に最終値をコードへ)
- direct 経路の `run`:
  1. shape assert(現行と同一)
  2. `template_energy` 計算・0 なら全 0 で early return(現行と同一)
  3. 分子: `acc[k] += template[i]·signal[k+i]`(i 外ループ、
     `acc` は K+1 要素の workspace バッファを再利用)
  4. `signal_squares`→`cumulative`→`norms`→`normalize` は
     現行コードと同一(`real` ではなく `acc`/`norms` バッファを
     使うが規則は同一)
  5. 戻り `&acc[..=max_lag]`(clamp は現行と同一)
- `Bandpass` 系・pool・dsp フリー関数は無変更

## Non-Functional Constraints

- 出力は Valid FFT plan と全有効ラグで一致(誤差 ~1e-13 未満、
  実測 ~1e-15)。argmax/tie/順位の回帰テストを維持
- `pulp` は al-math 内部のまま。direct カーネルも `WithSimd` で
  scalar↔dispatch 同等を保つ
- direct workspace は `real`/`a_*`/`b_*` なし(形状あたり
  ~100MB 級の節約が pool に効く)

## Risks and Trade-offs

- 閾値誤分類 → 性能のみ劣化(正しさ不変)。校正定数は保守側に
  取り、実形状での pin テストで回帰検出
- `acc` の蓄積順は i 昇順 — FFT 畳み込みとの和順差が誤差源だが
  実測 ~1e-15 で契約内
- K が大きい direct 形状(理論上 acc>L2)はモデルが FFT 側へ
  振るので、cache spill の悪化は閾値で回避される

## Deferred Design Concerns

- 別環境での閾値再校正 | 誤分類の実害が報告された時点 |
  定数を現行値から大きく外す必要が出た場合に design 更新
- #76 で narrow 形状が direct に落ちる場合の効果検証は #76 側

## Verification Notes

- [ ] 6 代表形状の mode 選択がベンチ最速側に一致
- [ ] direct⇔NccValid 全有効ラグ一致・scalar↔dispatch <1e-10
- [ ] direct plan で `gcc_phat` が panic(現行契約維持)
- [ ] workspace が direct plan で FFT バッファを確保しないこと

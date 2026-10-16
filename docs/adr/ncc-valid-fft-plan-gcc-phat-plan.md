---
id: "ADR-034H5mcOUBGtY1QeJj877L"
type: "adr"
status: "proposed"
title: "NCC valid FFT plan と GCC-PHAT plan の分離"
created: "2026-10-16"
updated: "2026-10-17"
owners: []
relations:
  source:
    - "../specs/valid-fft-ncc-plan-issue-71-p1.md"
    - "https://github.com/scarecrow173/audio-lens/issues/71"
  changes:
    added: []
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "../specs/valid-fft-ncc-plan-issue-71-p1.md"
  implemented-by: []
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "../discovery/issue-40-locate-pair-plan-cache.md"
    - "../discovery/issue-53-local-lag-fft.md"
    - "./bandfilter-inputcache.md"
  refines: []
  refined-by: []
  derives-from:
    - "../specs/valid-fft-ncc-plan-issue-71-p1.md"
  derived-by: []
  verifies: []
  verified-by: []
  references:
    - "../research/issue-71-band-correlation/algorithm-research-report.md"
  defers: []
  deferred-by: []
metadata:
  adr:
    decision-makers: []
    consulted: []
    informed: []
---

# NCC valid FFT plan と GCC-PHAT plan の分離

## Context and Problem Statement

`CorrelationPlan` は現在 `nextpow2(S+L−1)` の FFT 長を持ち、NCC と
GCC-PHAT が同一 plan(同一 shape なら同一 workspace)を共有する。
有効ラグ `0..=S−L` だけが必要な NCC では `nextpow2(S)` の FFT 長で
数学的に同一結果が得られる(wraparound 汚染は index `> S−L` にのみ
発生し返却範囲外)が、GCC-PHAT は bin ごとの位相正規化が FFT 長に
依存するため短縮できない。FFT 長の短縮を入れるには NCC/PHAT の
plan 契約を分離する必要がある。

## Decision Drivers

* 実測ボトルネック: band_ncc(全長 NCC)8.4M→4.2M、band_gain(窓 NCC)
  44.1kHz で 16K→8K と、支配的カーネル時間の半減余地
* 推定量契約の維持: PHAT の出力は FFT 長依存なので変更不可
* plan/workspace の誤共有防止: 異なる FFT 長を同一 shape キーで共有
  すると片方の計算が壊れる
* P3(形状適応 dispatch)の基盤: FFT 長選択を plan 責務として
  局所化しておきたい

## Considered Options

* **Option 1: 同一型 + mode フラグ** — `CorrelationPlan` に
  FFT 長選択(mode)フィールドを持たせ、`new`(従来)と
  `new_ncc_valid` を提供。`gcc_phat` は valid plan で呼ぶと panic。
* **Option 2: 別型 `NccPlan` / `PhatPlan` へ型分離** — NCC 専用型を
  新設し、コンパイル時に誤用を防ぐ。
* **Option 3: mode を実行時引数へ** — plan は従来通り、`ncc`/`gcc_phat`
  呼出側が FFT 長を渡す。

## Decision Outcome

Chosen option: "Option 1: 同一型 + mode フラグ"(コンストラクタで
FFT 長を確定、pool キーに mode を含める), because NCC と PHAT は
FFT 実行・正規化・workspace 構造の大半を共有しており、実行カーネル
`Correlation::run` を変更せず plan 生成時の FFT 長選択だけで差分を
完結できる。別型化は共有ロジックの複製か新しい抽象化を要し、この
変更規模に対して過剰。実行時引数化は plan の責務(長さ選択)を
呼出側へ漏らし、workspace が plan と不整合になるリスクがある。

### Consequences

* Good, because `Correlation::run`・workspace・SIMD カーネルは無変更で、
  FFT 長だけが plan 生成時に決まる。
* Good, because pool キー (template_len, signal_len, mode) で誤共有が
  構造的に防げる。
* Good, because `nextpow2(S) == nextpow2(S+L−1)` の形状では同一 plan と
  等価になり、形状適応の分岐不要で常に安全側。
* Bad, because `gcc_phat` の誤用(valid plan での呼出)はコンパイル
  時ではなく実行時 panic での検出になる。

### Confirmation

* valid plan での `gcc_phat` 呼出が panic するテスト
* valid/従来 plan の NCC 曲線が全有効ラグで一致する回帰テスト
* `dsp`/`pool`/`band_gain` 各経路で plan 選択が意図通りであることの
  テスト

## Implementation Plan

* Affected paths: `crates/al-math/src/correlation.rs`(plan 生成 + mode)、
  `crates/al-locate/src/pool.rs`(キー拡張)、`crates/al-locate/src/dsp.rs`
  (フリー関数の plan 選択)、`crates/al-analysis/src/band_gain.rs`
  (plan 切替)
* Patterns to follow: `CorrelationPlan::new` による shape 確定型の
  plan、`CorrelationWorkspace::new(&plan)` による plan 依存 workspace、
  pool の形状キー reuse
* Patterns to avoid: FFT 実行・正規化・workspace の複製、呼出側への
  FFT 長露出、PHAT plan の短縮
* Dependencies or configuration: なし

## Verification

* [ ] `cargo test -p al-math correlation` — valid/従来 plan の全ラグ一致、
      scalar↔dispatch 同等
* [ ] `cargo test -p al-locate` — pool mode 分離・ncc/phat 別 plan
* [ ] `cargo test -p al-analysis` — band_gain 出力不変
* [ ] 実サンプル run で band_ncc / band_gain busy 改善を記録
* [ ] `cargo clippy` / `audit_docs` clean

## Pros and Cons of the Options

### Option 1: 同一型 + mode フラグ

* Good, because `Correlation::run`・workspace・SIMD 部分を変更せず、
  plan 生成の FFT 長選択のみの差分で済む。
* Good, because plan が長さ選択の責務を持ち続ける。
* Bad, because PHAT 誤用が実行時検出になる。

### Option 2: 別型分離

* Good, because NCC-only が型で保証される。
* Bad, because FFT 実行・正規化・workspace の共有部分を複製するか
  新しい共通抽象が必要になり、変更規模に見合わない。

### Option 3: 実行時引数

* Good, because plan 型を変えない。
* Bad, because plan(FFT 長)と workspace(バッファ長)の整合責任が
  呼出側へ漏れ、形状ミスマッチのリスクを生む。

## More Information

* 数学的根拠・OSS 先例(STUMPY 型 valid FFT、GCC-PHAT の FFT 長依存):
  `docs/research/issue-71-band-correlation/algorithm-research-report.md`
* 確定方針・計測内訳:
  `docs/discovery/reference-analysis-analyze-pair-band-gain.md`

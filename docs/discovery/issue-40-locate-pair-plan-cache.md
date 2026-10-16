---
id: "DISC-034GMstpzt7NRBVyaC81KK"
type: "discovery"
status: "active"
title: "Issue 40 locate pair 間の相関 plan 再利用と共有 cache 監査"
created: "2026-10-16"
updated: "2026-10-16"
owners: []
relations:
  source:
    - "../../crates/al-locate/src/stage.rs"
    - "../../crates/al-locate/src/methods.rs"
    - "../../crates/al-locate/src/prepared.rs"
    - "../../crates/al-math/src/correlation.rs"
    - "remaining-issues-roadmap.md"
    - "https://github.com/scarecrow173/audio-lens/issues/40"
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
    - "./ncc-plan-workspace-reuse.md"
    - "./fft-rebaseline-post-plan-workspace.md"
    - "./locate-consensus-argmax-ambiguity-contract.md"
    - "./reference-analysis-analyze-pair-band-gain.md"
  refines: []
  refined-by: []
  derives-from: []
  derived-by:
    - "../specs/locate-plan-cache.md"
    - "../adr/locate-plan-run-pool.md"
    - "../designs/locate-plan.md"
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# Issue 40 locate pair 間の相関 plan 再利用と共有 cache 監査

## Exploration Goal

Issue #40「coarse-to-fine と帯域 cache の残件」の残存作業を確定する。
PR #44 で間引き coarse-to-fine と入力単位の bandpass cache は導入済み
であり、「pair 間の再利用は後続対応」と明記されていた。現 HEAD で
何が残っているかをコード上の証跡で固定し、実装スコープを決める。

確定決定(2026-10-15、roadmap 記載): スコープは **監査計測 +
NCC plan 再利用** に限定。IIR/SOS 帯域化は位相応答が変わるため
別 Issue に棚上げ。

## Key Issues and Assumptions

### 確認済み(実装)

- `InputCache`/`PreparedChannel`(`prepared.rs`)が帯域フィルタ済み
  信号と coarse 間引き信号を `Arc` で入力単位共有する。
  `stage.rs` では capture は全 source 間で、source は全 capture
  間で再利用される。`release(capture_id, "original")` が capture
  ループ末尾で呼ばれる
- `propose_coarse_to_fine`(`methods.rs:58`)は coarse 信号で NCC →
  候補をフルレート座標に換算し、`refine_coarse` が近傍 ±radius を
  全探索曲線で精密化する。全探索 corr は `locate_coarse_to_fine`
  で1回だけ計算され提案・精密化・最終証拠で共用される
- `dsp::ncc`/`dsp::gcc_phat`(`dsp.rs` re-export)は呼出しごとに
  `CorrelationPlan::new` + `CorrelationWorkspace::new` を構築する
  (al-math `correlation.rs:143` の `ncc` フリー関数経路)。
  plan は FFT twiddle 構築(4M 級で ~35ms、#42 ベンチ)、
  workspace は複数の O(n) バッファを毎回確保する

### 残存する重複(計測対象)

- 1 channel-pair あたりの相関呼出:
  `propose_coarse` 内 coarse NCC 1回 + FullNcc 全探索 1回 +
  BandNcc 帯域ごと 1回(アクティブ帯域数、最大6)+
  GccPhat 1回 ≈ **8-9 回**。SpectralEnvelope は `spectral_envelope`
  + `corr0` 経路で CorrelationPlan 非使用
- 各呼出が同じ (template_len, signal_len) 形状でも plan/workspace
  を再構築する。50s×50s pair は FFT size ~8M で plan 構築+workspace
  確保だけで毎回 ~50-150ms 級の見込み(#42 の plan 計測から外挿)。
  さらにフリー関数 `ncc()`/`gcc_phat()` は返却のための
  `.to_vec()` で curve 全体(full-rate で ~2.4M samples ≈ 19MB)を
  毎回追加確保する — plan API 経由の借用でこのコピーも消せる
- **メソッド実行は既にワーカー並列**(`stage.rs::run_pool`):
  `available_parallelism` までのワーカーが `(method, channel-pair)`
  タスクを steal する。各ワーカーが `default_methods()` を持ち、
  メソッドは `Send` ではないため証拠のみスレッド境界を越える。
  plan pool は stage グローバルではなく **ワーカー所有** が必須
- `InputCache` の実効性(hit/miss・release の実解放・ピーク RSS)
  は計測証跡がない — 設計上の再利用と実測の一致を検証する
- coarse-to-fine の境界(座標換算の丸め、radius と分解能差の関係、
  boundary-candidate マーク)は検証証跡がない

## Alternatives and Comparison

| Option | Pros | Cons | Lean |
| --- | --- | --- | --- |
| A 形状キーの plan/workspace プールを run_pool ワーカー単位で共有 | 同一 shape の全呼出で plan 構築・workspace 確保・`.to_vec()` を amortize。#52 の実績パターンを再利用。ワーカー逐次実行のため排他借用契約と整合 | 形状の多様性で pool が膨らむ可能性(bound/LRU 要検討)。`LocateMethod` へのコンテキスト伝播が必要 | yes |
| B `dsp::ncc` フリー関数内に暗黙 TLS cache | 呼出側変更ゼロ | #52 で TLS cache は棄却済み(暗黙共有・寿命不明瞭)。一貫性を損なう | no |
| C メソッドごとの手動 plan 渡し回し | 局所的 | 形状が pair ごとに変わるため呼出側での事前 plan 化が困難。cache が自然 | maybe |
| D IIR/SOS 帯域化などアルゴリズム置換 | 計算量自体の削減余地 | 位相/過渡応答が変わり golden 再基準化が必要。確定決定で棚上げ | no |

## Tentative Conclusions and Hypotheses

- **hypothesis**: `CorrelationPlan`/`CorrelationWorkspace` を
  `(template_len, signal_len)` キーの pool を `run_pool` の
  ワーカーコンテキストとして持たせると、pair 内の ~8-9 回 +
  pair 間の同形状呼出で plan 構築が 1 回/shape/worker に
  amortize される。帯域フィルタは長さを維持するため BandNcc の
  帯域別呼出も full-rate と同一 shape に乗り、同一 plan で
  NCC/PHAT 両方を実行できる(#52 の分離設計により plan は
  手法非依存)
- **hypothesis**: `InputCache` の hit 率は設計上高い(入力×帯域
  1回計算)が、実測証跡を取る。release の実効性とピーク RSS を
  併せて記録する
- **confirmed**: 方向は「監査計測 + plan 再利用」、IIR は除外
  (roadmap 確定決定 2026-10-15)

## Open Questions

- pool の API 形態: `LocateMethod::locate`/`locate_prepared` の
  シグネチャにコンテキストを追加するか、`run_pool` の work
  クロージャにワーカー固有コンテキストを渡す形に拡張するか
- pool の容量規約: shape が入力長に依存するため、長さの異なる
  入力が連続すると pool が膨らむ。上限数・per-pair クリア・
  LRU のどれで bound するか
- workspace 排他契約との整合: pool 内の workspace を `&mut` で
  取り出して曲線借用を返す間、次の呼出を待たせる借用設計
  (ワーカー内逐次なので実害なし)

## Sources and Relations

- `crates/al-locate/src/stage.rs`(InputCache 利用・pair ループ・
  release 呼出)
- `crates/al-locate/src/methods.rs`(propose_coarse_to_fine、
  FullNcc/BandNcc/GccPhat の dsp::ncc 呼出)
- `crates/al-locate/src/prepared.rs`(InputCache/PreparedChannel/
  FilterKey)
- `crates/al-math/src/correlation.rs`(CorrelationPlan/
  Workspace、`ncc` フリー関数の plan 再構築経路)
- `docs/discovery/ncc-plan-workspace-reuse.md`(#52 の plan/
  workspace 分離設計と実測)
- `docs/discovery/fft-rebaseline-post-plan-workspace.md`(#42 の
  plan 構築コスト計測)

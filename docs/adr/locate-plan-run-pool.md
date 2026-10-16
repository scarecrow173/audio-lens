---
id: "ADR-034GMAytptWAE2hKlPKyC5"
type: "adr"
status: "accepted"
title: "locate 相関 plan 形状プールを run_pool ワーカー単位で所有"
created: "2026-10-16"
updated: "2026-10-16"
owners: []
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
    - "../specs/locate-plan-cache.md"
  implemented-by:
    - "../designs/locate-plan.md"
  depends-on:
    - "./ncc-plan-workspace-reuse.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "./al-math-api-fft-split-re-im.md"
  refines: []
  refined-by: []
  derives-from:
    - "../discovery/issue-40-locate-pair-plan-cache.md"
  derived-by: []
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
metadata:
  adr:
    decision-makers: []
    consulted: []
    informed: []
---

# locate 相関 plan 形状プールを run_pool ワーカー単位で所有

## Context and Problem Statement

locate ステージのメソッド実行は `run_pool` でワーカー並列され、
各 `(method, channel-pair)` タスクが `dsp::ncc`/`dsp::gcc_phat`
フリー関数を呼ぶ。フリー関数は呼出ごとに `CorrelationPlan::new`
(FFT twiddle 構築)+ `CorrelationWorkspace::new`(O(n) 複数
バッファ)+ 返却用 `.to_vec()`(full-rate で ~19MB)を行う。
同一 (template_len, signal_len) 形状の呼出が 1 pair あたり
~8-9 回・pair 全体で繰り返されるにもかかわらず、毎回 plan を
構築し直している(#52 で plan 分離 API は確立済みだが locate
メイン経路は未移行)。

## Decision Drivers

* plan 構築(twiddle 生成)と workspace 確保の amortize:
  FFT size ~8M の入力で呼出あたり ~50-150ms 級の固定費が削減対象
* `.to_vec()` curve コピーの除去: full-rate ~2.4M samples ≈ 19MB/
  呼出のアロケーション削減
* 所有権の明瞭性: `CorrelationWorkspace` は排他借用契約を持つため、
  共有プールは借用規則と衝突する。run_pool のワーカーはタスクを
  逐次実行するため、ワーカー所有なら排他契約と自然に整合する
* #52 で棄却された暗黙 TLS cache を繰り返さない
* 将来の stage 並列化(#54)と矛盾しない所有権境界

## Considered Options

* ワーカー単位の形状プール(run_pool にワーカーコンテキストを追加)
* stage グローバルな共有プール(Mutex 等でスレッド安全化)
* `dsp::ncc` 内の暗黙 TLS cache
* メソッド呼出側で事前に plan を構築して渡し回す

## Decision Outcome

Chosen option: "ワーカー単位の形状プール"。

`run_pool` の work クロージャにワーカー固有コンテキストを渡す形に
拡張し、各ワーカーが `(template_len, signal_len)` キーの
`HashMap<shape, (CorrelationPlan, CorrelationWorkspace)>` を所有する。
`dsp::ncc`/`dsp::gcc_phat` を呼ぶメソッド経路には pool への参照を
コンテキストとして伝播し、`plan.ncc(&mut ws, ..)` の借用曲線を
`evidence_from_corr`/`refine_coarse` に直接渡す(`.to_vec()` なし)。
pool は上限形状数で bound し、超過時は最古または最多使用キーを
evict する(具体規約は実装計測で決定)。

### Consequences

* Good, because 同一形状の全呼出で plan 構築・workspace 確保・
  curve コピーが 1 回/worker/shape に amortize される。帯域フィルタは
  長さを維持するため BandNcc の帯域呼出が full-rate と同一 shape に
  乗り、1 plan で NCC/PHAT 両方が実行できる
* Good, because ワーカー所有により `Send`/`Sync` を新たに要求せず、
  `CorrelationWorkspace` の排他借用契約と衝突しない
* Good, because pool がワーカー寿命に閉じるため、#54 でワーカー
  ライフサイクルを再設計する際の境界がそのまま使える
* Bad, because ワーカー数 × shape 数分の plan/workspace が同時に
  生存する(例: 8 workers × 2 shapes × ~128MB workspace = ~2GB
  上限)。bound 規約で管理する
* Bad, because `LocateMethod` トレイトの呼出シグネチャに
  コンテキスト引数が増える(既存テストの更新が必要)

### Confirmation

* 同一形状の連続呼出で `RealFftPlan::new` が 1 回のみ呼ばれることを
  計数テストで確認
* pool 経由の出力とフリー関数出力が一致(既存テスト + 比較テスト)
* 既存 locate golden/合議テストが不変で通る

## Implementation Plan

* Affected paths: `crates/al-locate/src/stage.rs`(run_pool)、
  `crates/al-locate/src/methods.rs`(メソッド呼出経路)、
  `crates/al-locate/src/dsp.rs`(pool 型または re-export)、
  `crates/al-locate/src/prepared.rs`(InputCache 計数)
* Patterns to follow: #52 の plan/workspace 分離
  (`crates/al-math/src/correlation.rs`)、`band_gain::windows` の
  plan 利用パターン
* Patterns to avoid: 暗黙 TLS(#52 棄却)、Mutex 共有 workspace
  (借用契約と衝突)、`dsp::ncc` フリー関数への内部 cache 追加
* Dependencies or configuration: なし(既存 `al-math` API のみ)

## Verification

* [ ] `cargo test -p al-locate` で pool 経路と監査テストが pass
* [ ] `cargo test --workspace` 全 pass
* [ ] `cargo clippy --workspace --all-targets -D warnings` clean
* [ ] locate ベンチまたはサンプル実行で plan 構築回数の削減を記録

## Pros and Cons of the Options

### ワーカー単位の形状プール

* Good, because 排他借用契約・並列構造・将来の並列化設計と整合
* Good, because `.to_vec()` コピーも同時に消せる
* Bad, because `LocateMethod` シグネチャ変更が波及する
* Bad, because pool bound 規約の設計が必要

### stage グローバル共有プール(Mutex)

* Good, because ワーカーコンテキストの配管が不要
* Bad, because `CorrelationWorkspace` は排他借用が前提で、
  Mutex 内の workspace をロック中に借用して返す設計が borrowck と
  相性が悪い(返却曲線が workspace を借用するためロック保持中に
  次の呼出をブロックするか、曲線を `.to_vec()` して利点を失う)
* Bad, because `CorrelationPlan`/`RealFftPlan` の `Sync` 性に依存し、
  内部に非 Sync な状態が将来入ると破綻する

### `dsp::ncc` 内の暗黙 TLS cache

* Good, because 呼出側変更不要
* Bad, because #52 で明示的に棄却(暗黙共有・寿命不明瞭・
  テスト間の状態漏洩)

### メソッド呼出側で事前 plan 構築

* Good, because トレイト変更不要
* Bad, because 呼出側は実行時まで shape を知らず、事前構築するなら
  結局 shape キーの cache が必要 — 手段が違うだけで本質は同一

## More Information

* `docs/discovery/issue-40-locate-pair-plan-cache.md`
* `docs/specs/locate-plan-cache.md`
* `docs/adr/ncc-plan-workspace-reuse.md`(#52 plan/workspace 分離)
* `docs/discovery/fft-rebaseline-post-plan-workspace.md`(#42 plan
  構築コスト計測)
* `crates/al-locate/src/stage.rs::run_pool`(ワーカー並列の実態)

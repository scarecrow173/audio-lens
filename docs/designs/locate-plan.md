---
id: "DESIGN-034GMCyaWWhAaXk0LVJK4j"
type: "design"
status: "approved"
title: "locate 相関 plan ワーカープール"
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
    - "../adr/locate-plan-run-pool.md"
  implemented-by: []
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related: []
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
---

# locate 相関 plan ワーカープール

## Context

locate ステージ内で相関を呼ぶ経路は 3 系統ある。

1. `run_method_tasks` → `run_pool` ワーカーが `(method, channel-pair)`
   タスクを実行。`FullNcc::locate_coarse_to_fine`(coarse NCC +
   全探索 NCC + refine)、`BandNcc::evidence_from_bands`(帯域ごと
   NCC)、`GccPhat`(PHAT)が `dsp::ncc`/`dsp::gcc_phat` を呼ぶ
2. `window_anchors_in` → `run_pool` ワーカーが `best_window_match`
   を実行。窓テンプレートと捕捉区間の `ncc` を narrow→widened
   範囲で複数回呼ぶ
3. `propose_coarse_to_fine`(テスト・外部入口)も同じ経路

現状はすべて `dsp::ncc` フリー関数経由で、呼出ごとに
`CorrelationPlan::new`(FFT twiddle)+ `CorrelationWorkspace::new`
(複数 O(n) バッファ)+ 返却 `.to_vec()`(full-rate ~19MB)を行う。

## Scope

### In Scope

- `CorrelationPool` 型の新設(al-locate 内、呼出側所有ポリシー)
- `run_pool` のワーカーコンテキスト拡張と 2 呼出点の移行
- `LocateMethod`/`propose_coarse`/`locate_coarse_to_fine`/
  `best_window_match` の pool 引数追加
- `InputCache` の hit/miss 計数と `len()`/`stats()` 観測口
- coarse-to-fine 境界(座標換算・radius・boundary-candidate)の
  検証テスト

### Out of Scope

- `dsp::ncc`/`dsp::gcc_phat` フリー関数自体の変更・削除
  (他呼出経路・テストで継続使用)
- `al-math` の `CorrelationPlan`/`CorrelationWorkspace` API 変更
- メソッドの検出ロジック・合議の変更
- IIR/SOS フィルタ置換、stage 並列化(#54)

## Components and Boundaries

- **`CorrelationPool`**(`crates/al-locate/src/pool.rs` 新設):
  `HashMap<(usize, usize), Entry>` で `Entry { plan, workspace }` を
  保持。`ncc(&mut self, template, signal, max_lag) -> &[f64]` /
  `gcc_phat(..)` が借用曲線を返す。返却参照は pool 借用に紐づくため、
  保持中の再呼出は borrowck が静的に禁止(#52 の排他契約を型で継承)
- **`run_pool`**(`stage.rs`): `make_ctx: impl Fn() -> C + Sync` と
  `work: &(dyn Fn(usize, &mut C) -> T + Sync)` に拡張。ctx は
  `scope.spawn` 内で生成されワーカー寿命に閉じる。`C` は `Send`
  を要求しない(ワーカー内完結)
- **`InputCache`**(`prepared.rs`): `hits`/`misses` カウンタ
  (prepare が `&mut self` なので `u64` フィールド)と
  `stats() -> CacheStats`、`len()` を追加
- **`LocateMethod`**(`methods.rs`): `locate`/`locate_prepared` に
  `pool: &mut CorrelationPool` 引数を追加。`propose_coarse`/
  `propose_coarse_to_fine`/`refine_coarse` も pool 受け取りに変更
- **`best_window_match`**(`stage.rs`): `pool` 引数を追加し
  `dsp::ncc` を `pool.ncc` に置換

## Data and Control Flow

```text
run_pool(n_tasks, CorrelationPool::new, work)
  └─ worker i (scoped thread)
       ├─ let mut pool = make_ctx()        // worker 所有
       └─ loop { idx = next.fetch_add(..)
             work(idx, &mut pool)          // 逐次
               ├─ FullNcc.locate_coarse_to_fine(.., pool)
               │    ├─ propose_coarse → pool.ncc(coarse_t, coarse_s)   // shape A
               │    ├─ pool.ncc(template, signal)                     // shape B
               │    ├─ refine_coarse(&corr)    // 借用曲線を直接使用
               │    └─ evidence_from_corr(&corr)
               ├─ BandNcc → for band: pool.ncc(bt, bs)                 // shape B (帯域は全長)
               ├─ GccPhat → pool.gcc_phat(t, s)                        // shape B 再利用
               └─ best_window_match → pool.ncc(win, slice)             // shape 変動
       }
```

同一 worker 内で shape B の plan/workspace は 1 度だけ構築される。
`best_window_match` は slice 長が可変のため shape が安定しないが、
bound 規約により害は現状以下に留まる(最悪で呼出ごと再構築 =
現状と同等)。

### pool bound 規約

`MAX_SHAPES`(初期値 8)を超える miss 時は `entries.clear()` で全
evict。形状が交互に変わる悪性パターンでは thrash するが、現状
(毎回構築)より悪化しない。LRU 等の洗練は計測で必要性が示された
場合のみ検討する(YAGNI)。

## Non-Functional Constraints

- 検出結果は現行と bit 同一: plan/work は同じ `Correlation::run`
  を経由し、FFT 長・パディング・正規化は plan の shape で一意に
  決まる。pool は値に影響しない
- pool の同時メモリ ≈ `workers × live shapes × workspace bytes`。
  8M FFT workspace は ~150MB 級のため、workers=16・shapes=8 の
  悪条件で ~19GB になり得る — `MAX_SHAPES` と実測 RSS で上限を
  検証する(帯域は shape 共有で実効 shapes ≈ 2 + window 系)
- `InputCache::stats` は診断用であり、永続 artifact や JSON 出力
  スキーマには載せない

## Risks and Trade-offs

- **借用が直列化を強制**: pool からの借用曲線を保持したまま次の
  呼出ができない。現行の消費パターン(`evidence_from_corr`/
  `refine_coarse`/`peaks_from_correlation` に渡して破棄)と整合。
  `BandNcc` の `acc += c` もループ内で借用が解放される
- **`best_window_match` の shape 多様性**: `[lo..hi]` が可変で
  pool hit が限定的な可能性。それでも全 evict bound で現状より
  悪化しない設計とし、効果は計測で確認する
- **トレイト波及**: `LocateMethod` 実装・テスト・呼出点の更新。
  公開 API だが pre-release 方針に従い互換層は設けない
- **InputCache 計数の見掛け miss**: `prepare` は version 文字列を
  キーに含むため、呼出側が版を変えると miss になる — 計測時に
  version 規約の妥当性も確認する

## Deferred Design Concerns

- `dsp::ncc` フリー関数の廃止判断: 残存呼出の棚卸し後に別途
- pool 容量規約の洗練(LRU/サイズ重み付き): `MAX_SHAPES` で実害が
  出た場合のみ
- `window_anchors_in` の shape 正規化(固定長パディング等で
  hit 率を上げる): 計測で効果が小さい場合は見送り

## Verification Notes

- [ ] 同一 shape 連続呼出で plan 構築が 1 回の計数テスト
- [ ] pool 経由とフリー関数経由の出力一致テスト
- [ ] `InputCache` hit/miss・release 後 `len()` 減少のテスト
- [ ] coarse-to-fine 境界テスト(換算誤差・radius・boundary)
- [ ] 既存 locate テスト全 pass、`cargo test --workspace`、clippy
- [ ] サンプル job またはベンチでの時間・RSS 記録

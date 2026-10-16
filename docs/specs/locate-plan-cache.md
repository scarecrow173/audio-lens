---
id: "SPEC-034GMyXJaIeUXOZB86GpDC"
type: "spec"
status: "approved"
title: "locate 相関 plan ワーカープールと共有 cache 監査"
created: "2026-10-16"
updated: "2026-10-16"
owners: []
relations:
  source:
    - "../../crates/al-locate/src/stage.rs"
    - "../../crates/al-locate/src/methods.rs"
    - "../../crates/al-locate/src/prepared.rs"
    - "../../crates/al-math/src/correlation.rs"
  changes:
    added: []
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements: []
  implemented-by:
    - "../adr/locate-plan-run-pool.md"
    - "../designs/locate-plan.md"
    - "../plans/locate-plan.md"
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "./ncc-plan-workspace-reuse.md"
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

# locate 相関 plan ワーカープールと共有 cache 監査

## What Should Be Built

locate ステージの残存重複を除去する。

1. **相関 plan ワーカープール**: `dsp::ncc`/`dsp::gcc_phat` が呼出ごとに
   `CorrelationPlan` + `CorrelationWorkspace` + curve コピーを再構築
   している経路を、形状キー `(template_len, signal_len)` の
   plan/workspace pool に置き換える。pool は `run_pool` の各ワーカーが
   排他所有し、スレッド間共有しない。
2. **`InputCache` 監査計測**: 帯域フィルタ共有 cache の hit/miss 実測値と
   `release()` によるピーク RSS bounded 性の証跡を取る。
3. **coarse-to-fine 境界監査**: 間引き位置→原レート座標換算の丸め、
   近傍 radius と分解能差の関係、boundary-candidate マークを
   テストで検証する。

## Why Now

locate はサンプル実測で全パイプラインの ~60%(344.7s/571.9s)を占める。
帯域信号共有は実装済みだが、相関実行の plan/workspace 再構築が
pair・帯域ごとに残っており、#52 で確立した plan 分離 API が
locate メイン経路に未適用。issue #40 の残件として確定済みの
スコープ(監査計測 + plan 再利用、IIR 除外)を消化する。

## Users and Value

- ツール利用者: locate ステージ時間の削減(plan 構築・workspace
  確保・curve コピーの amortize)
- 開発者: cache 設計の実効性が計測証跡で担保される。将来の #54
  stage 並列化と矛盾しない所有権モデルが先に固定される

## Scope

### In Scope

- 形状キー `(template_len, signal_len)` の `CorrelationPlan`/
  `CorrelationWorkspace` pool。所有は `run_pool` ワーカー単位
- `dsp::ncc`/`dsp::gcc_phat` フリー関数経由の呼出を pool 経由に
  移行し、`.to_vec()` curve コピーを借用に置き換えられる箇所は
  置き換える
- `InputCache` の hit/miss 計数(診断用)と RSS 計測証跡
- coarse-to-fine の座標換算・radius・boundary-candidate の検証テスト
- 検出結果・証拠・合議判定は bit 互換(アルゴリズム不変、
  計算順序も同一 plan で同一)

### Out of Scope

- IIR/SOS 帯域フィルタへの置換(位相応答変化、別 issue)
- メソッド追加・合議ロジック変更
- stage レベルの新規並列化(#54)
- `al-math` の `CorrelationPlan` API 変更(既存 plan/workspace
  分離契約をそのまま利用)

## Requirements

- pool は `(template_len, signal_len)` をキーに plan + workspace を
  1 組保持する。同一キーへの 2 回目以降の呼出では新規構築しない
- pool はワーカー所有・排他利用。`Send`/`Sync` を新たに要求しない
- pool は bounded: 形状が連続して変わる入力列で無制限に膨らまない
  規約(上限数 + eviction、または pair 単位のライフサイクル)を持つ
- `CorrelationWorkspace` の借用契約(返却曲線を保持したまま
  workspace を再使用しない)を呼出側で守る。既存の借用 API と
  compile_fail 契約に反する形で曲線を保持しない
- `InputCache` に hit/miss 計数を持たせ、locate 実行で
  「prepare 呼出数 vs 実フィルタ計算数」の証跡を残す
  (計測専用の永続 UI 出力は不要、テスト/ログで検証可能であること)
- `release(capture_id, "original")` 後に当該入力の帯域データが
  実際に解放されることを検証する
- coarse-to-fine: 間引き位置→原レート座標の換算が四捨五入
  (round 半加算)で真値から ±coarse_rate 以内に収まること、
  radius が分解能差と丸め誤差をカバーすること、search_range 端の
  候補が "boundary-candidate" を返すことをテストで固定する
- 既存の locate golden/合議テストはすべて不変で通る

## Acceptance Criteria

- [ ] 同一形状の相関呼出で `CorrelationPlan::new`/`RealFftPlan::new`
      が 1 回のみ呼ばれる(計数テストまたはベンチで確認)
- [ ] `ncc()`/`gcc_phat()` フリー関数と pool 経由の出力が全テストで一致
- [ ] `InputCache` hit/miss の実測値が記録され、設計上の再利用
      (入力×帯域 1 回計算)と一致することを確認
- [ ] `release` 後の解放と複数 capture でのピーク RSS bounded 性を
      計測・記録
- [ ] coarse-to-fine の座標換算・radius・boundary テストが追加され pass
- [ ] `cargo test --workspace`・clippy `-D warnings`・fmt が clean
- [ ] locate ベンチまたはサンプル計測で plan 構築削減の効果を記録

## Deferred Design Concerns

- `dsp::ncc` フリー関数自体の削除: 現行はテスト・外部経路でも使用。
  pool 移行後に残存呼出を棚卸し、deprecate は別途判断
- pool 容量規約の具体値(上限形状数・eviction 方式)は実装時の
  計測で決める。悪性入力(交互に shape が変わる pair 列)でも
  構築コストが現状以下に収まること
- IIR/SOS 帯域化: 位相応答変化・golden 再基準化を伴うため別 issue

## Sources and Relations

- `docs/discovery/issue-40-locate-pair-plan-cache.md`(残存重複の
  コード証跡: pair あたり ~8-9 回の相関呼出、run_pool ワーカー並列、
  `.to_vec()` curve コピー)
- `docs/discovery/ncc-plan-workspace-reuse.md`(#52: plan/workspace
  分離設計、TLS 棄却、排他所有契約)
- `crates/al-math/src/correlation.rs`(`CorrelationPlan` は手法非依存、
  `ncc`/`gcc_phat` が同一 plan で実行可能)

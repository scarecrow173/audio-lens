---
id: "TSPEC-034z8k5JiNwiAsnNVpk7vn"
type: "test-spec"
status: "proposed"
title: "locate-tracking-residual"
created: "2026-10-02"
updated: "2026-10-02"
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
    - "docs/designs/localization.md"
  derived-by: []
  verifies:
    - "docs/specs/locate-tracking.md"
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# locate-tracking-residual

## Purpose

ループ反復・長尺トラッキング・混合推定・残差探索の永続化契約を
固定する。time map と減算来歴の表現力を保証する。

## Feature

ループ/長尺ソースのトラッキングと残差探索

## Rules

- ループ反復は個別に識別される(1 つの曖昧マッチに潰れない)。
  必要に応じ 2〜3 個連結したループ反復で探索する
- 出現ごとにループ index・出現位置・ソース位相/時間・
  キャプチャ時間を保存する
- time map は anchor + segment の区分的表現で、連続区間・
  ドリフト・局所ジッタ・スキップ・ギャップ・再開・未マッチを
  表せる
- 多 ch 混合は最小二乗で符号付き係数(極性反転は負)として推定し、
  適合品質と条件数を記録する(リッジ正則化)
- 残差探索は 2 段階で、減算来歴(親ソース・alignment id・
  推定ゲイン/チャンネル変換・減算前後エネルギー・減算確度・
  子 locate)を保持する

## Examples

- Given 同じループ素材が 3 回出現するキャプチャ、When tracking、
  Then 3 つの出現が個別に識別される
- Given 線形ドリフトした挿入、When tracking、Then time map が
  単一オフセットではなくドリフト区間として生成される
- Given スキップ+再開のある挿入、When tracking、Then gap と
  resume が区分的に表現される
- Given 極性反転した ch を含む混合、When mixer-fit、Then
  対応係数が負で推定される
- Given 支配的 BGM + 静かな素材、When 残差探索、Then 減算後に
  静かな素材が受理され減算来歴が残る

## Guarantees

- ループの各出現が区別可能なまま永続化される
- time map が spec の全現象(ジッタ含む)を表現できる
- 多 ch 減算はスカラーゲインでなく混合行列を使う

## Non-goals

- time map の映像側との照合(音声のみ)
- 残差の音質評価

## Risk

- 出現の潰れ・time map の表現欠落は spec 不適合となる
- 減算来歴の欠落は provenance 連鎖を断つ

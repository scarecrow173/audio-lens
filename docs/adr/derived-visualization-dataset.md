---
id: "ADR-034yYyqyqy2CzywC1zpHME"
type: "adr"
status: "superseded"
title: "可視化を派生データセットとして分離する"
created: "2026-10-02"
updated: "2026-10-31"
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
  implements:
    - "../specs/visualization-dataset.md"
  implemented-by: []
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by:
    - "../impl/ir/issue-106-viz-removal.md"
  related:
    - "analysis-dataset-storage.md"
    - "../specs/analysis-export.md"
    - "../specs/audiolens-overview.md"
    - "../specs/visualization-dataset.md"
    - "immutable-resumable-job.md"
  refines: []
  refined-by:
    - "ruviform-foundation-migration.md"
  derives-from:
    - "../discovery/initial-requirements-consolidation.md"
  derived-by: []
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
metadata:
  adr:
    decision-makers: ["scarecrow173"]
    consulted: []
    informed: []
---

> **Superseded (Issue #106)**: この決定は取り消された。派生
> Visualization Dataset とその生成ステージは廃止され、表示は
> Analysis Dataset を `al-service`/`DatasetSession` 経由で直接読む
> `al-gui` + `ruviform-viz` が担う(Ruviform #156/#157)。可搬 export
> は表示責務から分離され `al-dataset::export` が所有する。移行判断
> の記録は
> [`../impl/ir/issue-106-viz-removal.md`](../impl/ir/issue-106-viz-removal.md)
> を参照。以下は歴史的記録として残す。

# 可視化を派生データセットとして分離する

## Context and Problem Statement

可視化を解析パイプラインの内部ステップやスクリーンショット生成に
埋め込むと、表示のたびに再解析が必要になり、レンダリング用の
最適化が意味解析を汚染する。可視化は Analysis Dataset から派生する
独立成果物である必要がある。

## Decision Drivers

* 長尺キャプチャをズーム / パン可能にするには LOD / タイル化が必要
* 可視化は解析結果の再現物であり、新しい意味解釈を発明しない
* 可視化の再構築にメディアデコードや特徴抽出を再実行したくない
* confidence / validity マスクは描画時に潰さず保持したい

## Considered Options

* UI または解析時に直接描画する(画像 / スクリーンショット中心)
* Analysis Dataset から派生した可視化データセットを別成果物として
  生成する
* 可視化用に別の解析パイプラインを持つ

## Decision Outcome

Chosen option: "Analysis Dataset から派生した可視化データセットを
別成果物として生成する"。描画に必要な形へ事前変換することで、
再解析なしの再構築と複数ビューの同期を可能にするため。

### Consequences

* Good, because 可視化が再現可能で、LOD / タイル / マーカー等の
  レンダリング最適化と解析意味の分離ができる。
* Bad, because 可視化用の追加ストレージと生成ステップが必要になる。

### Confirmation

visualization builder が保存済み Analysis Dataset だけから再現
可能で、validity / confidence マスクを保持したまま
レンダリングデータを生成することを確認する。

## Implementation Plan

* Affected paths: visualization builder、可視化データの保存形式、
  イベントマーカー、共有タイムライン
* Patterns to follow: Analysis Dataset を入力として LOD・タイル・
  ヒートマップ・time-map 系列・イベントマーカーを生成する
* Patterns to avoid: 可視化中に新しい意味解析を行う、マスクや
  不確かさを焼き込む、生 PCM の全ズーム読み出しを前提にする
* Dependencies or configuration: LOD / タイルの保存形式は
  可視化コンシューマ側の要件に依存する

## Verification

* [ ] 可視化がメディアデコードや特徴抽出なしに再構築できる
* [ ] 共有タイムライン上で波形・マーカー・ヒートマップ等が
  アラインされる
* [ ] confidence / validity マスクが可視化データに残る

## Pros and Cons of the Options

### UI または解析時に直接描画する(画像 / スクリーンショット中心)

* Good, because 実装が最も直接的で初速が速い。
* Bad, because 再解析や再レンダリングが必要で、大規模データや
  別ビューへの展開が困難になる。

### Analysis Dataset から派生した可視化データセットを別成果物として生成する

* Good, because 再現性・ズーム効率・複数ビュー同期を確保できる。
* Bad, because 可視化用データの生成・保存コストが増える。

### 可視化用に別の解析パイプラインを持つ

* Good, because 表示に特化した最適化はできる。
* Bad, because 解析結果との不整合が生じ、二重実装になる。

## More Information

可視化データの要件は `../specs/visualization-dataset.md`、
派生元のデータセット契約は `../specs/analysis-dataset.md`、
可搬性は `../specs/analysis-export.md` を参照。

---
id: "KNOW-dM8sVwXcPzBnLkQfHjTy"
type: "knowledge"
status: "approved"
title: "可視化と export: dataset 由来製品・決定性・パッケージ"
created: "2026-11-19"
updated: "2026-11-19"
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
  derives-from: []
  derived-by: []
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# 可視化と export: dataset 由来製品・決定性・パッケージ

`al-viz` は Analysis Dataset を入力に可視化製品を生成する層で、新しい診断意味論を導入しない —— dataset 値の「再計算なし」の射影のみを行う(後の Ruviform 移行で al-viz 表示層は撤去され、export のみ `al-dataset` へ移設された)。本書は出力構造、決定性契約、export パッケージ、および golden 検証の知見をまとめる。

## 出力構造と skipped_products

出力は `analysis/visualization/` 配下で、再実行時はその role だけを置き換える。`skipped_products` は「必須だが生成されなかった製品」を記録し、「生成されたが期待 artifact が無い」は明示エラーとする —— 両者を区別しないと「未生成」を「生成失敗」と混同する。

`timeline.json` を最初に書き、units/window/hop の規約をここで確定する。製品群は timeline・LOD・tiles・views・summary・report・golden からなり、各 stage span(`viz::{timeline,lod,tiles,views,summary,report,package,golden}`)で計装される。

## 決定性契約

ファイル内容は dataset・schema version・config から決定的でなければならない。生成された artifact ID は決定性の対象外(ランごとに変わり得る)。`export_manifest.json` の `generator.id` は生成側 crate 名(`al-viz`→撤去後は `al-dataset`)。

## export パッケージ

export は `analysis/` + `visualization/` artifacts を複写し、media も任意で同梱する。絶対パスは除去し、hash と manifest metadata を含める。`export.name` は traversal/絶対/区切り/空名を拒否する。同一 basename 衝突は退避して対処する。

`al export` は `StageRunner::run_stage`/`run_stage_with_hooks` を使う単一 stage 実行で、依存 stage の完了記録のみを見て既存 job に追加実行できる。`ExportStage::depends_on` は `[Dataset]`(viz 成果物を必須入力としない実依存への修正、`export_stage_does_not_require_visualization` で固定)。

## golden 検証

export の golden 照合は相対パス→SHA-256 snapshot 方式で、`golden/snapshot.json` に記録する。`report.html`/`report.data.js` は撤去済み(静的 HTML レポートは GUI 直接表示に置き換わった)。

## Ruviform 移行後の代替経路(AL#106)

旧 `visualization/lod` → `WaveformSource`+`ruviform_viz::encode_buckets`(PCM 範囲読み+min/max 畳込みのみ)、`tiles` → `SpectrumSource`+`ruviform_viz::Spectrogram::tile`(永続化複素 STFT から、STFT 再計算なし)、`timeline` → `ruviform_viz::timeline::FrameGrid`、`views` → al-service view op、`summary`/report → GUI 直接表示。再計算は全て「なし」。

## 設計上の教訓

- 可視化製品は「dataset の射影」に限定し、診断意味論を新設しない。表示層が独自に解釈を加えると dataset と GUI で値が食い違う。
- 「必須だが未生成」と「生成失敗」を `skipped_products` と明示エラーで分ける。
- 生成物の内容を決定的に保ち、非決定的なのは生成 ID のみに限定する。
- stage の `depends_on` 宣言は実依存に合わせ、過剰指定(export が viz を要求)は契約テストで修正する。

関連 Issue/PR: AL#105、AL#106、#33、RF#156、RF#157。

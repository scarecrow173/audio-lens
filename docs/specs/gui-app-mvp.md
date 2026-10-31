---
id: "SPEC-035guiAppMvp0000000000000"
type: "spec"
status: "proposed"
title: "GUI アプリ MVP 画面・操作仕様"
created: "2026-11-27"
updated: "2026-11-27"
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
  implements: []
  implemented-by: []
  depends-on:
    - "analysis-dataset.md"
    - "analysis-job-model.md"
    - "candidate-event-model.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "../designs/gui-app-architecture.md"
    - "../designs/ruviform-migration-responsibility-map.md"
  refines:
    - "audiolens-overview.md"
  refined-by: []
  derives-from: []
  derived-by: []
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# GUI アプリ MVP 画面・操作仕様

Issue #105 の MVP 画面・操作・表示契約。実装層の詳細
(crate 構成・IPC op 一覧・ライフサイクル)は
`../designs/gui-app-architecture.md` を参照。

## 目的と原則

- GUI は永続 Analysis Dataset の**閲覧ビュー**であり、解析結果の
  生成装置ではない。表示に必要な範囲だけを `al-dataset` の range
  reader 経由で読む。GUI が解析・再計算を行わない(描画用
  downsample / LOD のみ許可)。
- 測定の意味を保存する:`Validity` / `Confidence` / `support_count` /
  `Provenance`(algorithm・alignment・lineage)を省略せず、
  「合否ビュー」や「原因断定」に還元しない。
- `al-cli` の `run` / `resume` / `inspect` と同一の Job
  サービス層を使い、同一 Job / 同一結果を扱う。

## 用語

- **セッション**: GUI が開いている1つの dataset または job。
- **ロール**: dataset manifest の `inputs` / `locate` / `intermediate` /
  `features/*` / `residual` / `events` / `video` 等の artifact 区分。
- **タイル / LOD**: 表示解像度に合わせた有界ペイロード
  (ruviform-viz `RVST` / `RVBW`)。

## 画面構成(MVP)

1画面(単一ウィンドウ)に左ナビ + メインペインの構成。
MVP では複数ウィンドウ・タブは持たない。

### ナビ(左ペイン)

- **ジョブ**: `base_dir` 配下の job 一覧(JobId・入力・stage 状態・
  エラー有無)。選択で inspect 表示。
- **データセット**: 開いている dataset のロール/artifact ツリー
  (browse 用)。
- **新規ジョブ**: ジョブ作成フォームを開く。

### メインペイン(状態で切替)

1. **ジョブ作成フォーム**
   - 入力追加: file パス + `InputRole` 選択(capture_bug /
     capture_reference / source_reference / source_ambience 等) +
     input_id 任意指定。
   - stage 選択: `Ingest`〜`Export` の pipeline order から部分集合を
     選択(既定は Dataset まで+解析系)。
   - `config.json` 任意指定。
   - 実行 → ジョブ進行ビューへ遷移。

2. **ジョブ進行 / inspect ビュー**
   - stage 一覧: Pending/Running/Completed/Failed + duration_ms +
     error 表示。
   - progress は stage 粒度(完了 stage / 全 stage)。キャンセル要求は
     stage 境界で効く(キャンセル = 次 stage 起動せず manifest を
     Pending のまま保存; resume で続行可能)。
   - `run` / `resume` / `cancel` 操作。失敗時は stage 名+error メッ
     セージを表示。
   - `al-cli inspect` と同一 manifest を読むため結果は一致する。

3. **データセット概要**
   - dataset root(job の `analysis/` または export package の
     `analysis/`)を開いた直後の画面。
   - 入力一覧(role・source path・sha256)、ロール別 artifact
     数、produced / not_produced 状態。
   - タイムライン概要: サンプルレート・フレーム数・各入力の有無。

4. **タイムライン閲覧(中核画面)**
   - 共通時間軸(秒)に対し、選択入力の:
     - **波形**(`inputs/<id>.wav` → `WaveformSource` → LOD/オンデマンド
       `RVBW`)
     - **スペクトログラム**(`intermediate/<pair>/stft_*` 等の永続
       複素スペクトルを `SpectrumSource` 経由で `RVST` タイル化。
       STFT の再計算はしない)
     - **イベント候補**(`events/candidates` table を時間位置に
       マーカー表示。kind 別色分け、クリックで詳細)
     - **locate アライメント**(capture_range を時間軸に区間表示)
   - 操作: スクラブ(クリック/ドラッグで viewport 移動)、ズーム
     (ホイール/ボタン)、範囲選択(start/end 秒の指定)、チャネル選択。
   - 要求データは viewport に比例した有界範囲のみサーバ側で読み、
     `rvf://data` pull で転送する(全量ロードしない)。

5. **フィーチュ / イベント詳細パネル**
   - `features/{cheap,reference,expensive,cross_capture}` の
     `FeatureRecord` を feature_id・input・時間範囲で検索/一覧。
   - 1レコード選択で全フィールド(value・unit・time/freq range・
     channels・resolution・target/reference/delta・confidence・
     validity・invalid_reason・support_count・algorithm_id/version・
     params・input_id・alignment_id・lineage)を表示。
   - record が参照する tensor(`features/**`, `intermediate/**`)は
     artifact ブラウザへリンク。

6. **アーティファクトブラウザ(汎用アクセス経路)**
   - dataset 内の全 artifact を列挙し、kind に応じて:
     - `table`: 行範囲+列選択で読み出し(ページング)
     - `tensor`: shape/chunk_shape と領域指定読み出し(統計量表示)
     - `audio`: フレーム数・sample_rate・チャネル + 波形表示リンク
     - `json`: 整形表示
   - これが「全特徴量/中間データへのアクセス経路」の保証になる。

## 状態・エラー表示

- `NotProduced` ロールは「未生成」として明示(グレー表示)、存在し
  ない artifact 参照はエラー扱いにしない。
- `Validity` は enum 値をそのまま表示
  (`decode_input_unavailable`, `ambiguous_alignment` 等)。
- dataset manifest 破損・schema 不一致・部分成果物はサービス層の
  エラーを GUI がそのまま通知表示する。
- ジョブ実行中のウィンドウクローズ: バックグラウンド job は継続、
  再表示で進行を再取得できる(worker は UiRuntime の job mailbox で
  管理)。

## 非目標(MVP 外)

- 複数 dataset の同時比較ビュー、エクスポート UI、
  リアルタイムストリーミング表示、動画フレーム再生。
- `al-viz` 派生 Visualization Dataset との連携(#106 で撤去)。
- WebView 実機動作の CI 自動化(手動検証手順として記録)。

## 表示契約(フロントへ渡す最小情報)

| ビュー | 転送 | 内容 |
| --- | --- | --- |
| 波形 | `rvf://data` RVBW + meta JSON | bins・min/max・coverage・sample_range |
| スペクトログラム | `rvf://data` RVST + meta JSON | rows/cols・dB range・colormap・frame range |
| イベント | JSON | event_id・kind・input_id・start/end・strength・confidence・validity・evidence |
| locate | JSON | alignment_id・src/cap・ranges・accepted・validity・confidence・agreeing_methods・evidence |
| FeatureRecord | JSON(素の record そのまま) | 上記「詳細パネル」の全フィールド |
| アーティファクト | JSON(meta)+ 範囲読み結果 | schema・shape・provenance・parents |

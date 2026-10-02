---
id: "SPEC-034yYlrxDrz2YSF7QQQ2AK"
type: "spec"
status: "proposed"
title: "AudioLens 全体仕様"
created: "2026-10-02"
updated: "2026-10-02"
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
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "../adr/measurement-over-verdict.md"
    - "../adr/analysis-dataset-storage.md"
    - "../adr/multi-method-locate-consensus.md"
    - "../adr/immutable-resumable-job.md"
    - "../adr/derived-visualization-dataset.md"
  refines: []
  refined-by:
    - "analysis-job-model.md"
    - "media-ingest.md"
    - "source-localization.md"
    - "locate-tracking.md"
    - "analysis-dataset.md"
    - "cheap-analysis-features.md"
    - "reference-analysis.md"
    - "expensive-analysis.md"
    - "candidate-event-model.md"
    - "video-av-timing.md"
    - "visualization-dataset.md"
    - "validation-strategy.md"
    - "analysis-export.md"
  derives-from:
    - "../discovery/initial-requirements-consolidation.md"
  derived-by: []
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# AudioLens 全体仕様

## What Should Be Built

ゲームのキャプチャ映像と任意個数の比較用入力(リファレンスキャプチャ、
ソース素材)を入力として受け取り、堅牢にアラインした上で「音声に何が
起きたか」を計測し、構造化された解析データと可視化用データを出力する
ゲーム音声デバッグ解析ツールを構築する。

パイプラインの全体構成:

```text
Input
  ↓
Ingest / Decode
  ↓
Locate
  ↓
Cheap Analysis
  ↓
Reference Analysis (アライン済みリファレンスが存在する場合)
  ↓
Candidate Detection
  ↓
Expensive Analysis (候補 / 手動 / 常時実行)
  ↓
Analysis Dataset
  ↓
Visualization Builder
  ↓
Visualization Dataset
```

解析ステージ:

- Stage 1 — Cheap features: 解析可能な全キャプチャ / リファレンス信号に
  対して常に計算する
- Stage 2 — Reference features: 信頼できるアライン済みリファレンス /
  ソース区間が存在する場合のみ計算する
- Stage 3 — Expensive features: 候補区間、手動指定区間、または明示的な
  always-run 設定時にのみ計算する

## Why Now

ゲーム音声の不具合(エンジン側のボイス管理、DSP、エンコード、
キャプチャ経路など多層の原因)は、単一の類似度スコアや合格/不合格
判定では切り分けられない。未知の不具合を前提とした「観測事実を
保持する解析基盤」が必要であり、そのためには計測値・中間スペクトル・
確度・来歴を体系立てて保存する仕様が先に必要である。

## Users and Value

- ゲームの音声バグを調査する開発者 / QA / サウンドプログラマ:
  「どこで・どのように音声が変化したか」の観測証拠を得られる
- バグレポート作成者: 再現可能な解析パッケージと要約を共有可能
- 将来の AI 診断コンシューマ: 構造化データを入力として利用できる
  (AI 推論自体はコア解析器の責務外)

## Scope

### In Scope

- 対象キャプチャ 1 つ + 任意個数のリファレンス / ソース素材を
  ロールメタデータ(`capture_bug`, `capture_reference`, `source_asset`
  およびユーザー定義の拡張ロール)付きの一般化入力として扱う
- ソース素材の定位(locate)を解析の前提処理として行い、どのソース
  区間がどのキャプチャ区間に対応するかを決定する
- 3 ステージ(cheap / reference / expensive)の計測
- 観測値・中間スペクトル・確度・validity・来歴を保持する
  Analysis Dataset と、そこから派生する Visualization Dataset
- 「異常が観測されなかった」ことと「計測不能 / 低確度」を
  区別して表現する
- 正常キャプチャを特別な実行パスではなくリファレンスとして扱う

### Out of Scope

- 根本原因の断定(例:「Wwise のボイススティール不具合」)
- ゲームバグとしての severity 判定
- 単一の総合類似度スコア
- 成功条件としての AI 診断(AI 推論はコア解析器に含めない)

## Requirements

- ツールは「オーディオ一致スコア」や合否判定器ではなく、観測事実を
  出力する計測基盤であること
- 未知のバグを前提とし、後から再解釈できるよう広い低レベル観測量を
  保持すること
- すべてをスカラー特徴に縮約せず、中間スペクトルと時間-周波数データを
  保存すること
- 観測事実と後段の仮説 / 診断を分離すること
- すべての派生値が信頼性と来歴を理解するのに十分なメタデータを
  持つこと
- 保存形式の原則:
  - 表形式 / 時系列特徴: Parquet または同等のカラムナ形式
  - 大規模 N 次元テンソル: チャンク化された Zarr 相当形式
  - 音声の中間物 / 残差: WAV / FLAC
  - メタデータ / スキーマ / 来歴: JSON
  - 複素 STFT と代替解析を導出できる十分な基礎量を保持する
  - アルゴリズムバージョン、パラメータ、サンプルレート、
    window/hop、alignment id、確度、validity、入力来歴を永続化する
- 可視化出力はスクリーンショットではなく派生データセットであり、
  visualization builder は新たな意味解析を行わず、Analysis Dataset を
  waveform LOD ピラミッド、スペクトログラムタイル、ヒートマップ、
  time-map 系列、イベントマーカー等のレンダリング可能な形に
  変換するだけであること

## Acceptance Criteria

- [ ] 対象キャプチャと任意個数のリファレンスを 1 ジョブとして
  受け付け、Ingest → Locate → Cheap → Reference → Candidate →
  Expensive → Dataset → Visualization の各段を実行できる
- [ ] リファレンス不在でも cheap ステージが完走し、リファレンス
  依存の特徴は `not applicable` 等で表現される
- [ ] 任意の派生値について、生成に使われた入力・アルゴリズム・
  バージョン・パラメータを dataset から追跡できる
- [ ] 「異常なし」と「計測不能」が異なる状態として記録される
- [ ] 可視化データの再生成にメディアデコードや特徴抽出の再実行が
  不要である
- [ ] 出力に総合類似度スコアや合否 / severity 判定が含まれない

## Deferred Design Concerns

- AI 診断コンシューマ | コア解析器の責務外 | エクスポートされた
  データを入力とする後続プロジェクト立ち上げ時 | 先回りして診断
  セマンティクスをスキーマに混入させるリスク
- 実装言語 / ランタイム / DSP ライブラリ選定 | 本書作成時点では
  未決定 | 開発環境セットアップ着手時 | 選定遅延による着工遅延

## Sources and Relations

領域別の要件は `refined-by` に列挙した各 spec に分解して記録する。
横断的なアーキテクチャ決定(観測主義、データセット保存形式、
locate 合議、ジョブモデル、可視化派生データ)は `related` の
ADR を参照。

# audio-lens

AudioLens は、ゲーム音声デバッグのための解析ツールです。

ゲームのキャプチャ映像と任意の比較用入力を受け取り、robust に
アラインし、音声に何が起きたかを計測して、詳細な構造化解析
データを export します。

AI ベースの診断は、export されたデータの consumer として後から
追加される可能性はありますが、**コアアナライザの一部では
ありません**。

## Core principles

- 「音声マッチスコア」や合否判定を出すツールではない
- source material の locate は解析の前提条件。どの source 区間が
  どの capture 区間に対応するかを決める
- 未知のバグを想定し、後から再解釈できるよう低レベルの
  observable を広く保持する
- すべてを scalar feature へ縮約せず、中間スペクトルと
  時間-周波数データを保持する
- 観測された事実と、後続の hypothesis / diagnosis を分離する
- すべての派生値は reliability と provenance を理解できる
  metadata を持つ
- "no anomaly observed" と "measurement unavailable /
  low confidence" を区別する
- target capture + 任意個数の reference を `capture_bug` /
  `capture_reference` / `source_asset` などの role で扱う。
  正常系 capture も特別扱いせず reference の一種として扱う

## Pipeline

```text
Input
  ↓
Ingest / Decode
  ↓
Locate
  ↓
Cheap Analysis
  ↓
Reference Analysis (when aligned reference exists)
  ↓
Candidate Detection
  ↓
Expensive Analysis (candidate/manual/always-run)
  ↓
Analysis Dataset
  ↓
Visualization Builder
  ↓
Visualization Dataset
```

## Analysis stages

- **Stage 1 — Cheap features**: 適用可能なすべての
  capture / reference 信号に対して常時計算する
- **Stage 2 — Reference features**: 信頼できるアライン済み
  reference / source 区間が存在する場合のみ計算する
- **Stage 3 — Expensive features**: candidate 区間・手動指定区間、
  または明示的に always-run 設定された場合に計算する

## Storage principles

- 表形式・時系列 feature: Parquet 相当の columnar 形式
- 大きな N 次元 tensor: chunked な Zarr 系 storage
- 音声の中間データ・residual: WAV / FLAC
- metadata / schema / provenance: JSON
- 複素 STFT と、後から別解析を導出できる十分な基礎量を保持する
- algorithm version・パラメータ・sample rate・window/hop・
  alignment id・confidence・validity・source lineage を永続化する

## Visualization principles

表示・検査は永続化した Analysis Dataset を直接読む GUI
(`al-gui` + `ruviform-viz`)が担う。表示のためだけに意味解析を
再実行しない — waveform は `inputs/<id>.wav` の範囲読み、
spectrogram は永続化済みの複素 STFT テンソルを tile 単位で読む。
表示専用の派生 dataset や静的 HTML レポートは持たない
(Issue #106 で撤去)。

## Non-goals(コアアナライザ)

- 根本原因の断定(例: 「Wwise の voice stealing バグ」)
- ゲームバグの深刻度 verdict
- 単一の総合類似度スコア
- AI 生成 diagnosis を解析成功の要件にすること

## Project status

設計フェーズ。全体アーキテクチャ・スコープ・各 stage の要件は
GitHub issues で管理しています。

- [#1](https://github.com/scarecrow173/audio-lens/issues/1)
  全体アーキテクチャとプロジェクトスコープ(正本)
- #2〜#14: Core / Ingest / Locate / Data / Analyze / Video /
  Visualization / Validation / Export の各 stage issue

実装順序の提案も issue #1 に記載されています。言語 stack は
未定であり、決定次第 toolchain と検証 task を `mise.toml` へ
追加します。

## Development

開発環境の正規インターフェースは mise task です。

```bash
mise install            # toolchain 復元(mise.lock 準拠)
mise run setup          # APM 依存の復元(apm install)
```

Coding Agent を使う場合は `AGENTS.md`(agent 向け指示)も参照
してください。

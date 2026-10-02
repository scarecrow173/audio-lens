---
id: "SPEC-034yXK5rGLsLHXCm7iJ3FC"
type: "spec"
status: "draft"
title: "AudioLens 全体アーキテクチャとプロジェクトスコープ"
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
  related: []
  refines: []
  refined-by: []
  derives-from:
    - "https://github.com/scarecrow173/audio-lens/issues/1"
  derived-by: []
  verifies: []
  verified-by: []
  references:
    - "https://github.com/scarecrow173/audio-lens/issues/1"
  defers: []
  deferred-by: []
---

# AudioLens 全体アーキテクチャとプロジェクトスコープ

> 転記元: GitHub issue #1 「[Spec] AudioLens overall architecture and
> project scope」。本ドキュメントが承認された時点で canonical となり、
> issue #1 は work tracking に徹する。

## What Should Be Built

AudioLens はゲーム音声デバッグのための解析ツール。ゲームの
キャプチャ映像と任意の比較用入力を受け取り、robust にアラインし、
音声に何が起きたかを計測して、詳細な構造化解析データと可視化用
データを export する。

## Why Now

ゲーム音声のバグ(音が出ない・遅れる・歪む・ループが壊れる等)は
原因特定が難しく、人手による耳と目視の確認に依存している。
キャプチャとソース素材を機械的にアライン・計測し、観測事実を
構造化して残すことで、原因調査の土台を自動化する。

## Users and Value

- ゲーム音声の QA / サウンドプログラマ: バグ報告に「どの区間で
  何が起きたか」の観測データを添えられる
- 解析結果の consumer(将来の AI 診断など): 中間データを含む
  再解釈可能な dataset を得られる

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

## Scope

### In Scope

- media decode / timing integrity / channel metadata の ingest
- NCC・band-NCC・PHAT・spectral-envelope consensus による
  multi-method source localization、loop / long-source / drift /
  multichannel の扱い
- 3 段階の解析(cheap / reference / expensive)と candidate /
  event モデル、confidence propagation
- 映像由来の timing feature と A/V 同期解析
- analysis dataset schema・provenance・validity 管理と export
- render-ready な可視化 dataset の構築

### Out of Scope

- 根本原因の断定(例: 「Wwise の voice stealing バグ」)
- ゲームバグの深刻度 verdict
- 単一の総合類似度スコア
- AI 生成 diagnosis を解析成功の要件にすること
  (AI 診断は export データの consumer として後続で追加されうる)

## Requirements

- tabular / 時系列 feature は Parquet 相当の columnar 形式で保持する
- 大きな N 次元 tensor は chunked な Zarr 系 storage で保持する
- 音声の中間データ・residual は WAV / FLAC で保持する
- metadata / schema / provenance は JSON で保持する
- 複素 STFT と、後から別解析を導出できる十分な基礎量を保持する
- algorithm version・パラメータ・sample rate・window/hop・
  alignment id・confidence・validity・source lineage を永続化する
- visualization builder は新しい意味解析を行わず、Analysis
  Dataset を waveform LOD pyramid・spectrogram tile・heatmap・
  time-map series・event marker 等の render-ready 形式へ
  変換するだけとする

## Acceptance Criteria

- [ ] capture + 任意個数の reference を role 付きで入力できる
- [ ] locate により source↔capture の区間対応が provenance 付きで
      得られる
- [ ] 解析結果が再解釈可能な中間データを含む dataset として
      export される
- [ ] 可視化 dataset が Analysis Dataset からの変換のみで
      構築される
- [ ] 「異常なし」と「計測不可 / low confidence」が区別される

## Sub-scope issues(草案)

要件の詳細化は GitHub issues に草案として存在し、canonical 化は
本 spec 群および子 spec / design doc で行う。

| Issue | 領域 |
|---|---|
| #2 | Core: job model・input roles・orchestration・config・再現性 |
| #3 | Ingest: decode・timing integrity・channel metadata・capture-artifact dictionary |
| #4 | Locate: NCC / band-NCC / PHAT / spectral-envelope consensus |
| #5 | Locate: loop・long-source・drift/skip/restart・multichannel・residual search |
| #6 | Data: dataset schema・storage・provenance・confidence・validity |
| #7 | Analyze/Cheap: 常時計算 feature・中間スペクトル保持 |
| #8 | Analyze/Reference: reference-aligned 比較・transfer function・channel response・staged residual |
| #9 | Analyze/Expensive: comb・SRC・echo・nonlinearity・codec・timing 異常 |
| #10 | Analyze: candidate/event モデル・multi-resolution・confidence propagation |
| #11 | Video: 映像由来 timing・A/V 同期 |
| #12 | Visualization: 同期 timeline・spectra・transfer・stereo・residual view |
| #13 | Validation: golden fixture・regression・数値検証 |
| #14 | Export: human-readable summary・portable package layout |

## Deferred Design Concerns

- 言語 / 実装 stack — 未決定。決定次第 toolchain と検証 task を
  `mise.toml` へ追加する(関連: docs/adr の初期 toolchain ADR)
- 推奨実装順序(issue #1 記載): #2・#6 → #3 → #4/#5 → #7 →
  #8 → #10 → #9 → #11 → #12 → #14。#13 は全 stage と並走。
  厳密な依存グラフではなく、schema と validation は各実装
  ステップと共に育てる

## Sources and Relations

- GitHub issue #1 が一次情報。承認後は本書が正本となる

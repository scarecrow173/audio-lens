---
id: "ADR-034yYy5Xx8zznKyVgAgdtP"
type: "adr"
status: "proposed"
title: "Analysis Dataset の保存形式を分割する"
created: "2026-10-02"
updated: "2026-10-14"
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
    - "../specs/analysis-dataset.md"
  implemented-by: []
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "derived-visualization-dataset.md"
    - "../specs/analysis-export.md"
    - "../specs/audiolens-overview.md"
    - "../specs/analysis-dataset.md"
    - "immutable-resumable-job.md"
  refines: []
  refined-by: []
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

# Analysis Dataset の保存形式を分割する

## Context and Problem Statement

Analysis Dataset は、表形式の特徴、大規模な時間-周波数テンソル、
音声波形、メタデータ / 来歴を混在して保持する必要がある。単一形式に
寄せると、クエリ性・チャンク読み出し・可搬性のいずれかを犠牲にする。

## Decision Drivers

* フレーム / セグメント特徴は列指向で効率的に範囲走査したい
* STFT / PSD / CSD / 伝達関数テンソルは全体を読まずにチャンク /
  範囲で読みたい
- 音声の中間物 / 残差は汎用の音声形式で再生・再解析したい
- manifest / スキーマ / 設定 / 来歴は人間が読める構造がほしい
- スキーマバージョンとアルゴリズムバージョンを独立に管理したい

## Considered Options

* 全成果物を単一のリレーショナル / ドキュメント DB に格納する
* JSON メタデータ + Parquet 表形式 + チャンクテンソル(Zarr 相当)+
  WAV に分割する
* すべてを自己流のバイナリ / JSON に格納する

## Decision Outcome

Chosen option: "JSON メタデータ + Parquet 表形式 + チャンクテンソル
(Zarr 相当)+ WAV に分割する"。各保存ロールをデータ形状に
対応させることが、クエリ性・再現性・可搬性を同時に満たすため。

### Consequences

* Good, because 表・テンソル・音声・メタデータがそれぞれ適した
  フォーマットで保存され、大規模テンソルの部分読み出しと
  manifest ベースの来歴追跡が両立する。
* Bad, because 複数フォーマットの読み書きとバージョン管理が必要で、
  実装は単一形式より複雑になる。

### Confirmation

保存された解析成果物が `manifest.json` とロール別フォーマットに
分割され、大規模テンソルを全体読み込みなしに範囲参照できることを
確認する。

## Implementation Plan

* Affected paths: `analysis/` 配下のデータセットレイアウト、
  dataset reader / writer、export / visualization builder
* Patterns to follow: manifest.json にスキーマ / 設定 / 来歴を集約し、
  各ロールの実フォーマットは spec の推奨レイアウトに従う
* Patterns to avoid: スカラー要約への縮約、単一フォーマットへの
  強制、チャンク読み出しを阻害するファイル配置
* Dependencies or configuration: Parquet / チャンクテンソル /
  WAV の読み書きライブラリ選定は開発環境決定時に行う

## Verification

* [ ] manifest + Parquet + チャンクテンソル + WAV の役割が
  分離されている
* [ ] 大規模テンソルを全体読み込みなしに範囲 / チャンクで読める
* [ ] スキーマバージョンとアルゴリズムバージョンが独立している

## Pros and Cons of the Options

### 全成果物を単一のリレーショナル / ドキュメント DB に格納する

* Good, because 一貫したクエリインターフェースを持てる。
* Bad, because 大規模テンソルの部分読み出しと音声波形の汎用再生が
  難しく、ポータブルなパッケージにしにくい。

### JSON メタデータ + Parquet 表形式 + チャンクテンソル + WAV に分割する

* Good, because 各データ形状に適したフォーマットを選べ、範囲読み・
  可搬性・人間可読性を両立できる。
* Bad, because フォーマット数が増え、読み書きとバージョン検出の
  実装が複雑になる。

### すべてを自己流のバイナリ / JSON に格納する

* Good, because 実装が最も単純で依存が少ない。
* Bad, because 大規模テンソルの部分読み出しや汎用ツールでの利用が
  難しく、検証・デバッグの透明性も下がる。

## More Information

推奨レイアウトと validity セマンティクスは
`../specs/analysis-dataset.md`、可搬性要件は
`../specs/analysis-export.md` を参照。

## #33 の音声保存形式の実施判断 (2026-10-11)

当時の保存ロールは WAV/FLAC 方針を維持し、FLAC 保存を coordinator 承認済み
defer とする。現行は WAV のみを実装し、#32 の減算後残差は Float64 WAV と
provenance で保持する。FLAC 圧縮を実装済みとは扱わず、保存容量・I/O の
実測と浮動小数解析値の保持方式を review し、新 codec 依存は明示承認を得る。
RIFF 上限超過はエラーとし、別形式への暗黙変換は行わない。
[詳細と証拠](../tasks/spec-residual-decisions.md) を参照。

## #33 現在の正式契約 (2026-10-14)

2026-10-11 の FLAC defer 判断とその完了証拠は履歴として保持する。2026-10-14 に
ユーザーは残差・中間音声出力を WAV のみと正式仕様化し、FLAC 出力を現行要件から外した。
fused 実行と独立 events/candidates、空保存、manifest/resume 境界、Float64 PCM と来歴、
Resolution.sample_rate の既存契約を維持する。入力 decode と出力 encode は独立。
将来の圧縮は容量・保存時間・I/O の実測と PCM 精度保持を条件に別 Issue で検討する。
今回の承認/適用/検証は [今回task](../tasks/spec-residual-contract-finalization.md) に記録する。
旧taskのdoneは今回検証を意味しない。

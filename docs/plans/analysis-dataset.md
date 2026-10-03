---
id: "PLAN-034z7Go0WvShlVKkU7NCCH"
type: "plan"
status: "completed"
title: "Analysis Dataset ストアの実装"
created: "2026-10-02"
updated: "2026-10-03"
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
    - "docs/specs/analysis-dataset.md"
  implemented-by: []
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related: []
  refines: []
  refined-by: []
  derives-from:
    - "docs/specs/analysis-dataset.md"
    - "docs/designs/dataset-schema.md"
    - "docs/adr/analysis-dataset-storage.md"
  derived-by: []
  verifies: []
  verified-by:
    - "docs/test-specs/dataset-store-roundtrip.md"
    - "docs/test-specs/dataset-provenance-validity.md"
  references: []
  defers: []
  deferred-by: []
---

# Analysis Dataset ストアの実装 Implementation Plan

## Goal

`al-dataset` クレートに Analysis Dataset の保存レイヤ
(manifest + ロール別ストア + 来歴)を実装し、`Dataset` ステージ
として runner に接続する。

## Scope

本プランは 1 つの実装ストリーム。対象はデータセットの保存契約と
Dataset ステージまで。上流解析ステージ(locate / 解析)はまだ存在
しないため、Dataset ステージは現時点で存在する成果物
(declared inputs + ingest 成果物)を正規化して `<job_dir>/analysis/`
に物化する。

データセットルートは `<job_dir>/analysis/` とする(ジョブの
`manifest.json` とは別物: `<job_dir>/analysis/manifest.json`。
ingest が書く `<job_dir>/inputs/` とも別階層)。物化ポリシー:
宣言入力はコピーせず `source_path` + `source_sha256` の参照のみを
dataset manifest に記録し、`analysis/inputs/` には ingest が
デコードした PCM を WAV 音声として格納する(データセットの
「デコード済み音声」ロール)。大容量ソースファイルの複製は行わない。

- In scope: データセット manifest スキーマ、表形式ストア
  (列指向チャンク、範囲読み)、チャンクテンソルストア
  (Zarr 相当、部分読み出し)、WAV 音声ストア、provenance 記録
  (入力 id・algorithm id/version・パラメータ・alignment id・
  親子系譜)、validity セマンティクス、`dataset-writer` /
  `dataset-reader` コンポーネント、`DatasetStage` 実装と CLI 接続
- Out of scope: 各特徴の算出式(locate/解析ステージ側)、
  可視化向けフォーマット(visualization 側)、真の Parquet /
  FLAC コーデック(下記の形式選定を参照)

## Upstream Documents

- `docs/specs/analysis-dataset.md`
- `docs/adr/analysis-dataset-storage.md`

## Design Inputs

- `docs/designs/dataset-schema.md`
- `docs/designs/overview.md`

## 形式選定(ADR の deferred concern の決定)

ADR は「Parquet 表形式 + チャンクテンソル(Zarr 相当)」を選択し、
具体的実装を実装フェーズに委ねた。本プランではワークスペースの
方針(外部依存を増やさない。serde/serde_json のみ相互運用目的で
許可)に従い、契約を満たす最小の自前形式を実装する:

- 表形式: テーブルごとのディレクトリ(`schema.json` + カラム別
  チャンクの little-endian バイナリ + チャンクインデックス)。
  行範囲 / カラム選択で全体読み込みなしに読める。
  カラム物理エンコーディング: 固定幅数値は LE のまま、可変長
  (文字列・バイト列)は `u32 length + payload`、NULL はチャンク
  先頭の null ビットマップ(1bit/行)で表す。チャンクインデックスは
  `row_start`/`row_count` → チャンクファイル名の対応を持ち、
  行範囲クエリはインデックス経由で該当チャンクのみ読む
- テンソル: 配列ごとのディレクトリ(`array.json` に shape/dtype/
  chunk_shape/order + チャンク座標ファイルの raw LE)。チャンク
  単位の部分読み出し。チャンク粒度の初期デフォルト: テーブルは
  65_536 行/チャンク、テンソルは ~4 MiB/チャンクを目安に
  chunk_shape を算出。いずれも config で上書き可能とし、
  実データでの計測による調整は運用フェーズに defer
  (design の deferred concern の扱いどおり)
- 音声: WAV(PCM)。読み出しは既存 `al-media::wav` を再利用、
  書き込みは同モジュールにエンコーダを追加。FLAC は非可逆でない
  圧縮コーデックの自前実装が大きいため本プランでは持たず、
  `audio_format: "wav"` を manifest に記録する(Open Questions
  に明記)。エンコードは全バッファを一度に作らず、サイズ未確定の
  RIFF ヘッダを書いてからサンプルをストリーミング書き出し、
  最後にサイズをシークパッチして fsync + rename とする
  (大容量キャプチャでメモリを倍加させない)

形式変更時はリリース前ポリシーどおり互換層を持たず作り直す。

## File Map

- `crates/al-dataset/Cargo.toml` - データセットクレート
  (al-core / al-job / al-media 依存)
- `crates/al-dataset/src/lib.rs` - 公開 API(writer/reader/types)
- `crates/al-dataset/src/manifest.rs` - データセット manifest
  (schema_version・inputs・config・role 別成果物・生成経緯。
  各ロールに `produced` / `not_produced` + 件数の明示ステータスを
  持ち、「何も生成しなかった」と「上流が未実装」を区別する)
- `crates/al-dataset/src/types.rs` - 結果形状(FeatureRecord)。
  spec の結果形状を 1:1 で持つ: 特徴名/スキーマ id、値または
  テンソル位置(tensor の場合は配列パス + 座標範囲)、単位、
  時間範囲、周波数範囲、チャンネル/チャンネルペア、
  分解能/window/hop、絶対対象値、リファレンス値、差分/比率、
  確度、validity、無効/低確度の理由、サポート数/有効窓数、
  algorithm id/version、パラメータ、input id、alignment id、
  親/中間物の系譜
- `crates/al-dataset/src/table.rs` - 列指向チャンク表ストア
  (schema + カラム別チャンク + 範囲読み)
- `crates/al-dataset/src/tensor.rs` - チャンクテンソルストア
  (array.json + チャンクファイル + 部分読み出し)
- `crates/al-dataset/src/audio.rs` - WAV 音声ストア
  (al-media のデコーダ + 新規エンコーダで往復)
- `crates/al-dataset/src/provenance.rs` - provenance レコード
  (input id・algorithm id/version・params・alignment id・親子)
- `crates/al-dataset/src/writer.rs` - dataset-writer(role 振り分け
  → 各ストア、durable write 統一)
- `crates/al-dataset/src/reader.rs` - dataset-reader(manifest 経由の
  範囲/チャンク/来歴参照)
- `crates/al-dataset/src/stage.rs` - `DatasetStage`(Stage trait)。
  `depends_on()` は `[StageKind::Ingest]` を返す(デフォルトの
  「先行ステージ全て」にしない。locate/解析ステージは未実装であり、
  存在する成果物だけを物化するのが契約のため、runner レベルの必須
  依存は Ingest のみ。各成果物の有無は実行時に manifest から確認)
- `crates/al-media/src/wav.rs` - WAV エンコーダ追加(f64 PCM →
  PCM s16/s24/s32/float 書き出し、ストリーミング + サイズ
  シークパッチ)
- `crates/al-core/src/types.rs` - `Validity` を spec の区分一式で
  **置き換える**(拡張ではない。`Unavailable`/`Degraded` は廃止し、
  語彙の重複を防ぐ): `Valid` / `SilenceInsufficientEnergy` /
  `InsufficientSupport` / `LowCorrelation` / `LowCoherence` /
  `AmbiguousAlignment` / `OutOfSupportedFrequencyRange` /
  `DecodeInputUnavailable` / `NotApplicable` /
  `ComputationFailed`。既存単体テスト
  `validity_is_distinct_from_absence` も新 variants で更新する
- `crates/al-cli/src/main.rs` - `Dataset` スタブを実装に差し替え、
  USAGE の「他ステージはスタブ」記述を更新
- `crates/al-dataset/tests/` - 往復・部分読み出し・来歴の結合テスト
- `Cargo.toml` - workspace `members` / `workspace.dependencies` に
  `al-dataset` を追加
- `crates/al-cli/Cargo.toml` - `al-dataset.workspace = true` を追加

`<job_dir>/analysis/` レイアウト(spec の推奨どおり):
`manifest.json` / `inputs/` / `locate/` / `intermediate/` /
`features/{cheap,reference,expensive}/` / `residual/` / `events/` /
`visualization/`(上流未実装のロールは `not_produced` ステータスで
manifest に記録)

job manifest への artifact 登録粒度: 1 ArtifactRecord =
1 ストアルート(テーブルディレクトリ / テンソル配列ディレクトリ /
WAV ファイル / `analysis/manifest.json`)。チャンクファイル群は
ストア内部の `schema.json`/`array.json` が保持し、個別 artifact
にはしない(manifest 肥大化防止)

## Implementation Sequence

- [ ] `al-core::Validity` を spec の区分一式に拡張する
- [ ] `al-dataset` クレート骨格と結果形状・manifest 型を実装
- [ ] 列指向チャンク表ストアを実装(書き込み + 範囲読み)
- [ ] チャンクテンソルストアを実装(書き込み + 部分読み出し)
- [ ] WAV エンコーダと音声ストアを実装(往復)
- [ ] provenance レコードと dataset-writer(role 振り分け)を実装
- [ ] dataset-reader(manifest 経由の参照 API)を実装
- [ ] `DatasetStage` を実装して `al-cli` に接続
- [ ] 往復・部分読み出し・来歴の結合テストを追加

## Dependencies and Risks

- 依存: `al-core` / `al-job`(fsutil の durable write を再利用)/
  `al-media`(WAV デコード)。外部クレートは追加しない
- リスク: 上流解析成果物が未だ存在せず features/events が空 →
  Dataset ステージは「存在する成果物の物化」に限定し、空ロールは
  ディレクトリのみ生成。解析ステージ実装時に writer の対象を拡張
- リスク: 自前形式が真の Parquet/Zarr との相互運用を持たない →
  spec は「Zarr 相当」を許容し、外部ツール連携は export 側の
  責務とする。移行が必要になった場合は形式変更=作り直しで対応
- リスク: `Validity` 拡張は破壊的変更 → リリース前ポリシーで許容。
  現在 Validity を実値として使うコードはない

## Task Breakdown

- `al-core` Validity 拡張と `al-dataset` 骨格・結果形状・manifest
- 列指向チャンク表ストア
- チャンクテンソルストア
- WAV エンコーダと音声ストア
- provenance + dataset-writer
- dataset-reader
- `DatasetStage` 実装と CLI 接続
- 結合テスト(往復・部分読み出し・来歴・manifest)

## Verification

- [ ] `mise run test` が全 crate で通る
- [ ] manifest から各成果物の生成経緯(input・algorithm・params・
  alignment・親)を一意に辿れる
- [ ] 大規模テンソルを全体読み込みなしにチャンク / 範囲で読める
- [ ] 表を行範囲 / カラム選択で部分読みできる
- [ ] 保持が要求される中間データがスカラー要約に置き換わらない
- [ ] スキーマバージョンとアルゴリズムバージョンが別フィールドで
  管理される
- [ ] 「計測不能」と「異常なし」が異なる validity で表現される
- [ ] `al run --base-dir <dir> --input capture_bug:<wav> --stages
  ingest,dataset` で `<job_dir>/analysis/` が物化される

## Open Questions / Gaps

- FLAC コーデックは持たない(音声ロールは WAV のみ。必要になった
  時点で形式変更=作り直し)
- 真の Parquet/Zarr 相互運用は export プランで再評価
- 上流解析ステージ未実装のため、features/locate/events 等の実
  データ投入は後続プランに依存

## Self-Review

- [ ] Every spec requirement maps to a task.
- [ ] No placeholders remain.
- [ ] File paths, relations, status values, and terminology are consistent.

## Implementation Handoff

- Recommended mode: inline execution(単一ストリーム、依存順が明確)
- Independent work streams: なし
- User approval required before dispatch: yes
- If approved: discover available implementation and delegation capabilities in the current environment, then execute task-by-task with review checkpoints.
- If declined or unavailable: execute inline using the same dependency graph and verification matrix.

## Review Handoff

- 自前の列指向/テンソル形式を「Parquet / Zarr 相当」として採用する
  判断(spec は「相当」を許容、ADR は選定を実装に委任)を確認
- FLAC を持たない範囲設定の妥当性を確認
- 上流未実装ロールを空ディレクトリとして物化する判断を確認

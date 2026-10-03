---
id: "PLAN-034z8FkQ8VWjuojrvS0HhT"
type: "plan"
status: "approved"
title: "visualization-export"
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
  implements:
    - "docs/specs/visualization-dataset.md"
    - "docs/specs/analysis-export.md"
  implemented-by: []
  depends-on:
    - "docs/plans/analysis-stages.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related: []
  refines: []
  refined-by: []
  derives-from:
    - "docs/specs/visualization-dataset.md"
    - "docs/specs/analysis-export.md"
    - "docs/designs/visualization-export.md"
    - "docs/adr/derived-visualization-dataset.md"
  derived-by: []
  verifies: []
  verified-by:
    - "docs/test-specs/visualization-export.md"
  references: []
  defers: []
  deferred-by: []
metadata:
  experiments:
    adopted: []
    rejected: []
---

# visualization-export Implementation Plan

## Goal

Analysis Dataset から派生する可視化プロダクト(LOD / タイル /
ビューデータ)、人間可読サマリ、移設可能な成果物パッケージを
`al-viz` クレートとして実装し、Visualization / Export ステージを
`al-cli` に接続する。

## Scope

1 つの実装ストリーム。`al-viz` クレート新設。派生層は新しい
診断セマンティクスを追加せず、dataset の計測を表示可能な形に
変換するだけに留める(derived-visualization-dataset ADR)。

analysis-stages プランの「Dataset 追記プロトコル」
(open_or_create / 置換 / 冪等 / locate ロール物化)に依存する。

## Upstream Documents

- `docs/specs/visualization-dataset.md`
- `docs/specs/analysis-export.md`
- `docs/specs/analysis-dataset.md`(入力契約)
- `docs/specs/validation-strategy.md`

## Design Inputs

- `docs/designs/visualization-export.md`(approved)
- `docs/adr/derived-visualization-dataset.md`
- `docs/adr/analysis-dataset-storage.md`
- `docs/plans/analysis-stages.md`(追記プロトコル・成果物名契約)

## 出力先とスキーマ

- 可視化プロダクトは **`analysis/visualization/` ロール**に書く
  (analysis-dataset spec のレイアウトどおり。dataset manifest に
  `visualization` ロールは既に宣言済み)
- `open_or_create` で analysis dataset を開き、dataset 自身の
  `SCHEMA_VERSION` に従う(独立バージョンは持たない。
  不一致は reader/writer 側の既存検証で明示的失敗)
- VisualizationStage の再実行は `visualization/` ロール配下を
  置換セマンティクスで上書きする(他ロールは触らない。
  部分残骸も同じ規則で上書きされる)
- **`skipped_products` 記録**: 必須ロールが `not_produced` の
  プロダクトは、viz manifest 書き込み時に
  `{product, required_role, reason}` として `visualization/
  skipped.json` に記録する。「未実行」と「バグで欠落」を
  区別できるようにする
- produced ロールに期待成果物が無い場合は契約違反として
  **明示的エラー**(静かな空プロダクトにしない)

## クロスプラン成果物名契約

viz が読む dataset 成果物名(analysis-stages プランが供給):

- `intermediate/stft_<input_id>` — 複素 STFT tensor
  [frame, bin, {re,im}] または分解済み magnitude/power
- `intermediate/psd_<input_id>` — PSD tensor
- `intermediate/bands_<input_id>` — 固定帯域 / 1/3oct パワー
- `features/cheap/*` — 特徴 table
- `features/reference/*` — lag / gain / band / transfer / channel
- `features/expensive/*` — expensive 解析結果
- `residual/*` — R0〜R3 波形 + 付随観測
- `events/events` — イベント table(安定 id 列を持つ)
- `locate/<src>/<cap>.json` — LocateResult + time_map +
  occurrences(DatasetStage が物化)
- `inputs/<id>.wav` — 物化済み PCM(LOD の対象)

いずれかのロールが `not_produced` なら対応プロダクトは
skipped 記録付きで省略する。

## 共有タイムライン

`visualization/timeline.json` を最初に書く — 全系列が従う共通
キャプチャタイムベース:

```json
{
  "sample_rate": 48000,
  "capture_samples": 123456,
  "series": {
    "waveform_lod":   {"unit": "bucket", "buckets_per_second": [...]},
    "spectrogram":    {"window": 1024, "hop": 256},
    "band_heatmap":   {"window": 2048, "hop": 512},
    "events":         {"unit": "sample"}
  }
}
```

各プロダクトはこの宣言に一致する window/hop/オフセットで書き、
コンシューマが再推定せずにアラインできる。

## ステージ依存

- `VisualizationStage::depends_on() = [Dataset]`(DatasetStage の
  `[Ingest]` 前例と同じく、存在する成果物だけを派生する)
- `ExportStage::depends_on() = [Dataset, Visualization]`

## 決定性の範囲

- viz / export の**ファイル内容**は dataset 内容 +
  `SCHEMA_VERSION` + config のみに依存する
- provenance の `parents` は dataset 安定識別子(alignment_id、
  dataset 相対パス、イベント id)を使い、新規発行の
  `ArtifactId`(時刻/PID シード)は dataset 内部に埋め込まない
- ジョブ manifest 側の `ArtifactRecord.artifact_id` 自体は
  再生成で変わり得る — 決定性の対象は「ファイル内容」であり
  job manifest の id 列は対象外と明記する
- サマリ・パッケージ内のパスは**パッケージ相対**のみ。
  タイムスタンプ・絶対パス・環境値は埋め込まない
- golden スナップショットは正規化済みシリアライズ内容の
  ハッシュで比較する

## パッケージ可搬性

- `export/<name>/` に `analysis/` + `viz` 成果物を**コピー**で
  同梱(中間スペクトル・テンソル・残差音声は参照でなくコピー。
  移設後も読める)
- 元ソースメディアは hash + ファイル名参照のみ(複製は
  `config.export.copy_media` で opt-in)
- **パッケージ manifest は変換して書く**: `source_path` /
  `source_path_abs` をファイル名+sha256 に縮約し、`config` 内の
  絶対パス(dictionary 等)を除去する。検証として「export ツリー
  内に絶対パスを含む文字列がない」ことを確認する
- `export_manifest.json` に package schema_version・生成器
  id/version・同梱ロール一覧・入力ハッシュ一覧を記録する

## File Map

- `Cargo.toml` - workspace members / dependencies に `al-viz` を追加
- `crates/al-cli/Cargo.toml` - `al-viz.workspace = true` を追加
- `crates/al-viz/Cargo.toml` - 可視化/エクスポートクレート
  (al-core / al-job / al-dataset / **al-locate**(LocateResult /
  TimeMap 等の型)/ al-media / serde / serde_json)
- `crates/al-viz/src/lib.rs` - 公開 API
- `crates/al-viz/src/timeline.rs` - 共有タイムライン記述子の
  生成と検証
- `crates/al-viz/src/lod.rs` - 波形 LOD ピラミッド
  (min / max / RMS、対象・リファレンス・残差の各レベル。
  **チャンク読み** — `DatasetReader` に範囲読みを追加し
  全ファイルをメモリに載せない)
- `crates/al-viz/src/tiles.rs` - スペクトログラムタイル
  (対象 / アライン済みリファレンス / 差分 dB / 残差 R0〜R3 /
  コヒーレンス + validity マスク)、帯域ヒートマップ
  (時間×帯域: ゲイン / 相関 / コヒーレンス / 残差エネルギー)
- `crates/al-viz/src/views.rs` - 伝達ビュー(magnitude / 位相 /
  群遅延 / コヒーレンス + 選択範囲の頑健要約)、
  locate/time-map ビュー(source vs capture / anchors /
  区分セグメント / lag / 時間スケール / 確度 /
  skip・resume・unmatched / ソース出現)、ステレオビュー
  (L/R 相関 / M-S 比 / 帯域別幅 / IPD / ILD / 複素コヒーレンス /
  source→output 応答 / 短範囲ゴニオメータ用データ)、
  残差ビュー(capture / aligned ref / prediction / R0〜R3 の
  波形・スペクトログラム・**PSD・帯域エネルギー** の各形)、
  イベント・候補マーカー(安定イベント id リンク)
- `crates/al-viz/src/summary.rs` - 決定的な人間可読サマリ
  (inputs / 実行ステージ / aligned ref / valid・invalid カバレッジ /
  主要計測範囲 / **候補数・イベント数** / 出力パス[相対]。
  根本原因の推定はしない)
- `crates/al-viz/src/package.rs` - パッケージライタ + manifest
  変換(絶対パス除去)+ export_manifest.json
- `crates/al-viz/src/golden.rs` - validation スナップショット
  (区間指定 JSON、内容ハッシュ比較)
- `crates/al-viz/src/stage.rs` - `VisualizationStage` /
  `ExportStage`(上記 depends_on)
- `crates/al-dataset/src/reader.rs` - `read_audio_range` 等の
  範囲読み追加(LOD / ゴニオメータのためのチャンクアクセス)
- `crates/al-cli/src/main.rs` - 2 ステージを実装に差し替え、
  USAGE 更新
- `crates/al-viz/tests/` - 結合テスト

## spec → プロダクト対応表

| spec 要求 | プロダクト |
|---|---|
| 共有タイムライン | `visualization/timeline.json` |
| 映像位置 | `skipped_products` に「video 系列は未物化」として記録(入力側に video 系列の保存契約が無いため) |
| 波形(対象/ref/残差) | `visualization/lod/*` |
| ソース出現 / locate 確度 / lag / time map | `visualization/views/locate_*` |
| ゲイン / 帯域メトリクス | `visualization/tiles/band_*` |
| 残差エネルギー | `visualization/tiles/residual_*` |
| イベント / 候補 | `visualization/views/events` |
| スペクトログラム(対象/ref/差分/残差/コヒーレンス) | `visualization/tiles/*` + validity マスク |
| 伝達ビュー | `visualization/views/transfer_*` |
| ステレオ/チャンネル | `visualization/views/stereo_*` |
| 残差 R0-R3 各形 | `visualization/views/residual_*` |
| サマリ | `export/summary.txt`(+ `analysis/visualization/summary` コピー) |
| パッケージ | `export/<name>/` + `export_manifest.json` |
| golden | `export/golden/*.json` |

## Implementation Sequence

- [ ] `al-viz` 骨格 + `timeline.rs` + `read_audio_range`
- [ ] `lod.rs`: 波形 LOD ピラミッド(チャンク読み)
- [ ] `tiles.rs`: スペクトログラムタイル + 帯域ヒートマップ
- [ ] `views.rs`: 伝達 / time-map / ステレオ / 残差 / イベントビュー
- [ ] `summary.rs`: 人間可読サマリ
- [ ] `package.rs`: 可搬パッケージ + manifest 変換
- [ ] `golden.rs`: validation スナップショット
- [ ] `stage.rs` + `al-cli` 配線 + USAGE 更新
- [ ] 結合テスト(dataset→viz 再生成 / 可搬性 / 決定性 / resume)

## Dependencies and Risks

- 依存: `al-dataset`(reader / writer / storage 契約)、
  `al-locate`(結果型)、`al-media`(WAV)。外部クレートは追加しない
- 依存: analysis-stages プランの `intermediate` / `features/*` /
  `events` / `residual` / `locate` ロールと成果物名契約。
  `depends-on: analysis-stages.md` を宣言済み
- リスク: 派生データの肥大 → LOD / タイル化 + 元計測への参照
- リスク: 決定性 → 上記「決定性の範囲」セクションどおり、
  dataset 安定識別子のみ使用
- リスク: 意味の作り込み → 診断語彙を追加せず、confidence /
  validity / 元計測参照をそのまま保持
- リスク: 元メディア欠落でパッケージが読めなくなる → 数値 /
  テンソルはコピー同梱、元メディアは hash + 名前参照のみ
- リスク: 部分残骸で resume 不能 → `visualization/` / `export/`
  配下を置換セマンティクスで上書き(export は dir 単位で
  作り直し)

## Task Breakdown

- `al-viz` 骨格 + timeline 記述子 + `read_audio_range`
- 波形 LOD ピラミッド
- スペクトログラムタイル + 帯域ヒートマップ
- 伝達 / time-map / ステレオ / 残差 / イベントビュー
- 人間可読サマリ
- 可搬パッケージライタ + manifest 変換
- validation golden エクスポート
- Visualization/Export ステージ + CLI 接続
- 結合テスト

## Verification

- [ ] `mise run test` / `cargo test --workspace` 全 pass
  (`cargo metadata` で `al-viz` が member として解決される)
- [ ] `cargo clippy --workspace --all-targets -- -D warnings` /
  `cargo fmt --all -- --check` clean
- [ ] **populated fixture**(features/events/residual/locate が
  埋まった analysis dataset)から viz データセットを再生成できる
  — メディアデコード・特徴抽出を伴わない
- [ ] LOD / タイルで対象が生 PCM 非参照で描画可能(LOD は
  `read_audio_range` で構築される)
- [ ] 派生データが confidence / validity / 元計測参照を保持する
- [ ] `skipped_products` が必須ロール未生成のプロダクトを
  理由付きで記録する
- [ ] 同一 dataset → 同一ファイル内容(バイト比較、job manifest
  の新規 id を除く)
- [ ] サマリが決定的で根本原因を推定しない
- [ ] パッケージが相対パスのみ、hash で入力同一性を検証でき、
  元メディア欠落でも数値 / テンソルが読める。export ツリー内に
  絶対パスを含む文字列がない
- [ ] `al run --stages ingest,locate,dataset,visualization,export`
  が完走し `visualization/` / `export/` 成果物が登録される
  (analysis ステージなしの選択も `depends_on` で受理される)
- [ ] viz ステージ失敗後の resume で `visualization/` の部分
  出力が上書きされ完走する

## Open Questions / Gaps

- 図版フォーマット(ラスタ画像の要否)→ 現状はデータのみ出力し、
  図版化はコンシューマ側(deferred design concern)
- fixture 抽出の最小単位 → golden.rs は区間指定の JSON 形式で
  実装時に確定
- `inputs` WAV の LOD 構築は `read_audio_range` で chunk 読み。
  十分大きな入力でのメモリ上限は実装時に確定(定数チャンク)

## Self-Review

- [x] Every spec requirement maps to a task.
- [x] No placeholders remain.
- [x] File paths, relations, status values, and terminology are consistent.

## Implementation Handoff

- Recommended mode: inline execution
- Independent work streams: none(逐次依存、analysis-stages 後)
- User approval required before dispatch: no(review-protocol で
  P0/P1 ブロッカー解消をもって承認とみなす)

## Review Handoff

- review-protocol レビュー(Reviewer A 10 件・Reviewer B 12 件の
  候補)で指摘されたブロッカーをすべて反映済み:
  locate ロール物化(analysis-stages 側で対応)、viz 出力先
  (`analysis/visualization/`)+ shared timeline、depends_on 明示、
  決定性の範囲と id ポリシー、絶対パス除去の manifest 変換、
  skipped_products 記録、spec 抜け(映像位置 / PSD・帯域残差 /
  候補数 / イベント id 安定性)、範囲読み、al-locate 依存追加、
  populated fixture による検証、resume 置換

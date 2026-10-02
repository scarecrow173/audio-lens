---
id: "PLAN-034z8GHrYvN6v2lYjA4vY4"
type: "plan"
status: "draft"
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
  depends-on: []
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
  verified-by: []
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

Analysis Dataset から派生する Visualization Dataset(LOD /
タイル / ビューデータ)、人間可読サマリ、移設可能な成果物
パッケージを `al-viz` クレートとして実装し、Visualization /
Export ステージを `al-cli` に接続する。

## Scope

1 つの実装ストリーム。`al-viz` クレート新設。派生層は新しい
診断セマンティクスを追加せず、dataset の計測を表示可能な形に
変換するだけに留める(derived-visualization-dataset ADR)。

## Upstream Documents

- `docs/specs/visualization-dataset.md`
- `docs/specs/analysis-export.md`
- `docs/specs/analysis-dataset.md`(入力契約)
- `docs/specs/validation-strategy.md`

## Design Inputs

- `docs/designs/visualization-export.md`(approved)
- `docs/adr/derived-visualization-dataset.md`
- `docs/adr/analysis-dataset-storage.md`

## File Map

- `Cargo.toml` - workspace members / dependencies に `al-viz` を追加
- `crates/al-cli/Cargo.toml` - `al-viz.workspace = true` を追加
- `crates/al-viz/Cargo.toml` - 可視化/エクスポートクレート
  (al-core / al-job / al-dataset / al-media / serde / serde_json)
- `crates/al-viz/src/lib.rs` - 公開 API
- `crates/al-viz/src/lod.rs` - 波形 LOD ピラミッド
  (min / max / RMS、対象・リファレンス・残差の各レベル)
- `crates/al-viz/src/tiles.rs` - スペクトログラムタイル
  (対象 / アライン済みリファレンス / 差分 dB / 残差 /
  コヒーレンス + validity マスク)、帯域ヒートマップ
  (時間×帯域: ゲイン / 相関 / コヒーレンス / 残差エネルギー)
- `crates/al-viz/src/views.rs` - 伝達ビュー(magnitude / 位相 /
  群遅延 / コヒーレンス + 頑健要約)、locate/time-map ビュー
  (source vs capture / anchors / 区分セグメント / lag /
  時間スケール / 確度 / skip・resume・unmatched)、
  ステレオビュー(L/R 相関 / M-S 比 / 帯域別幅 / IPD / ILD /
  複素コヒーレンス / source→output 応答 / ゴニオメータ用
  短範囲データ)、残差ビュー(capture / aligned ref /
  prediction / R0〜R3)、イベントマーカー(event id リンク)
- `crates/al-viz/src/dataset.rs` - Visualization Dataset writer:
  `viz/` 配下に schema_version 付きで LOD / タイル / ビューを
  tensor / table / json として保存(dataset の storage 契約を再利用)
- `crates/al-viz/src/summary.rs` - 決定的な人間可読サマリ
  (inputs / 実行ステージ / aligned ref / valid・invalid カバレッジ /
  主要計測範囲 / イベント数 / 出力パス。根本原因の推定はしない)
- `crates/al-viz/src/package.rs` - パッケージライタ:
  `export/` 配下に manifest + 入力ハッシュ/メタデータ +
  locate 結果 + features + 中間スペクトル参照 + 残差音声 +
  イベント + visualization dataset + サマリを再配置可能
  (相対パスのみ)に出力。大きな元メディアは hash/path 参照
  (複製は config ポリシー)
- `crates/al-viz/src/golden.rs` - validation-exporter:
  fixture / replay 用の golden スナップショットを
  バージョン付きで切り出し
- `crates/al-viz/src/stage.rs` - `VisualizationStage` /
  `ExportStage`(Stage trait)
- `crates/al-cli/src/main.rs` - 2 ステージを実装に差し替え、
  USAGE 更新
- `crates/al-viz/tests/` - 結合テスト

## Implementation Sequence

- [ ] `al-viz` 骨格 + `dataset.rs`(viz dataset writer +
  schema_version)
- [ ] `lod.rs`: 波形 LOD ピラミッド
- [ ] `tiles.rs`: スペクトログラムタイル + 帯域ヒートマップ
- [ ] `views.rs`: 伝達 / time-map / ステレオ / 残差 / イベントビュー
- [ ] `summary.rs`: 人間可読サマリ
- [ ] `package.rs`: 可搬パッケージ + 入力 hash 参照 +
  schema_version
- [ ] `golden.rs`: validation スナップショット
- [ ] `stage.rs` + `al-cli` 配線 + USAGE 更新
- [ ] 結合テスト(dataset→viz 再生成 / 可搬性 / 決定性)

## Dependencies and Risks

- 依存: `al-dataset`(DatasetReader / writer / storage 契約)、
  `al-media`(WAV)。外部クレートは追加しない
- 依存: analysis-stages プランの features / events / residual
  ロール。未生成ロールは `not_produced` のままビューをスキップ
  (失敗ではなく空の派生として記録)
- リスク: 派生データの肥大 → LOD / タイル化 + 元計測への
  参照 id(再計算しない)
- リスク: 決定性 → 出力は dataset 内容と schema_version のみに
  依存し、タイムスタンプ / 絶対パス / 環境値を埋め込まない
- リスク: 意味の作り込み → 派生層は診断語彙を追加せず、
  confidence / validity / 元計測参照をそのまま保持
- リスク: ソースメディア欠落でパッケージが読めなくなる →
  数値 / テンソルは dataset 内コピーを出力し、元メディアは
  hash + 相対参照のみ
- リスク: dataset スキーマとの不整合 → viz dataset も
  schema_version を明示し、不一致は明示的失敗(互換層なし)

## Task Breakdown

- `al-viz` 骨格 + Visualization Dataset writer
- 波形 LOD ピラミッド
- スペクトログラムタイル + 帯域ヒートマップ
- 伝達 / time-map / ステレオ / 残差 / イベントビュー
- 人間可読サマリ
- 可搬パッケージライタ
- validation golden エクスポート
- Visualization/Export ステージ + CLI 接続
- 結合テスト

## Verification

- [ ] `mise run test` / `cargo test --workspace` 全 pass
  (`cargo metadata` で `al-viz` が member として解決される)
- [ ] `cargo clippy --workspace --all-targets -- -D warnings` /
  `cargo fmt --all -- --check` clean
- [ ] Visualization Dataset が Analysis Dataset のみから再生成できる
  (メディアデコード / 特徴抽出を伴わない)
- [ ] LOD / タイルで長尺キャプチャが生 PCM 非参照で描画可能
- [ ] 派生データが confidence / validity / 元計測参照を保持する
- [ ] サマリが決定的(同一 dataset → 同一出力)で根本原因を
  推定しない
- [ ] パッケージが相対パスのみで移設可能、hash で入力同一性を
  検証でき、元メディア欠落でも数値 / テンソルが読める
- [ ] schema_version 不一致が明示的失敗になる
- [ ] `al run --stages …,dataset,visualization,export` が
  完走し `viz/`・`export/` 成果物が manifest に登録される

## Open Questions / Gaps

- 図版フォーマット(ラスタ画像の要否)→ 現状はデータのみ出力し、
  図版化はコンシューマ側(deferred design concern)
- fixture 抽出の最小単位 → golden.rs は区間指定の JSON 形式で
  実装時に確定

## Self-Review

- [x] Every spec requirement maps to a task.
- [x] No placeholders remain.
- [x] File paths, relations, status values, and terminology are consistent.

## Implementation Handoff

- Recommended mode: inline execution
- Independent work streams: none(逐次依存)
- User approval required before dispatch: no(review-protocol で
  P0/P1 ブロッカー解消をもって承認とみなす)

## Review Handoff

- 注目点: 派生データの意味保存(confidence/validity/参照)、
  決定性(環境値を含めない)、パッケージ可搬性(相対パス /
  hash 参照 / メディア欠落耐性)、analysis-stages との
  ロール名整合

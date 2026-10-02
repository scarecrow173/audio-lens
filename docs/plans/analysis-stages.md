---
id: "PLAN-034z8FkpJb4ejhkehZ3pgc"
type: "plan"
status: "draft"
title: "analysis-stages"
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
    - "docs/specs/cheap-analysis-features.md"
    - "docs/specs/reference-analysis.md"
    - "docs/specs/expensive-analysis.md"
    - "docs/specs/candidate-event-model.md"
  implemented-by: []
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related: []
  refines: []
  refined-by: []
  derives-from:
    - "docs/specs/cheap-analysis-features.md"
    - "docs/specs/reference-analysis.md"
    - "docs/specs/expensive-analysis.md"
    - "docs/specs/candidate-event-model.md"
    - "docs/designs/analysis-stages.md"
    - "docs/adr/measurement-over-verdict.md"
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

# analysis-stages Implementation Plan

## Goal

cheap / reference / expensive の 3 解析ステージと、それらを束ねる
candidate / event モデル + 確度伝播を `al-analysis` クレートとして
実装し、全観測を Analysis Dataset に保存する。

## Scope

1 つの実装ストリーム。`al-analysis` クレート新設 + DatasetWriter の
追記オープン + DatasetStage の終了処理調整を含む。可視化 /
エクスポートは `visualization-export` プランに分離する。

境界: 解析ステージは「観測」のみ書き、イベントは
「interesting measurement」(確認済みバグではない)として
`strength` で記録する(measurement-over-verdict ADR)。

## Upstream Documents

- `docs/specs/cheap-analysis-features.md`
- `docs/specs/reference-analysis.md`
- `docs/specs/expensive-analysis.md`
- `docs/specs/candidate-event-model.md`
- `docs/specs/analysis-dataset.md`(データセット契約)
- `docs/specs/validation-strategy.md`(fixture 要件)

## Design Inputs

- `docs/designs/analysis-stages.md`(approved)
- `docs/adr/measurement-over-verdict.md`
- `docs/adr/analysis-dataset-storage.md`

## File Map

- `Cargo.toml` - workspace members / dependencies に `al-analysis` を追加
- `crates/al-cli/Cargo.toml` - `al-analysis.workspace = true` を追加
- `crates/al-analysis/Cargo.toml` - 解析クレート
  (al-core / al-job / al-dataset / al-locate / al-media / serde /
  serde_json 依存。DSP プリミティブは `al-locate::dsp` を再利用)
- `crates/al-analysis/src/lib.rs` - 公開 API
- `crates/al-analysis/src/stft.rs` - 複素 STFT / magnitude / power /
  PSD / 帯域パワー / 1/3 オクターブ / スペクトル包絡 / CSD /
  コヒーレンス(`al-locate::dsp::fft` 再利用)
- `crates/al-analysis/src/records.rs` - `FeatureRecord` への
  マッピング、標準解像度クラス(spec の 7 クラス: sample / ~5ms /
  ~20ms / ~100ms / ~500ms / ~1-5s / segment)、頑健要約
  (median / MAD / min / max / p05 / p25 / p75 / p95 / 有効率 /
  サポート数)
- `crates/al-analysis/src/cheap.rs` - cheap 特徴群:
  レベル(ピーク / true peak 推定 / RMS / DC / クレスト / ラウドネス
  近似 / レベル軌跡)、スペクトル形状(セントロイド / 帯域幅 /
  ロールオフ / 傾斜 / 平坦度 / エントロピー / フラックス /
  狭帯域ピーク・ノッチ候補)、ステレオ(ch レベル / ペア相関 /
  複素コヒーレンス / M-S パワー / 帯域別 M-S / ILD / IPD)、
  過渡(オンセット強度・時刻 / 持続 / 自己相似ヒント)、
  デジタル異常(クリッピング率・持続 / ゼロラン / スタックラン /
  不連続・クリックスコア / 一致ブロック候補 / 量子化推定)、
  ノイズ・変調(ノイズフロア / トーン性ノイズ / 50-60Hz+高調波 /
  AM スペクトル)
- `crates/al-analysis/src/reference.rs` - reference 特徴群:
  ラグ τ(t) / 統計 / ドリフト / ステップ候補 / 時間スケール /
  ピーク幅 / 窓別確度、ゲイン(最小二乗 / 400Hz / RMS 比 /
  軌跡 / レベル対ゲイン)、帯域(1/3 オクターブ M2 ゲイン /
  400Hz 相対 / 帯域相関・コヒーレンス / PSD 比)、類似度
  (フルバンド相関 / 帯域相関 / PHAT 証拠 / 包絡類似度)、
  複素伝達(CSD / H(f) / H(f,t) / magnitude / アンラップ位相 /
  群遅延 / コヒーレンス・サポート)、チャンネル(混合行列 /
  応答差 / 帯域別 L/R 複素コヒーレンス[magnitude+位相] /
  M/S 変化 / 極性 / クロストーク)、ダイナミクス(オンセット
  オフセット / 強度比 / F0 比 / 持続比)、段階的残差 R0〜R3
- `crates/al-analysis/src/events.rs` - candidate / event モデル:
  event フィールド(start/end or point / ソース特徴 / strength /
  確度 / validity / 周波数・ch 範囲 / アナライザ・バージョン /
  裏付け特徴へのリンク)、確度伝播ルール(上流 validity / 確度が
  下流を制限)、candidate 抽出器(cheap / reference 観測→候補)
- `crates/al-analysis/src/expensive.rs` - expensive 解析:
  トリガ(candidate / high-residual / low-similarity /
  abnormal-timing / manual / always-run、理由を永続化)と
  解析ファミリ(精密伝達・カットオフ / コム・エコー / SRC /
  非線形 / ダイナミクス / タイミング・反復 / コーデック /
  ノイズ分解 / 高度チャンネル)
- `crates/al-analysis/src/stage.rs` - `CheapAnalysisStage` /
  `ReferenceAnalysisStage` / `ExpensiveAnalysisStage`
  (Stage trait。`analysis/` dataset を open-or-create し
  features/cheap・features/reference・features/expensive・
  events・residual ロールへ書き込む)
- `crates/al-dataset/src/writer.rs` - `DatasetWriter::open_or_create`
  / 重複登録の冪等化(再実行時の (role,path) 置換)
- `crates/al-dataset/src/stage.rs` - 既存 manifest が有効なら
  追記して終了処理のみ行う(inputs/declared/PCM は未登録時に追加)、
  manifest が無い / 不正な部分残骸のみ全消去して再生成
- `crates/al-cli/src/main.rs` - 3 ステージを実装に差し替え、
  USAGE 更新
- `crates/al-analysis/tests/` - 合成 fixture 結合テスト

## Implementation Sequence

- [ ] `al-analysis` 骨格 + `records.rs`(解像度クラス / 要約 /
  FeatureRecord 変換)
- [ ] `stft.rs`: 複素 STFT / PSD / 帯域 / CSD / コヒーレンスの
  中間スペクトル
- [ ] `DatasetWriter::open_or_create` + 冪等 register +
  DatasetStage の終了処理調整(先行変更、後段の前提)
- [ ] cheap 特徴(レベル / スペクトル形状)を実装し dataset へ保存
- [ ] cheap 特徴(ステレオ / 過渡 / 異常 / ノイズ)を追加
- [ ] reference 特徴(lag / gain / band / similarity)を実装
  (locate の受理アライメントを入力)
- [ ] reference 特徴(複素伝達 / チャンネル / R0〜R3 残差)を実装
  (`al-locate` の mixer / subtract を再利用)
- [ ] `events.rs`: event モデル + candidate 抽出 + 確度伝播
- [ ] `expensive.rs`: トリガ永続化 + 解析ファミリ実装
- [ ] 3 ステージの Stage 実装 + `al-cli` 配線 + USAGE 更新
- [ ] 合成 fixture 結合テスト

## Dependencies and Risks

- 依存: `al-locate::dsp`(FFT / NCC / bandpass / envelope /
  least_squares / condition_number)、`al-locate` の mixer /
  residual、`al-dataset` の DatasetWriter/Reader、`al-media` の
  decoded PCM。外部クレートは追加しない
- 依存: locate の受理アライメント(alignment_id)が reference /
  expensive の前提。未受理時は reference 特徴を validity で
  明示(`AmbiguousAlignment` 等)し推定を実行しない
- リスク: `analysis/` を複数ステージが追記する →
  `DatasetWriter::open_or_create` で manifest を永続化し、
  (role,path) 重複は置換(失敗再実行の冪等性)。部分残骸は
  DatasetStage ではなく「manifest 不在/破損時のみ」全消去
- リスク: 特徴量が多く設計詳細が spec レベル →
  パラメータ(window/hop/帯域)は `config.analysis.*` で
  上書き可能にし、全出力に algorithm id / version / params /
  resolution を記録
- リスク: expensive のコスト爆発 → candidate 駆動をデフォルト、
  always-run は明示 config のみ。候補区間のみに適用
- リスク: 確度伝播の不徹底 → 下流特徴は上流の validity / 確度を
  `min` で伝播し、invalid な上流を高確度の主張にしない
  (spec candidate-event-model の規範)
- リスク: true peak / F0 / ラウドネスの厳密規格準拠は scope 外 →
  「推定」として algorithm params に手法を記録し validity を付す
- リスク: decoded PCM はネイティブスケール → `sample_format` で
  float 域に正規化してから解析(locate と同じ規約)

## Task Breakdown

- `al-analysis` 骨格 + 解像度クラス / 要約 / FeatureRecord 変換
- STFT / PSD / 帯域 / CSD / コヒーレンス中間スペクトル
- DatasetWriter 追記オープン + DatasetStage 終了処理調整
- cheap: レベル + スペクトル形状
- cheap: ステレオ + 過渡 + デジタル異常 + ノイズ/変調
- reference: lag / gain / band / similarity
- reference: 複素伝達 / チャンネル / R0-R3 段階残差
- events: candidate / event モデル + 確度伝播
- expensive: トリガ + 解析ファミリ
- 3 ステージ Stage 実装 + CLI 接続
- 合成 fixture 結合テスト

## Verification

- [ ] `mise run test` / `cargo test --workspace` が全 crate で通る
  (`cargo metadata` で `al-analysis` が member として解決される)
- [ ] `cargo clippy --workspace --all-targets -- -D warnings` /
  `cargo fmt --all -- --check` clean
- [ ] cheap ステージがリファレンス素材なしで完走する
- [ ] 中間スペクトル(複素 STFT / PSD)が dataset に保存され、
  後段ステージが再計算なしに DatasetReader から再利用できる
- [ ] 特徴出力に分解能(クラス + 実 window/hop)、validity、
  algorithm id / version / params が含まれる
- [ ] 合成 fixture(既知ゲイン・EQ・ドリフト)で reference 特徴が
  既知値を回復する
- [ ] R0〜R3 残差が段階的にエネルギーを減らし、各段の説明量が
  下流から判断できる
- [ ] イベントが裏付け特徴へのリンクを持ち、低確度入力が
  高確度の下流イベントにならない
- [ ] expensive 結果がトリガ理由を記録し、手動範囲指定と
  always-run で実行できる
- [ ] 失敗後の resume で重複 artifact が増えない(冪等)
- [ ] `al run --stages ingest,locate,cheap_analysis,reference_analysis,
  expensive_analysis,dataset` が合成 WAV で完走し
  features/*・events・residual ロールが produced になる

## Open Questions / Gaps

- 1/3 オクターブ M2 の帯域数・窓の最終値 → fixture で調整
  (spec の deferred concern どおり)
- true peak のオーバーサンプル次数 → 実装時に 4x 線形補間を
  初期値として params に記録

## Self-Review

- [x] Every spec requirement maps to a task.
- [x] No placeholders remain.
- [x] File paths, relations, status values, and terminology are consistent.

## Implementation Handoff

- Recommended mode: inline execution
- Independent work streams: none(逐次依存)
- User approval required before dispatch: no(review-protocol で
  P0/P1 ブロッカー解消をもって承認とみなす)
- If approved: task-by-task に実装し、意味境界でコミット。

## Review Handoff

- 注目点: dataset 追記プロトコル(open_or_create + 冪等)と
  DatasetStage の既存データ取り扱い、確度伝播の具体ルール、
  cheap / reference の特徴網羅性、expensive トリガの永続化

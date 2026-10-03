---
id: "PLAN-034z8FkpJb4ejhkehZ3pgc"
type: "plan"
status: "approved"
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
  verified-by:
    - "docs/test-specs/analysis-stages.md"
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

1 つの実装ストリーム。`al-analysis` クレート新設 + `al-dataset`
の追記書き込みプロトコル + `al-locate` の空入力耐性 +
`FeatureRecord` スキーマ拡張を含む。可視化 / エクスポートは
`visualization-export` プランに分離する。

境界: 解析ステージは「観測」のみ書き、イベントは
「interesting measurement」(確認済みバグではない)として
`strength` で記録する(measurement-over-verdict ADR)。

## Upstream Documents

- `docs/specs/cheap-analysis-features.md`
- `docs/specs/reference-analysis.md`
- `docs/specs/expensive-analysis.md`
- `docs/specs/candidate-event-model.md`
- `docs/specs/analysis-dataset.md`(データセット契約)
- `docs/specs/analysis-job-model.md`(ステージ選択)
- `docs/specs/validation-strategy.md`(fixture 要件)

## Design Inputs

- `docs/designs/analysis-stages.md`(approved)
- `docs/adr/measurement-over-verdict.md`
- `docs/adr/analysis-dataset-storage.md`

## Dataset 追記プロトコル(本プランの中核設計)

複数ステージが `analysis/` dataset に逐次書き込むために、以下を
確立する(新規実装):

1. **`DatasetWriter::open_or_create`**: `manifest.json` があれば
   `schema_version`・`inputs`・`config` の完全一致を検証して開く
   (不一致はエラー。互換層・マージはしない)。無ければ
   `create`。create するステージは job manifest の宣言入力から
   `InputReference` を構築する(pinned sha256 必須)。この構築は
   `DatasetStage::run` 内の私有ロジックを共有関数
   `input_references(&Manifest) -> Result<Vec<InputReference>>`
   として切り出し、作成ステージ全てが使う。
2. **置換セマンティクス**: `write_table` / `write_tensor` /
   `write_audio` / `write_json` は対象 `role/name` の既存 store
   root があれば manifest 登録の有無に関わらず削除してから
   再作成する(失敗再実行の冪等性。部分残骸・孤立 store も
   同じ規則で上書きされる)。manifest の `register` は
   (role, path) 同一エントリを置換する。
3. **作成主体**: 最初に `analysis/` を必要とするステージ
   (通常は CheapAnalysisStage)が job manifest inputs/config で
   create する。後続ステージは検証付きで open。
4. **DatasetStage は終了処理のみ**: `analysis/` が有効な manifest
   を持てば追記モードで開き、`inputs/declared.json`・PCM・
   `locate` ロールを未登録時に追加する。manifest が存在せず/
   破損している場合:
   - Completed 解析ステージがあれば成果物を壊すため **エラーで
     失敗**(人間の介入を求める)
   - なければ `analysis/` 全体を消して再生成
   (これまで通り部分残骸は再生成される)
5. **locate ロールの物化**: `DatasetStage` が job manifest の
   `locate_result` 成果物(`locate/<src>/<cap>/result.json`)を読み
   `analysis/locate/` へ JSON + サマリ table として登録する。
   これで dataset 単体から alignment の根拠を辿れる。

## ステージ依存と空入力耐性

- `CheapAnalysisStage::depends_on() = [Ingest]`(locate 不要:
  キャプチャ単体観測はソース無しで完走する)
- `ReferenceAnalysisStage::depends_on() = [Ingest, Locate,
  CheapAnalysis]`(locate の受理アライメントと cheap の中間
  スペクトルを再利用)
- `ExpensiveAnalysisStage::depends_on() = [Ingest, CheapAnalysis]`
  (cheap 由来の候補で起動できる;reference が無くても動く)
- **`LocateStage` は `source_asset` / `capture_*` のどちらかが無い
  場合 `Err` ではなく `Ok(vec![])`** を返す(ペアが作れない以上
  やるべき仕事が無い。capture-only / source-only ジョブが
  パイプラインを止めない)。

## 確度・validity 伝播規約

spec `candidate-event-model.md` の規範を実装可能な規則に落とす:

- `Confidence` に順序 `Low < Medium < High` を定義(`Ord` 導出)。
  下流の確度は入力確度の **min** と自身の算出確度の min。
- `Validity` は順序を持たないため、**依存元マッピング**で伝播する:
  - 上流が非 `Valid` → 下流は推定を行わず、マッピングされた
    validity を持つ「無効な観測」として記録する(数値は出さない)
  - マッピング: alignment 不確か → `AmbiguousAlignment`、
    帯域コヒーレンス不足 → `LowCoherence`、無音/エネルギー不足 →
    `SilenceInsufficientEnergy`、decode 不可 →
    `DecodeInputUnavailable`、その他 → `ComputationFailed`
  - spec の 3 規則を必須実装とする: (a) アラインメント不確かなら
    伝達/ゲインの確度を下げる(確度 min)、(b) 帯域コヒーレンスが
    低ければ帯域伝達を `LowCoherence` で invalid にする、
    (c) 無音/不十分なリファレンスエネルギーはゲインを
    `SilenceInsufficientEnergy` で invalid にする
- 上流が `Valid` でも下流独自の判定(低コヒーレンス等)で
  invalid にできる。下流 invalid はイベントの validity にも
  反映される。

## Event / evidence リンク契約

- イベントは `events/` ロールの table(FeatureRecord 互換 + event
  拡張)として保存: start/end または point 時刻、ソース特徴名、
  `strength`、`confidence`、`validity`、周波数範囲・ch 範囲
  (params)、`algorithm_id`/`algorithm_version`、evidence リンク
- evidence リンク形式: `{"artifact": "<role>/<name>",
  "rows": [row_index,...]}`(dataset 相対パス + 行 index。
  tensor は `{"artifact": ..., "region": {...}}`)。行 index は
  FeatureRecord の time_base 行を指す
- イベント id は内容由来の安定文字列
  (`EVT-<algorithm>-<input>-<start>` のハッシュ)とし、再生成でも
  同一 id になる(タイムスタンプ乱数 id は使わない)
- **candidate 抽出の実行主体**: `ExpensiveAnalysisStage` の先頭で
  cheap/reference の特徴を dataset から読み candidate を生成し
  `events/` ロールに書く。その後 trigger 評価 → 対象区間に
  expensive 解析を実行し `features/expensive` に書く。
  expensive が plan に無い実行では `events` は `not_produced` のまま
  (候補検出が元の特徴を破壊しないことは保持される)

## File Map

- `Cargo.toml` - workspace members / dependencies に `al-analysis` を追加
- `crates/al-cli/Cargo.toml` - `al-analysis.workspace = true` を追加
- `crates/al-analysis/Cargo.toml` - 解析クレート
  (al-core / al-job / al-dataset / al-locate / al-media / serde /
  serde_json 依存。DSP プリミティブは `al-locate::dsp` を再利用)
- `crates/al-analysis/src/lib.rs` - 公開 API
- `crates/al-locate/src/decode.rs` - 共有 decoded-PCM ローダー
  (`decoded_pcm` 成果物 + `ingest_metadata` から deinterleave +
  `sample_format` 正規化 + lineage。LocateStage の私有ロジックを
  公開関数として切り出し、al-analysis / al-dataset も再利用する)
- `crates/al-analysis/src/stft.rs` - 複素 STFT(`al_locate::dsp::fft`
  を再利用、非 2 乗窓は次の 2 乗へゼロパディング)/ magnitude /
  power / PSD / 固定帯域パワー / 1/3 オクターブ / スペクトル包絡 /
  CSD / 複素コヒーレンス。中間スペクトルは **`intermediate` ロール
  の tensor** として保存し、reference/expensive は DatasetReader
  経由で再計算なしに再利用する
- `crates/al-analysis/src/records.rs` - `FeatureRecord` への
  マッピング、頑健要約(median / MAD / min / max / p05 / p25 /
  p75 / p95 / 有効率 / サポート数)、確度・validity 伝播規約の
  実装(上記セクション)
- `crates/al-analysis/src/cheap.rs` - cheap 特徴群:
  レベル(ピーク / true peak 推定 / RMS / DC / クレスト /
  モーメンタリ・ショートターム・統合ラウドネス推定 /
  **ラウドネスレンジ** / 短窓レベル軌跡)、スペクトル形状
  (セントロイド / 帯域幅 / ロールオフ / 傾斜 / 平坦度 /
  エントロピー / フラックス / 狭帯域ピーク・ノッチ候補)、
  ステレオ(ch レベル / ペア相関 / 複素コヒーレンス /
  M-S パワー / 帯域別 M-S / ILD / IPD)、過渡(オンセット強度・
  時刻 / 持続 / 短窓自己相似・反復ヒント)、デジタル異常
  (クリッピング率・持続 / ゼロラン / スタックラン /
  不連続・クリックスコア / 一致ブロック候補 / 量子化ステップ推定)、
  ノイズ・変調(ノイズフロア / トーン性ノイズ / 50-60Hz+高調波 /
  AM スペクトル / **粗いラグ・チャンネル変調**)
- `crates/al-analysis/src/reference.rs` - reference 特徴群:
  ラグ τ(t) / median・MAD・パーセンタイル / 微分・ドリフト /
  ステップ候補 / 局所時間スケール / ピーク幅 / 窓別確度、
  ゲイン(最小二乗 / 400Hz / 区間 RMS 比 / 軌跡 g(t) /
  ソースレベル対ゲイン)、帯域(1/3 オクターブ M2 ゲイン /
  400Hz 相対 / 帯域相関・コヒーレンス / PSD 比)、類似度
  (フルバンド相関 / 帯域相関 / PHAT 証拠 / 包絡類似度)、
  複素伝達(CSD / H(f) / H(f,t) / magnitude / アンラップ位相 /
  群遅延 / コヒーレンス・サポート)、チャンネル(混合行列 /
  **出力ごとの応答 — モノソースでは source→L と source→R の
  伝達・コヒーレンス差を第一級計測として個別保持、ステレオ要約に
  縮約しない** / 帯域別 L/R 複素コヒーレンス[magnitude+位相] /
  帯域別 ch 間相関・サポート / M/S 変化 / **低域 L/R 類似度変化** /
  極性 / クロストーク・リーク)、ダイナミクス(ソースレベル対
  ゲイン / オンセットオフセット / 強度比 / F0 比 /
  **高調波周波数比** / 持続・時間スケール比)、段階的残差 R0〜R3
  — 各残差について **波形・STFT・PSD・帯域エネルギー・
  RMS/リファレンス比・ソース相関** の 6 項目を保持し
  `residual` ロールへ保存
- `crates/al-analysis/src/events.rs` - candidate / event モデル
  (上記イベント契約)+ candidate 抽出器
- `crates/al-analysis/src/expensive.rs` - expensive 解析:
  トリガ(candidate / high-residual / low-similarity /
  abnormal-timing / manual / always-run、理由を永続化)と
  解析ファミリ(精密伝達・**カットオフ候補と `config.dictionary_text`
  のインジェスト辞書照合** / コム・エコー・反射 / SRC /
  非線形 / ダイナミクス / タイミング・反復・ドロップ /
  コーデック / ノイズ分解 / 高度チャンネル)。手動範囲は
  `config.analysis.expensive.manual_ranges`(サンプルまたは秒)で
  指定、always-run は `config.analysis.expensive.always_run`
- `crates/al-analysis/src/stage.rs` - `CheapAnalysisStage` /
  `ReferenceAnalysisStage` / `ExpensiveAnalysisStage`
  (上記 depends_on を実装。`analysis/` を open_or_create し
  `intermediate`・`features/cheap`・`features/reference`・
  `features/expensive`・`events`・`residual` ロールへ書き込む)
- `crates/al-dataset/src/types.rs` - `FeatureRecord` に
  `resolution_class` フィールド追加(7 クラス enum: `Sample` /
  `Fine5ms` / `Mid20ms` / `Broad100ms` / `Wide500ms` /
  `Seconds` / `Segment`。クラス名 + 実 window/hop の両方を
  永続化。スキーマ v1 の in-place 変更、後方互換なし)
- `crates/al-dataset/src/writer.rs` - `open_or_create`、
  置換セマンティクス、冪等 `register`
- `crates/al-dataset/src/stage.rs` - `input_references` 共有関数の
  切り出し、追記モード、locate ロール物化、manifest 不在/破損時の
  明示失敗
- `crates/al-locate/src/stage.rs` - `decoded` を `decode.rs` 共有
  ローダーへ置き換え、source/capture ゼロで `Ok(vec![])` を返す
- `crates/al-core/src/types.rs` - `Confidence` に `Ord` 導出
- `crates/al-cli/src/main.rs` - 3 ステージを実装に差し替え、
  USAGE 更新
- `crates/al-analysis/tests/` - 合成 fixture 結合テスト
- `crates/al-dataset/tests/` - 追記・置換・resume 冪等の回帰テスト

## Implementation Sequence

- [ ] `Confidence` に `Ord` 導出、`FeatureRecord.resolution_class`
  追加(al-core / al-dataset の基盤変更)
- [ ] `DatasetWriter::open_or_create` + 置換セマンティクス +
  冪等 `register` + `input_references` 共有関数 +
  DatasetStage 追記/物化/明示失敗モード
- [ ] `decode.rs` 共有ローダー + LocateStage の空入力耐性
- [ ] `al-analysis` 骨格 + `records.rs`(解像度クラス / 要約 /
  伝播規約 / FeatureRecord 変換)
- [ ] `stft.rs` 中間スペクトル + `intermediate` ロール保存
- [ ] cheap 特徴(レベル / スペクトル形状)を実装し dataset へ保存
- [ ] cheap 特徴(ステレオ / 過渡 / 異常 / ノイズ・変調)を追加
- [ ] reference 特徴(lag / gain / band / similarity)
- [ ] reference 特徴(複素伝達 / チャンネル / R0〜R3 付随データ)
- [ ] `events.rs`: event モデル + candidate 抽出 + 確度伝播
- [ ] `expensive.rs`: トリガ永続化 + 解析ファミリ + 辞書照合
- [ ] 3 ステージの Stage 実装 + `al-cli` 配線 + USAGE 更新
- [ ] 合成 fixture 結合テスト + resume 冪等テスト

## Dependencies and Risks

- 依存: `al_locate::dsp`(`fft` / `ncc` / `gcc_phat` / `bandpass` /
  `spectral_envelope` / `least_squares` / `condition_number`)、
  `al_locate::mixer::fit_mix` / `al_locate::residual::subtract_fit`、
  `al-dataset` の writer/reader、`al-media` の decoded PCM。
  外部クレートは追加しない
- 依存: `fft` は 2 乗長のみ対応 → STFT 窓はゼロパディングで
  2 乗に揃える
- 依存: locate の受理アライメント。未受理時は reference の
  位置依存特徴を `AmbiguousAlignment` で invalid として記録
- リスク: `analysis/` への複数ステージ追記 → 上記「Dataset 追記
  プロトコル」で検証付き open + 置換 + 明示失敗を規約化
- リスク: 特徴量が多く設計詳細が spec レベル → パラメータは
  `config.analysis.*` で上書き可能にし、全出力に algorithm id /
  version / params / resolution_class+実 window/hop を記録
- リスク: expensive のコスト爆発 → candidate 駆動をデフォルト、
  always-run は明示 config のみ
- リスク: 確度伝播の不徹底 → 上記規約(Confidence min +
  Validity 依存元マッピング)を records.rs に集約
- リスク: true peak / F0 / ラウドネスの厳密規格準拠は scope 外 →
  「推定」として params に手法を記録し validity を付す
- リスク: decoded PCM はネイティブスケール → `decode.rs` で
  `sample_format` による float 域正規化を 1 箇所に集約
  (al-dataset / al-locate / al-analysis の三重実装を防ぐ)

## Task Breakdown

- `al-core` Confidence 順序付け + `al-dataset`
  `FeatureRecord.resolution_class`
- `al-dataset` 追記プロトコル(open_or_create / 置換 / 冪等 /
  input_references / DatasetStage 追記・locate 物化・明示失敗)
- 共有 decoded-PCM ローダー + LocateStage 空入力耐性
- `al-analysis` 骨格 + records.rs(解像度/要約/伝播)
- STFT / PSD / 帯域 / CSD / コヒーレンス中間スペクトル +
  intermediate ロール
- cheap: レベル + スペクトル形状
- cheap: ステレオ + 過渡 + デジタル異常 + ノイズ/変調
- reference: lag / gain / band / similarity
- reference: 複素伝達 / チャンネル / R0-R3 + 付随データ
- events: candidate / event モデル + 確度伝播
- expensive: トリガ + 解析ファミリ + 辞書照合
- 3 ステージ Stage 実装 + CLI 接続
- 合成 fixture 結合テスト + resume 冪等テスト

## Verification

- [ ] `mise run test` / `cargo test --workspace` が全 crate で通る
  (`cargo metadata` で `al-analysis` が member として解決される)
- [ ] `cargo clippy --workspace --all-targets -- -D warnings` /
  `cargo fmt --all -- --check` clean
- [ ] `al run --input capture_bug:<wav> --stages
  ingest,cheap_analysis,dataset` がソース素材なしで完走し、
  cheap 特徴が `features/cheap` に保存される(locate を plan に
  含めず実行可能)
- [ ] `al run --input capture_bug:<wav>,source_asset:<wav>` で
  full pipeline が完走する
- [ ] 中間スペクトル(複素 STFT / PSD)が `intermediate` ロールの
  tensor として保存され、reference/expensive が DatasetReader から
  再計算なしに再利用できる
- [ ] 特徴出力に resolution_class + 実 window/hop、validity、
  algorithm id / version / params が含まれる
- [ ] 合成 fixture(既知ゲイン・EQ・ドリフト)で reference 特徴が
  既知値を回復する
- [ ] R0〜R3 残差が段階的にエネルギーを減らし、各残差の
  波形+STFT+PSD+帯域+RMS比+相関が `residual` ロールに存在する
- [ ] イベントが `{"artifact","rows"}` リンクで裏付け観測を指し、
  低確度入力が高確度の下流イベントにならない(上流 invalid →
  下流 invalid)
- [ ] expensive 結果がトリガ理由を記録し、manual_ranges /
  always_run で実行できる。カットオフ候補が辞書エントリと
  照合される
- [ ] 失敗注入(テスト限定の失敗ステージ)による中途失敗後の
  resume で同じ (role,path) が重複登録されず、部分残骸を上書き
  して完走する
- [ ] `analysis/locate/` に locate 結果が物化され、dataset 単体で
  alignment を辿れる
- [ ] dataset を消さないで analysis ステージを再実行しても
  成果物が置換される

## Open Questions / Gaps

- 1/3 オクターブ M2 の帯域数・窓の最終値 → fixture で調整
  (spec の deferred concern どおり)
- true peak のオーバーサンプル次数 → 4x 線形補間を初期値として
  params に記録
- candidate 統合の重み付け → fixture での評価前提
  (design の deferred concern どおり)

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

- review-protocol レビュー(Reviewer A 13 件・Reviewer B 12 件の
  候補)で指摘されたブロッカーをすべて反映済み:
  depends_on / 空入力耐性、store 置換 + manifest 冪等 + 孤立残骸、
  manifest.inputs 供給元、確度・validity 伝播規約、resolution
  クラス、locate/intermediate ロール、evidence リンク、
  spec 抜け(LRA / 粗変調 / 高調波比 / per-ch 応答 / 低域 L/R /
  R0-R3 付随 / 辞書照合)、candidate 抽出主体、DatasetStage
  追記/明示失敗、検証の実行可能性(fault injection、明示
  stages 指定)

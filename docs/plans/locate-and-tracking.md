---
id: "PLAN-034z7HA18FxYMCQPBZ6Gg2"
type: "plan"
status: "approved"
title: "ソース定位・トラッキングの実装"
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
  implements:
    - "docs/specs/source-localization.md"
    - "docs/specs/locate-tracking.md"
  implemented-by: []
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related: []
  refines: []
  refined-by: []
  derives-from:
    - "docs/specs/source-localization.md"
    - "docs/specs/locate-tracking.md"
    - "docs/designs/localization.md"
    - "docs/adr/multi-method-locate-consensus.md"
  derived-by: []
  verifies: []
  verified-by:
    - "docs/test-specs/locate-method-consensus.md"
    - "docs/test-specs/locate-tracking-residual.md"
  references: []
  defers: []
  deferred-by: []
---

# ソース定位・トラッキングの実装 Implementation Plan

## Goal

`al-locate` クレートに、独立した複数手法によるソース定位証拠・
合議受理・ループ/長尺トラッキング・多ch混合推定・2段階残差探索を
実装し、`Locate` ステージとして runner に接続する。

## Scope

本プランは 1 つの実装ストリーム。locate の全コンポーネント
(locate-methods / consensus / tracking / mixer-fit /
residual-search)を対象とする。

- In scope: NCC・band-NCC・GCC-PHAT・スペクトル包絡の 4 系統の
  独立定位手法、候補/証拠/棄却理由の永続化、合議受理
  (≥2 手法一致 + ピーク/背景閾値)、ループ反復の個別識別、
  長尺ソースの区分的 time map(anchor + segment)、最小二乗
  混合行列推定(符号付き・リッジ正則化・条件数記録)、
  2 段階残差探索と減算来歴、`LocateStage` 実装と CLI 接続
- Out of scope: 定位結果の意味評価(バグ判定)、定位精度の新規
  DSP 研究、UI マッピング表示、実コンテナからの映像側定位
  (音声 PCM のみ対象)

## Upstream Documents

- `docs/specs/source-localization.md`
- `docs/specs/locate-tracking.md`
- `docs/adr/multi-method-locate-consensus.md`

## Design Inputs

- `docs/designs/localization.md`
- `docs/designs/overview.md`

## File Map

- `Cargo.toml` - workspace `members` / `workspace.dependencies`
  に `al-locate` を追加(未追加だと `mise run test` の対象外になる)
- `crates/al-cli/Cargo.toml` - `al-locate.workspace = true` を追加
- `crates/al-locate/Cargo.toml` - 定位クレート
  (al-core / al-job / al-media / serde / serde_json 依存)
- `crates/al-locate/src/lib.rs` - 公開 API
- `crates/al-locate/src/dsp.rs` - DSP プリミティブ(NCC、
  FFT/DFT 補助、バンドパス、スペクトル包絡、最小二乗)。
  interleaved `decoded.f64le` の deinterleave / ch 選択もここ
- `crates/al-locate/src/methods.rs` - locate-methods:
  `LocateMethod` trait + 4 実装(full_ncc / band_ncc /
  gcc_phat / spectral_envelope)。それぞれ候補位置・スコア・
  ピーク対背景比・ピーク幅・第1/第2ピーク比・validity・
  棄却理由を生成
- `crates/al-locate/src/candidate.rs` - 候補/証拠の型
  (`LocateCandidate` / `MethodEvidence`)
- `crates/al-locate/src/result.rs` - locate 結果スキーマ:
  `AlignmentId` 発行と `LocateResult`(spec 出力必須項目:
  ソース素材 id・ソース区間・キャプチャ区間・手法ごとの証拠・
  手法一致数・位置のばらつき・確度/validity・
  アルゴリズム・バージョン・パラメータ)。artifact role は
  `locate_result` / `locate_evidence` / `time_map` / `residual`
- `crates/al-locate/src/consensus.rs` - 合議受理
  (≥2 手法の位置一致[許容差設定可] + ピーク/背景閾値。
  不一致・棄却理由を保存)
- `crates/al-locate/src/tracking.rs` - tracking:
  `Anchor` / `Segment` / `TimeMap`、ループ反復識別
  (必要に応じ 2〜3 個連結したループ反復で探索し、出現ごとに
  ループ index / 出現位置 / ソース位相・時間 / キャプチャ時間を
  保存)、ドリフト・局所ジッタ・スキップ・ギャップ・再開・
  未マッチの区間表現
- `crates/al-locate/src/mixer.rs` - mixer-fit:
  per-channel 最小二乗 source→capture 混合行列(符号付き係数、
  極性反転を負係数として表現、リッジ正則化、適合品質・条件数)
- `crates/al-locate/src/residual.rs` - residual-search:
  支配的ソースの減算(来歴: 親ソース・alignment id・
  推定ゲイン/チャンネル変換[混合行列]・減算前後エネルギー・
  減算確度・子 locate)
- `crates/al-locate/src/stage.rs` - `LocateStage`(Stage trait、
  ingest の decoded PCM + metadata を読み locate 成果物を出力。
  `manifest.inputs[].role` で `source_asset` × `capture_*` を
  ペアリングし、ch 数・サンプルレートを metadata から復元。
  fs 不一致ペアは推定せず `decode_input_unavailable` 相当で
  明示マーク)
- `crates/al-cli/src/main.rs` - `Locate` スタブを実装に差し替え、
  USAGE の「他ステージはスタブ」記述を更新
- `crates/al-locate/tests/` - 合成信号 fixture の結合テスト

## Implementation Sequence

- [ ] workspace / crate の Cargo 配線と `al-locate` 骨格、
  DSP プリミティブ(deinterleave・ch 選択含む)を実装
- [ ] `LocateMethod` trait + 4 手法を実装し候補/証拠を生成
- [ ] `LocateResult` スキーマと `AlignmentId` 発行を実装
- [ ] 合議受理(閾値・位置許容差は設定可能)を実装
- [ ] 失敗モードフラグ(無音境界・周期信号・広いピーク・低情報窓・
  偽相関)を実装
- [ ] ループ反復識別と anchor/segment/TimeMap を実装
- [ ] 最小二乗混合行列推定を実装
- [ ] 2 段階残差探索と減算来歴を実装
- [ ] `LocateStage` を実装(source/capture ロールペアリング +
  fs 不一致の明示マーク)して `al-cli` に接続
- [ ] 合成 fixture の結合テストを追加

## Dependencies and Risks

- 依存: `al-core` / `al-job` / `al-media`(ingest 成果物の
  decoded PCM)。外部クレートは追加しない(FFT も自前実装)
- リスク: 合議閾値・位置許容差のデフォルトが未調整 →
  設定可能なパラメータとして config 化し、合成 fixture で
  「真の位置が受理される」「単一手法の偽ピークが受理されない」
  を検証する(設計の deferred concern どおり fixture 基準)
- リスク: 4 手法の計算コスト → 候補生成と受理を分離し、
  高コスト補助解析は設計どおり後段へ回す
- リスク: ingest の PCM は f64 の「ソースネイティブスケール」
  (整数 PCM は整数値、IEEE float はそのまま、`sample_format`
  に記録)で保存されている → NCC/GCC-PHAT はスケール不変だが、
  スペクトル包絡と残差エネルギーは絶対スケール依存なので
  `sample_format` を見て比較前に正規化する
- リスク: `decoded.f64le` は interleaved でヘッダなし →
  ch 数・fs は artifact params / metadata.json から復元し、
  定位は ch 単位(deinterleave)で行う。ステレオ source と
  ステレオ capture の ch 対応は mixer 推定に任せ、定位自体は
  ch ごとに証拠を集める
- リスク: ソースとキャプチャのサンプルレート不一致 →
  リサンプルは行わず、そのペアは `decode_input_unavailable`
  相当で明示マーク(spec の「低情報は明示マーク」方針)

## Task Breakdown

- `al-locate` 骨格 + DSP プリミティブ
- locate-methods 4 手法 + 候補/証拠型
- 合議受理 + 失敗モードフラグ
- tracking(anchor/segment/TimeMap + ループ)
- mixer-fit(最小二乗混合行列)
- residual-search(2 段階 + 減算来歴)
- `LocateStage` + CLI 接続
- 合成 fixture 結合テスト

## Verification

- [ ] `mise run test` が全 crate で通る(`cargo metadata` で
  `al-locate` が workspace member として解決される)
- [ ] 合成信号(既知オフセットで埋め込んだソース)で真の位置が
  受理される
- [ ] 1 手法のみの偽ピークが受理マッチにならない
- [ ] 無音・低情報区間が通常失敗ではなく明示的にマークされる
- [ ] 手法ごとの位置・棄却理由が成果物に保持される
- [ ] ゲイン変更挿入 / EQ フィルタ済み挿入 / 複数の正当な出現
  (`validation-strategy` の Locate fixture)が受理される
- [ ] 線形ドリフト・スキップ+再開の fixture で区分的 time map が
  生成される
- [ ] マルチチャンネルダウンミックス(極性反転含む)で混合行列が
  符号付き係数として推定される
- [ ] ループ入力で反復が個別識別される(1 つの曖昧マッチに
  潰れない)
- [ ] 長尺入力でグローバルオフセットではなく区分的 time map が
  生成される
- [ ] 残差探索成果物が減算来歴(親・alignment id・
  ゲイン/チャンネル変換・エネルギー)を保持し、支配的 BGM
  減算後に静かな素材が受理される
- [ ] `al run --base-dir <dir> --input capture_bug:<cap.wav>
  --input source_asset:<src.wav> --stages ingest,locate` で
  locate 成果物が manifest に記録される(source=input src /
  capture=input cap にペアリングされる)

## Open Questions / Gaps

- dataset ストアが別ストリームのため、locate 成果物はまず job
  manifest の artifact として保存する(dataset プラン実装後に
  writer 側で取り込む)
- 閾値デフォルトは合成 fixture 基準。実キャプチャでの調整は
  運用フェーズ

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

- locate 成果物を job artifact として保存し dataset 取り込みは
  writer 側責務とする境界を確認
- 4 手法すべて自前実装(外部 DSP クレート不使用)の妥当性を確認
- 音声 PCM のみ対象とし映像側定位を持たない範囲設定を確認

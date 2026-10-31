# Issue #103 Job・Dataset・Provenance・GUI 参照データ回帰検証記録

- 対象 Issue: audio-lens #103
- 基準 baseline: Issue #98 成果物(`baseline/runs`)
- 検証比較出力: `docs/impl/ir/issue-103-baseline/`(`tools/baseline/compare.mjs` の結果)
- 関連: #97(責務仕様)/ #98(golden)/ #100(media I/O 境界)/ #101(汎用 f64 カーネル)/ #102(解析意味論分離)/ #115(decoder rev provenance 乖離、別 Issue)

## 目的

Ruviform 移行(#100–#102)後も AudioLens 側が所有する永続スキーマ・
Job 制御・provenance・UI 非依存 reader 契約が回帰していないことを
実ジョブ・失敗後再開・golden 比較で検証する。

## 検証した契約と根拠

### al-job(既存テストで担保)

| 契約 | 根拠 |
| --- | --- |
| immutable/serialized Job 定義・入力 pin(絶対パス化+sha256) | `crates/al-job/src/job.rs` `JobDefinition::create` |
| Stage 依存順・plan 検証・部分失敗・再開 | `crates/al-job/src/runner.rs` / `crates/al-job/tests/`(resume・部分失敗・完了 stage skip) |
| 欠損/切断 artifact・dir/chunked store の部分再生成 | `crates/al-job/tests/`(missing/truncated/member missing 系) |
| Manifest・ArtifactRegistry・hash・lineage 再構成 | `crates/al-job/src/manifest.rs` / `registry.rs` + 同上テスト |

### al-dataset(既存テストで担保)

| 契約 | 根拠 |
| --- | --- |
| chunked table / f64 tensor / 複素 STFT 中間 / WAV・Float64 WAV | `crates/al-dataset/tests/` round-trip・range read 系 |
| 階層・ID・Validity・Confidence・Resolution・invalid_reason・support_count | `FeatureRecord` スキーマ + dataset 統合テスト |
| Provenance(input/algorithm/params/alignment/lineage) | `crates/al-dataset/src/provenance.rs` |
| schema/input/config mismatch・corrupt manifest 拒否 | `crates/al-dataset/tests/` |
| UI 非依存読み出し(manifest-only entry・table/tensor/audio/JSON range) | `crates/al-dataset/src/reader.rs` + 実ジョブ headless 検証(下記) |

## 実ジョブ検証(本 Issue で実施)

- `ingest,locate,cheap_analysis,reference_analysis,expensive_analysis,dataset,export`
  を Visualization Stage **なし**で実ジョブ実行
  (`samples/_runs/vizfree/JOB-x91RS1aYGa8tZY2q6S6EtQ`):
  全 stage completed、`analysis/visualization/` 未生成のまま
  `export/analysis/`(dataset 複写)+ `export_manifest.json` +
  `golden/snapshot.json` + `report.html`/`report.data.js` が生成された
- `resume` run(全 stage plan・locate 完了時点で kill → `al resume` で完走)
  を capture で再実行 → `resume` vs `canonical`(同一入力)の golden 比較で
  再開後 Dataset/Manifest/hash の一致を確認
- 全 9 run(canonical/aa/small/resume/variant-×5)を
  `tools/baseline/capture.mjs` で再収集し #98 golden と比較

### UI 非依存 reader(headless 検証)

`tools/baseline/lib/verify-dataset-read.mjs` で実ジョブの
`analysis/`(Analysis Dataset)を GUI 層なしに走査:

- **table**: `events/candidates`(36 rows, event_id/kind/validity/
  confidence/evidence/params)、`locate/summary`(2 rows, alignment_id/
  validity/confidence/capture_start..end timemap)
- **tensor**: `features/cheap|expensive|reference`(level/spec
  trajectories,onset_times,time_map)、`intermediate`(stft_re/stft_im
  複素 STFT,magnitude,psd,coherence,csd,transfer_h 等)
- **audio**: `inputs/*.wav`(Float WAV 4.8M frames)、
  `residual/<pair>/r0..r3.wav`(音声残差)
- **json**: `locate/<src>/<cap>.json`、manifest.json 本体
- DatasetReader API(manifest-only open・role 列挙・table/tensor 範囲・
  audio range・JSON)は `crates/al-dataset/src/reader.rs` + 統合テスト
  (`large_table_range_and_column_reads`,`tensor_and_audio_round_trip_via_dataset`,
  `a16d_read_range_micro_timing_baseline` 等)でも担保

### provenance(実ジョブ manifest から確認)

dataset manifest の input artifact provenance.params に記録済み:

- decoder attempts(backend=`ruviform`、version=`e467ccb…`、outcome、
  reason)、fallback フラグ
- request(audio/video 選択・video_config)、stream metadata
  (codec=`pcm_s16le`、channels、pcm=`unscaled_interleaved_f64le`)
- `settings.ruviform`{io=[ruviform-io-symphonia,ruviform-io-avi],
  rev, video_path=native_dib}
- 変換履歴:コンテナ(AAC-in-MP4 / pcm-in-AVI)→
  unscaled_interleaved_f64le 変換が decoder params に記録
- 演算アルゴリズム/精度/パラメータ: 各 FeatureRecord の
  algorithm_id/version/params + f64 dtype で保持

## 変更点

- `crates/al-viz/src/stage.rs`: `ExportStage::depends_on` を
  `[Dataset, Visualization]` → `[Dataset]`。export パッケージは
  `analysis/` ディレクトリを丸ごとコピーするのみで visualization
  成果物を必須入力としない(不在時は skipped 扱い)。宣言依存の
  過剰指定を実依存に合わせる境界修正(#103 完了条件「旧
  Visualization Stage 非依存」)
- `crates/al-viz/tests/viz.rs`: `export_stage_does_not_require_visualization`
  契約テスト追加
- `tools/baseline/lib/verify-dataset-read.mjs`: headless Dataset
  読み出し検証スクリプト追加

## 発見した問題

- 解析 stage(`al-analysis`)の `Provenance` / `FeatureRecord.params` には
  Ruviform revision が直接 stamp されない。manifest 上の Ruviform 識別は
  ingest 側 decoder provenance(`settings.ruviform.rev`)に集約されている
  (ruviform は単一 repo pin のため全体を同定可能)。解析系 artifact にも
  `ruviform_numeric` rev/precision を記録する改善は Issue 起票案として報告
- decoder rev の記録値 `e467ccb…` が lockfile の実値 `f3f474b…` と
  乖離 → 既知 #115(別 Issue。「rev が記録される」契約は満たす)

## Golden 比較結果(tools/baseline)

`baseline/runs/<name>`(#98 凍結)を `<name>-golden98` へ退避し、9 ランを
本 Issue 変更後コード(`target/release/al`)で `capture.mjs` 再収集、
`compare.mjs`(rel 1e-9 / abs 1e-12)で比較後に凍結 golden を復元。
差分実体は `issue-103-baseline/*-vs-golden98.json`、機械分類は
`tools/baseline/lib/classify-diffs-103.mjs`。

**全 9 ラン・全差分が既知 2 クラスに完全帰属し、説明不能な新規差分は 0 件**:

| run | total | #101 既知 | #100 decoder | unexplained |
| --- | --- | --- | --- | --- |
| canonical | 175 | 45 | 130 | 0 |
| aa | 175 | 45 | 130 | 0 |
| resume | 175 | 45 | 130 | 0 |
| small | 1309 | 1149 | 160 | 0 |
| variant-desync | 150 | 20 | 130 | 0 |
| variant-clipping | 172 | 42 | 130 | 0 |
| variant-reverb | 161 | 31 | 130 | 0 |
| variant-filter | 167 | 37 | 130 | 0 |
| variant-clicks | 181 | 51 | 130 | 0 |

- `#101 既知`: `issue-101-baseline/<run>-diff.json` と
  surface/path/kind が完全一致(SIMD 累積順序差の tolerance/value 差。
  transfer_h_t fingerprint・events params `measured` 等を含む)
- `#100 decoder`: `*/params/decoder/*`・`*/decoder/*` パスのみの差分。
  `backend: "native"→"ruviform"`、`version: "0.1.0"→<git rev>`、
  `settings/ruviform` キー追加(#100 media I/O 移行の正当な
  provenance 変更)
- validity/confidence/status・artifact 構成・ID・イベント数に差分なし

### 決定性・再開一致

- `resume` vs `canonical`(同一入力): **0 diffs** — locate 完了時点で
  kill → `al resume` で完走した Dataset/Manifest/golden が
  クリーンランとバイト一致
- `aa` vs `canonical`(同一入力の別ジョブ): **0 diffs** — 実行間決定性

## 検証結果

- `mise run fmt` / `check` / `lint` / `test` / `build`: pass
  (`cargo test --workspace` 全 41 セクション green、新規契約テスト
  `export_stage_does_not_require_visualization` 含む)
- `mise run verify-toolchain` / `check-cli-headless`: pass
- `cargo build --release -p al-cli`: pass(baseline capture 用バイナリ)
- 実ジョブ(Visualization なし ingest→…→dataset→export): pass
- 実ジョブ resume(kill-after-locate → `al resume`): 全 stage completed、
  canonical と 0 diffs
- headless dataset 読み出し: pass(table/tensor/WAV/JSON 全 role)

## 残課題・Issue 起票案

- **解析 stage artifact への `ruviform_numeric` rev/precision stamp**
  (起票案): manifest は ingest 側 `settings.ruviform.rev` で Ruviform
  全体を同定可能だが、解析系 artifact の params には直接記録されない。
  FeatureRecord params への追加は golden 差分を生むため本 Issue では
  変更せず、起票案として報告する。
- decoder rev 記録値 `e467ccb…` vs lockfile `f3f474b…` の乖離は
  既知 **#115**(rev が記録される契約自体は本 Issue で検証済み)。

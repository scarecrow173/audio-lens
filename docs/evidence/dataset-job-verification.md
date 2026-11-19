---
id: "EVID-kN8zRtWqPbYjXsFmVcDh"
type: "evidence"
status: "approved"
title: "検証記録: ジョブ基盤・dataset ストア・PcmView"
created: "2026-11-19"
updated: "2026-11-19"
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
  implements: []
  implemented-by: []
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related: []
  refines: []
  refined-by: []
  derives-from: []
  derived-by: []
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# 検証記録: ジョブ基盤・dataset ストア・PcmView

`al-job`/`al-dataset` の manifest・resume・永続化・PCM 読み出しに関する検証の事実記録。

## ジョブ基盤の実証テスト

- `crates/al-job/tests/resume.rs` 5 件: スタブ stage に実行カウンタを持たせ副作用を直接観測し、部分失敗→resume で完了済み成果物再利用+未完了のみ再実行(A→B→C で B 失敗→B/C のみ)、全成功後 resume が no-op、再実行で新 job id・別 manifest、依存未充足計画の拒否、lineage 再構成を検証。
- minimal CLI 実検証: `al run --base-dir <dir> --input capture_bug:cap.mkv,source_asset:src.wav --stages ingest,locate` でジョブ作成、`al resume --job-id <id>` が no-op で Completed、`al inspect` が manifest JSON 出力。`cargo build`/`cargo test --workspace`/clippy/fmt 全 pass。

## stage 計時の永続化検証

- `manifest_without_duration_ms_loads_as_none`(旧 manifest 読込)・`duration_ms_is_recorded_and_resume_overwrites_only_rerun_stages`(sentinel 値で上書き/保持検証)を追加。`al run` 後 manifest.json で実行 stage が `Some`・未実行が `None` を確認。

## dataset ストア検証

- `store_replacement_is_full_and_idempotent`・`failed_dataset_stage_is_resumable`: 書き込み中失敗→resume の機構を dataset 層と runner 層で固定。同一 `(role,path)` 重複登録防止は store 置換テストで担保。
- `cargo test -p al-dataset` 全 pass(export 系 5 件含む: depends_on、パッケージ内容、同一 basename 退避、`export.name` の traversal/絶対/区切り/空名拒否、絶対パス痕跡走査)。
- `Value::Scalar(f64)` の `Deserialize` 手動実装で JSON `null`→`NaN` 復元を検証(invalid スカラーが null に serialize される往復一貫)。
- `feature_record_round_trips` で `sample_rate` 保持と必須フィールド欠落の deserialize エラーを固定(旧成果物の黙った補完をしない)。

## PcmView(mmap + Owned fallback)

- `PcmView::open` で非空 regular file の mmap を試み、空・非 regular・失敗で `open_owned` へ fallback する経路を検証。唯一の unsafe は `Mmap::map`。
- `deinterleave` の融合化で中間 interleaved `Vec<f64>` を生成しないこと、`chunks_exact` の部分フレーム drop・`channels==0` で空・チャンネル順序保持を `dsp::deinterleave` と揃えたことを確認。
- caller 移行検証: `decode_input` の `len % 8` エラー文言維持、`read_normalized_record` の expected length 厳密一致と有限性検証維持、`cargo test -p al-locate -p al-analysis` 全件 pass。
- job-scoped cache 不採用の判定根拠: mmap `open` p50 81–86µs で materialize の 0.00–0.04% に過ぎず、`Mutex<HashMap>`+bounded eviction の複雑性に見合わない。計測は `tasklist` LIST 形式の parse(日本語ロケール非依存)。
- 全長 materialize の計測値(Windows x86_64、debug、N=30): 36MB で baseline p50 248ms/mmap view 219ms/owned view 161ms、368MB で 2548ms/2242ms/1584ms。本環境では Owned が mmap より p50 高速(連続 `fs::read` が page-fault 駆動より速い)だが、mmap は alloc bytes 半分(36 vs 72MB、368 vs 736MB)で ADR の mmap-first 方針を維持した。

## 未検証・制約

- PcmView 計測は debug build で、release/他 OS での Owned/mmap 逆転確認は将来課題。部分読み取り対応(将来)では mmap 優位が変わる。
- 基盤 task 群の Validation 節は空欄が多く、実行証拠は minimal-cli IR に集中していた(当時の文書上の記録状況)。
- `analysis/` 不在時の `open_or_create` 失敗、manifest 破損時の Completed 保護エラーの経路は stage 実装側で固定。

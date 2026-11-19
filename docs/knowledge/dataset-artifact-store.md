---
id: "KNOW-ftoVnskfNQROBPHiY48VVR"
type: "knowledge"
status: "confirmed"
lifecycle: "current"
knowledge_kind: "architecture"
title: "Analysis Dataset: 永続化スキーマ・追記プロトコル・PCM ビュー"
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

# Analysis Dataset: 永続化スキーマ・追記プロトコル・PCM ビュー

`al-dataset` クレートは、解析ステージの全観測を `<job_dir>/analysis/` 以下に永続化するストア層である。manifest・role 別ストア・provenance・テーブル・テンソル・WAV 入力・可視化ロールを一体として管理する。本書はスキーマ設計、追記/置換の冪等性、PCM ビュー、および durability の設計をまとめる。

## スキーマと物理構成

- `manifest.json` は `schema_version`/`inputs`/`config` を含み、`open_or_create` はこれらの完全一致を検証して不一致・破損は `InvalidData` で失敗する。互換層・マイグレーションは設けない。
- `ArtifactRecord` は「role/path」で 1 ストア 1 レコードを表し、内部チャンクを個別にレコード化しない。`ArtifactRecord::path` は job_dir 相対で、外部入力は `params.source_path`/`source_path_abs` に記録する。`resolve()`/`resolve_by_role()` は空 path を `None`/スキップして job_dir 自体への誤解決を防ぐ。
- テーブルは自己完結スキーマ + リトルエンディアン列チャンク + null bitmap。テンソルは shape・stride・chunk 境界をメタデータとして持ち、実データは生 LE バイト列チャンク。
- 音声ストアは WAV のみ(`WavEncoding`)。FLAC は非採用で、圧縮は再検討条件(容量・I/O 支配の実測)が揃うまで別 Issue とする(#33 承認)。

## 追記・置換・provenance

`write_table`/`write_tensor`/`write_audio`/`write_json` は対象 `role/name` の既存ストアルートを削除して再作成し、`register` は `(role,path)` 一致の既存エントリを置換する。孤立残骸も同規則で消す。作成主体は最初に `analysis/` を必要とするステージで、`InputReference` は pinned sha256 を必須とし、`input_references(&Manifest)` に集約する。

dataset 追記は provenance を維持する:`ArtifactRecord` に `produced_by`、role 別に producer の `algorithm`/`alignment_id`/`parents` を記録し、共有 writer が誤って `cheap.v1` や `alignment_id: None` に固定しないよう引数化した。tensor chunk は `chunk_shape==shape` で部分読み出しが全体ロード化するため、時間軸方向に ~4 MiB 上限(`config.analysis.dataset.tensor_chunk_bytes` で上書き可)で分割する。

## PcmView(mmap + Owned fallback)

`decoded.f64le`/`normalized_pcm` artifact を read-only `PcmView` で読む設計(ADR `artifact-id-read-only-pcm-view-cache`)。

- `PcmBacking::Mapped`(mmap)と `PcmBacking::Owned`(`fs::read`)を内包し、`open` は非空 regular file の mmap を試み、空・非 regular・失敗で `open_owned` に fallback、I/O エラーは伝播。唯一の unsafe は `Mmap::map` で、artifact は `write_durable` の rename 完了後に不変という前提を SAFETY コメントに記録する。
- `deinterleave` は per-sample の `f64::from_le_bytes / scale` を融合し、中間 interleaved `Vec<f64>` を生成しない。`chunks_exact` の部分フレーム drop・`channels==0` で空・チャンネル順序保持を `dsp::deinterleave` に揃える。
- job-scoped cache は不採用(mmap `open` p50 81–86µs で materialize の 0.00–0.04% に過ぎず、支配コストは全長 materialize)。`ArtifactId` 紐付き read-only view のみで、deinterleave 済み `Vec<Vec<f64>>` はキャッシュしない。

## durability と resume

- `write_durable` は temp file + `sync_all` + rename + ディレクトリ fsync(Unix)の atomic & durable write。stage 成果物・manifest・dataset ストアのすべてに一本化し、「manifest は Completed だが artifact 欠損」の電源断不整合を防ぐ。
- `StageRecord.duration_ms: Option<u64>` は `#[serde(default)]` で旧 manifest 互換を保ち、resume 再実行は上書き(合算しない)。
- resume セマンティクスは manifest 記録の `inputs`/`config`/`stages` と JobDefinition の完全一致を検証し、同一 JobId の改変定義は実行前に拒否する。

## 設計上の教訓

- ストア置換は「manifest 登録の有無に関わらず削除」でなければ冪等にならない。同名残存を見逃すと resume で重複登録になる。
- role/path の分解を文字列操作で行うと role が `/` を含む(`features/cheap`)場合に破壊される。manifest の `artifact.role` を正本とする。
- `source_reference` は存在せず、正しい入力ロール名は `source_asset`。
- `Value::Scalar(f64)` の JSON `null` は NaN を表し、deserialize で復元する —— この往復一貫を崩すと invalid スカラーが消える。

関連 Issue/PR: #33、#40、#54、#56、#58、AL#191、#199、#202。

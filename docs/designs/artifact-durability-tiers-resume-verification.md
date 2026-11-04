---
id: "DESIGN-034GsNQAvnJk0iYj6jFzOI"
type: "design"
status: "approved"
title: "Artifact durability tiers and resume verification"
created: "2026-10-15"
updated: "2026-11-04"
owners: []
relations:
  source:
    - "../specs/durability-tiered-artifact-writes.md"
    - "../adr/artifact-durability-tiers-relaxed-bulk-writes.md"
  changes:
    added: []
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "../specs/durability-tiered-artifact-writes.md"
    - "../adr/artifact-durability-tiers-relaxed-bulk-writes.md"
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
  verified-by:
    - "../test-specs/durability-tiers-resume-verification.md"
  references: []
  defers: []
  deferred-by: []
---

# Artifact durability tiers and resume verification

## Overview

`al_job::fsutil` は `ruviform-io-file` の StagedFile type-state 契約
(`StagedFile` → `SealedFile::publish`)を介して全 artifact 書込を
処理し、等級は呼出側が `PublishOptions` で明示する: bulk データは
`Durability::BestEffort`、記録系は `Durability::FileAndDirectorySync`。
`StageRunner::resume` は Completed/Skipped stage の artifact 実在を
検証し、欠落を検出した stage を Pending へ降格して再生成する。
検証メタデータ(`params.byte_len` / `params.files`)は producer が
ArtifactRecord に記録する。

当初は fsutil 内の自前プリミティブ(`write_durable`/`write_atomic`/
`rename_atomic`)として実装されたが、AL-IO-02([#143](https://github.com/scarecrow173/audio-lens/issues/143)、
PR #154)で汎用 staging/publish が `ruviform-io-file` へ移管され、
等級割当と resume 検証の設計意図は本書のとおり維持された。
移行一覧と差異の記録は [ruviform-io-migration-issue-139.md](./ruviform-io-migration-issue-139.md)
を参照。

## Structure

### `al_job::fsutil`(StagedFile adapter)

```
stage_file(path)                      # parent mkdir + StagedFile::create
publish_bytes(path, bytes, options)   # stage → write_all → seal → publish_sealed
publish_sealed(sealed, options)       # SealedFile::publish + temp recovery
```

`options` は `PublishOptions::new(PublishMode::Replace, Durability::*)`
を呼出点が用途ごとに明示する。`Replace` は既存 destination を
原子的に置き換える。`Durability::BestEffort` は fsync/dir sync を
要求しない relaxed 経路、`Durability::FileAndDirectorySync` は
file sync + dir sync の durable 経路であり、読み手への原子性
(完成ファイルか旧ファイルかのどちらか)は両等級で維持される。
失敗は `PublishError` が `PublishFailure` の kind/phase/effect/
durability/temp state を構造化して保持し、`Indeterminate` の temp は
削除せず診断へ報告する。

### `al_job::runner`(resume 検証)

`resume()` は manifest 整合チェックの後、`execute()` の前に
Completed/Skipped stage を検証する:

```
for stage in manifest.stages where status in {Completed, Skipped}:
    for id in stage.artifacts:
        record = manifest.artifact(id)  # 不在 → 検証失敗
        path が空 → skip(declared input)
        abs = checked_job_path(job_dir, record.path)
        abs.is_file() → params.byte_len があればサイズ一致を要求
        abs.is_dir()  → params.files の各エントリの実在を要求
        それ以外(不在/型不一致) → 検証失敗
    失敗 → stage.status = Pending、stage.artifacts をクリア、
           対応する ArtifactRecord を manifest.artifacts から除去、
           error/duration_ms をクリア、警告を tracing で記録
1件でも降格があれば manifest.save(job_dir)
```

降格で除去された artifact id を下流 Completed stage の
`inputs`(lineage)が参照して残ることは許容する —
`lineage_of` は不在レコードを静かにスキップし、再実行は当該
stage のみで完結する(決定的再生成により下流成果物は等価)。

### producer 側の検証メタデータ

ArtifactRecord.params(自由 JSON、schema 変更なし)に:

- `byte_len: u64` — 単一ファイル artifact。producer は書込み
  バッファ長を既に持つためコストなし。
- `files: [String]` — ディレクトリ artifact(store ルート)。
  ルートからの相対パスのソート済み列挙。

記録対象:

- `decoded_pcm`(al-media): `byte_len = pcm_bytes.len()`
- `normalized_pcm`(al-locate): `byte_len = bytes.len()`
- `analysis_dataset`(al-dataset): `files` = `analysis/` 以下の
  全ファイルの相対列挙(`writer.finish()` 後に走査)

それ以外の artifact は存在検証のみ(params 変更なし)。

### 等級割り当て(call site)

| 経路 | ファイル | 内容 |
| --- | --- | --- |
| relaxed | al-media/src/stage.rs | `decoded.f64le` |
| relaxed | al-locate/src/normalize.rs | normalized PCM f64le |
| relaxed | al-dataset/src/table.rs | chunk `NNNNNN.bin` |
| relaxed | al-dataset/src/tensor.rs | chunk ファイル |
| relaxed | al-dataset/src/audio.rs | WAV streaming(`stage_file` → `publish_sealed`、BestEffort) |
| durable | al-job/src/manifest.rs | job manifest.json |
| durable | al-dataset/src/manifest.rs | dataset manifest.json |
| durable | al-dataset/src/table.rs | schema.json, index.json |
| durable | al-dataset/src/tensor.rs | array.json |
| durable | al-media/src/stage.rs | metadata/video_features/video_thumbs JSON |
| durable | al-locate/src/stage.rs | persist_json |
| durable | al-dataset/src/writer.rs | write_json |

## Data / Control Flow

1. stage が bulk artifact を `Durability::BestEffort` の publish で書く
   (staged temp→publish、fsync 非要求)。params に検証メタを記録。
2. stage 完了時、runner が manifest を `FileAndDirectorySync` の
   publish で commit(現行どおり) — manifest の durable 記録が
   artifact 書込に先行しない順序規約。
3. 電源断: manifest が Completed を記録した relaxed artifact が
   欠落/切り詰められ得る。
4. resume: 検証が欠落を検出 → stage を Pending 降格 → execute が
   再実行 → artifact 再生成。検証通過後は「manifest 記録 =
   存在かつ完全」が成り立つ。

## Interface Contracts

- `publish_bytes(path, bytes, options) -> Result<PublishReceipt,
  PublishError>`: staged temp → seal → publish。`PublishReceipt` の
  `DurabilityEvidence` が要求保証の達成を示す。
- `Durability::BestEffort` の契約は「原子だが非耐久」— 電源断で
  ファイル全体が欠落または切り詰められ得る。
- `stage_file` は `Write + Seek` sink を返し、streaming producer は
  `seal()` 後 `publish_sealed` で完了する。未 seal で drop された
  staged file の temp は除去される。
- resume 検証の失敗は stage エラーではなく降格である
  (修復可能な状態であり、ユーザーの再実行要求は要らない)。
- `ArtifactRecord.params.byte_len`: 記録されていれば必ず正しい
  値でなければならない(嘘のサイズは resume を壊す)。

## Error Handling

- resume 検証で io エラー(stat 失敗等)は「欠落」として扱い
  降格に倒す — 保守方向は再生成であり、破損を隠さない。
- `checked_job_path` は現行どおり job dir エスケープを拒否する
  (manifest は resume 時点で信頼しない入力)。
- demote 後の manifest.save が失敗した場合は resume をエラー
  終了する(検証結果が揮発して再検出されるだけだが、一貫して
  失敗として扱う)。

## Testing Strategy

- fsutil 単体: `publish_bytes` が内容を正しく書き staged temp を
  残さない、既存ファイルを置き換える(`Replace`)。`CreateNew` は
  `DestinationExists` を分類して報告し、失敗後の temp は recovery
  policy が処理する。
- resume 統合(`crates/al-job/tests/resume.rs`):
  - Completed stage の artifact 削除 → resume で当該 stage のみ
    再実行(他の Completed はスキップ)。
  - `byte_len` 記録 artifact の切り詰め → resume で再実行。
  - dir artifact の `files` 内ファイル削除 → resume で再実行。
- 構造確認: relaxed 経路の呼出点が `Durability::BestEffort` の
  publish を使い、記録系が `FileAndDirectorySync` を維持することを
  レビューで確認。

## Risks / Non-goals

- 「存在するが内容未フラッシュ」(サイズ一致・内容ゼロ化)は
  検出しない — store reader の既存検証が残存リスクとして発火
  (spec の deferred concern)。
- `.rvf-tmp-*` 孤児の定期掃除は行わない — 分類済み失敗・成功時の
  temp は adapter の recovery policy が除去し、`Indeterminate` は
  診断のため保持する。プロセス kill 直後の残存 temp は次回 run で
  無害(読み手は publish 済み destination のみを見る)。
- fsync 削減量の定量計測は行わない(環境依存)。

## Sources and Relations

- `docs/specs/durability-tiered-artifact-writes.md`(要求)
- `docs/adr/artifact-durability-tiers-relaxed-bulk-writes.md`(決定)
- `docs/discovery/durability-tiers-artifact-fsync.md`(調査)
- `docs/designs/ruviform-io-migration-issue-139.md`(StagedFile 移行
  記録・残存分類)

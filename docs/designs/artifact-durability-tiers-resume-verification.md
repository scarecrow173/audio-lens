---
id: "DESIGN-034GsNQAvnJk0iYj6jFzOI"
type: "design"
status: "approved"
title: "Artifact durability tiers and resume verification"
created: "2026-10-15"
updated: "2026-10-15"
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

`al_job::fsutil` に耐久性のない原子書き込み `write_atomic` /
`rename_atomic` を追加し、bulk データ書き込みを切り替える。
`StageRunner::resume` に Completed/Skipped stage の artifact 実在
検証を追加し、欠落を検出した stage を Pending へ降格して再生成
する。検証メタデータ(`params.byte_len` / `params.files`)は
producer が ArtifactRecord に記録する。

## Structure

### `al_job::fsutil`(等級プリミティブ)

```
write_durable(path, bytes)  # tmp+write+sync_all+rename+dir sync (現行)
rename_durable(tmp, path)   # rename + dir sync (現行)
write_atomic(path, bytes)   # tmp+write+rename、fsync なし (新規)
rename_atomic(tmp, path)    # fs::rename のみ (新規)
```

`write_atomic` は `write_durable` と同じ tmp パス・エラー時
cleanup 規律を共有する。fsync を外すだけで読み手への原子性
(完成ファイルか旧ファイルかのどちらか)は維持される。

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
| relaxed | al-dataset/src/audio.rs | WAV streaming(sync_all→除去、rename_atomic) |
| durable | al-job/src/manifest.rs | job manifest.json |
| durable | al-dataset/src/manifest.rs | dataset manifest.json |
| durable | al-dataset/src/table.rs | schema.json, index.json |
| durable | al-dataset/src/tensor.rs | array.json |
| durable | al-media/src/stage.rs | metadata/video_features/video_thumbs JSON |
| durable | al-locate/src/stage.rs | persist_json |
| durable | al-dataset/src/writer.rs | write_json |

## Data / Control Flow

1. stage が bulk artifact を `write_atomic`/`rename_atomic` で書く
   (tmp→rename、fsync なし)。params に検証メタを記録。
2. stage 完了時、runner が manifest を `write_durable` で commit
   (現行どおり) — manifest の durable 記録が artifact 書込に
   先行しない順序規約。
3. 電源断: manifest が Completed を記録した relaxed artifact が
   欠落/切り詰められ得る。
4. resume: 検証が欠落を検出 → stage を Pending 降格 → execute が
   再実行 → artifact 再生成。検証通過後は「manifest 記録 =
   存在かつ完全」が成り立つ。

## Interface Contracts

- `write_atomic(path, bytes) -> io::Result<()>`: `write_durable` と
  同じシグネチャ・同じエラー時 tmp cleanup。契約は「原子だが
  非耐久」— 電源断でファイル全体が欠落または切り詰められ得る。
- `rename_atomic(tmp, path) -> io::Result<()>`: `fs::rename` のみ。
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

- fsutil 単体: `write_atomic` が内容を正しく書き tmp を残さない、
  既存ファイルを置き換える。
- resume 統合(`crates/al-job/tests/resume.rs`):
  - Completed stage の artifact 削除 → resume で当該 stage のみ
    再実行(他の Completed はスキップ)。
  - `byte_len` 記録 artifact の切り詰め → resume で再実行。
  - dir artifact の `files` 内ファイル削除 → resume で再実行。
  - 既存テストの StubStage は path 登録のみでファイルを書かない
    → 検証に抵触するため、stub を実ファイル書込に修正する。
- 構造確認: relaxed 経路の呼出点が `write_atomic`/`rename_atomic`
  を使い、記録系が `write_durable` を維持することをレビューで確認。

## Risks / Non-goals

- 「存在するが内容未フラッシュ」(サイズ一致・内容ゼロ化)は
  検出しない — store reader の既存検証が残存リスクとして発火
  (spec の deferred concern)。
- `.tmp` 孤児の掃除は行わない(inert、再書込で再利用される)。
- fsync 削減量の定量計測は行わない(環境依存)。

## Sources and Relations

- `docs/specs/durability-tiered-artifact-writes.md`(要求)
- `docs/adr/artifact-durability-tiers-relaxed-bulk-writes.md`(決定)
- `docs/discovery/durability-tiers-artifact-fsync.md`(調査)

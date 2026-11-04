---
id: "TSPEC-034GsRevKRO1s5QbTzJ7c8"
type: "test-spec"
status: "approved"
title: "durability-tiers-resume-verification"
created: "2026-10-15"
updated: "2026-11-04"
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
  verifies:
    - "docs/specs/durability-tiered-artifact-writes.md"
    - "docs/designs/artifact-durability-tiers-resume-verification.md"
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# durability-tiers-resume-verification

## Purpose

Issue #55 の耐久性等級(relaxed bulk 書込 + resume 検証)が
「manifest 記録 = 存在かつ完全」の不変条件を守ることを検証する。
等級割当と検証規約が維持される限り保持する。

## Feature

Artifact durability tiers: bulk データは fsync 非要求の原子
publish(`Durability::BestEffort`)、記録系は durable publish
(`Durability::FileAndDirectorySync`)、resume は Completed stage の
artifact 実在を検証して欠落 stage を再生成する。書込経路は
`ruviform-io-file` の StagedFile 契約(`fsutil::stage_file` /
`publish_bytes` / `publish_sealed`)。

## Rules

- Rule: publish は staged temp→destination の原子性を持ち、
  `BestEffort` は fsync/dir sync を要求しない。失敗時に分類済み
  temp を残さない(`Indeterminate` は診断のため保持する)。
- Rule: relaxed tier の呼出点(decoded.f64le、normalized PCM、
  table/tensor chunk、WAV payload)は `PublishOptions::new(
  PublishMode::Replace, Durability::BestEffort)` を使う。
- Rule: 記録系(manifest.json、schema.json、index.json、
  array.json、metadata/provenance JSON)は `Durability::
  FileAndDirectorySync` の publish を維持する。
- Rule: resume は Completed/Skipped stage の `stage.artifacts`
  列挙 artifact を検証する。ファイルは実在(+`params.byte_len`
  があればサイズ一致)、ディレクトリは `params.files` 列挙の
  全実在。
- Rule: 検証失敗した stage は Pending に降格し、その artifact
  レコードを manifest から除去して再実行する。検証失敗は
  stage エラーではなく降格である。
- Rule: producer は検証メタを params に記録する —
  `decoded_pcm`/`normalized_pcm` は `byte_len`、
  `analysis_dataset` は `files`。
- Rule: declared input(path 空)は検証対象外。

## Examples

- Completed stage の relaxed ファイルを削除 → resume → 当該
  stage のみ再実行、他 Completed はスキップ。
- `byte_len` 記録ファイルを短く書き換え → resume → 再実行。
- `analysis/` 配下の chunk を1つ削除 → resume → Dataset stage
  が再実行。
- 正常 run 後の resume は no-op(全 artifact 実在)。

## Guarantees

- 正常系の run で生成されるファイル内容・manifest・provenance は
  現行と同一。
- resume 検証通過後、「stage.artifacts に列挙された artifact は
  存在し完全」が成り立つ。
- relaxed 書込の分類済み失敗時に staged temp が残らない。

## Non-goals

- 存在するが内容が未フラッシュ(サイズ一致・内容破損)の検出。
- `.rvf-tmp-*` 孤児の定期掃除(`Indeterminate` 保持を含む方針は
  設計書のとおり)。
- fsync 削減量の定量計測。

## Risk

- 検証を落とすと電源断後の resume が欠落 artifact を持つ
  Completed stage をスキップし、下流が存在しないファイルを読む。

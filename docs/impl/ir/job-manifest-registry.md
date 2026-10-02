---
id: "IMPL-034yZLFSUeyyN0EpillaV0"
type: "impl"
status: "completed"
title: "job manifest registry"
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
    - "docs/tasks/job-manifest-registry.md"
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
metadata:
  experiments:
    adopted: []
    rejected: []
---

# job manifest registry

## Summary

al-job に不変ジョブ定義、manifest の JSON 永続化・系譜再構成、成果物 id 発行/解決を行う artifact-registry を実装した。

## Context

task `docs/tasks/job-manifest-registry.md`、spec `docs/specs/analysis-job-model.md`、ADR `docs/adr/immutable-resumable-job.md`、design `docs/designs/job-orchestration.md` に従う。

## Implementation

### Changes

- `job.rs`: JobDefinition(不変)・JobInput・input ビルダ。再実行は常に新 JobId
- `manifest.rs`: StageRecord/ArtifactRecord/Manifest。save/load・stage 別成果物紐付け・lineage 再構成。宣言入力は `produced_by: None` の `declared_input` artifact として登録され、下流成果物の `inputs` が元入力を指せる
- `registry.rs`: ArtifactRegistry(id 発行・manifest 参照での絶対パス解決・role 検索)

### Decisions During Implementation

- registry は状態を持たず命名規則のみを担い、正本は manifest に集約
- `ArtifactRecord::path` は「job_dir 相対」契約を維持し、外部入力の
  実際の位置は `params.source_path`/`source_path_abs` に記録する
  (declared_input の `path` は空)。`resolve()` /
  `resolve_by_role()` は空 path の artifact を `None` / スキップ
  とし、job_dir 自体への誤解決を防ぐ
- `JobDefinition::create` は `io::Result` を返し、入力パスの絶対化と
  内容ハッシュ(`source_sha256`)の固定を作成時に必ず行う。
  読めない入力は Job 作成自体が失敗し、ライブラリ経路でも
  unpinned input の Job を作れない。declared_input artifact の
  params にも `source_sha256` を記録し、manifest だけで入力
  バイト列を特定できる。Ingest 側は resume/再実行時に一致を検証する。
  呼び出し側が明示的に設定した `source_sha256` はそのまま保持する
- SHA-256 実装は `al-core::hash` に置く(al-job が入力固定に使う
  ため al-media から移動。al-media は re-export で `sha256_hex` を
  公開)
- serde/serde_json は相互運用目的の依存として採用(コア処理ではない)

## Related Experiments

### Adopted

<!-- Record adopted experiment logs and why they were adopted. -->

### Rejected

<!-- Record rejected experiment logs and why they were rejected. -->

## Validation

<!-- Record the checks that were run. -->

## Risks

<!-- Record remaining risks. -->

## Follow-ups

<!-- Record follow-up tasks or next actions. -->

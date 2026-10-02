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
  (declared_input の `path` は空)。`resolve()` が外部ファイルを
  誤って job_dir 配下に解決しないため
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

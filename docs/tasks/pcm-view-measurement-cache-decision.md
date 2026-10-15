---
id: "TASK-034G5V5nHwcNPj19b1UpuN"
type: "task"
status: "done"
title: "PCM view 計測と job cache 導入判定"
created: "2026-10-15"
updated: "2026-10-15"
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
    - "docs/plans/implement-decoded-pcm-artifact-view.md"
  implemented-by: []
  depends-on:
    - "docs/plans/implement-decoded-pcm-artifact-view.md"
    - "docs/tasks/decode-callers-view-migration.md"
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
    - "docs/test-specs/decoded-pcm-artifact-view.md"
  references: []
  defers: []
  deferred-by: []
---

# PCM view 計測と job cache 導入判定

## Work

1. `decode.rs` cfg(test) に計数 allocator harness を実装し、
   baseline(fs::read+collect+`dsp::deinterleave`)・mmap view・Owned
   view の3経路で `channel_samples` bit-identical を検証。
2. 計測: alloc calls/bytes・wall time(p50/p95、N=30)・RSS・
   fixture(約36MB、実サイズ級は ignored ゲート)・環境を
   `target/pcm-view-evidence/` に記録。mmap は計数 allocator を
   通らないため wall/RSS を併記する。
3. cache 判定: view open が materialize 全体の10%超または e2e 有意
   なら job cache(`Mutex<HashMap<ArtifactId, Arc<PcmView>>>` bounded)
   を検討。小さければ per-call view で閉じる。判定根拠を IR に記録。
4. IR 3件・roadmap Stage 更新・完成 review・commit gate 確認。

## Classification

- `normal-plan-task`

## Upstream Documents

- `docs/plans/implement-decoded-pcm-artifact-view.md`(approved)

## Dependencies

- `docs/tasks/decode-callers-view-migration.md`

## Affected Files

- `crates/al-locate/src/decode.rs`(cfg(test) 計測)
- `docs/impl/ir/` 3件・index・roadmap Stage 表記

## Done When

- [ ] 3経路 bit-identical と計測記録がある。
- [ ] cache 導入可否の判定と根拠が IR に記録される。
- [ ] workspace 全体の fmt/clippy/test/audit が green。

## Verification

- [ ] `mise exec -- cargo test -p al-locate decode::tests`(計測含む)
- [ ] `mise exec -- cargo test --workspace`、`clippy --workspace
  --all-targets -D warnings`、`fmt --check`、audit_docs no findings
- [ ] test-spec Rule 4 を満たす計測証拠。

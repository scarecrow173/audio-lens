---
id: "IMPL-034G6svWmRTPolTX8ICKpS"
type: "impl"
status: "completed"
title: "PCM view 計測と job cache 導入判定"
created: "2026-10-15"
updated: "2026-10-15"
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
    - "docs/tasks/pcm-view-measurement-cache-decision.md"
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

# PCM view 計測と job cache 導入判定

## Summary

baseline(`fs::read` + collect + `dsp::deinterleave`)・mmap view・
Owned view の3経路を 36MB / 368MB stereo fixture で比較計測した。
`PcmView::open`(mmap)は materialize 全体の 0.00-0.04% に過ぎず、
plan の閾値(view open が materialize の10%超、または e2e 有意)を
満たさないため **job-scoped cache は導入せず、per-call `PcmView` で
確定** とした。

## Context

`docs/tasks/pcm-view-measurement-cache-decision.md`(Slice 3)を実施。
`docs/plans/implement-decoded-pcm-artifact-view.md` の既定 —
計測が cache 必要性を示した場合のみ導入を検討し、小さければ
per-call view で閉じる — に従う。

## Implementation

### Changes

- `decode.rs` の `cfg(test)` に thread_local 計数 `GlobalAlloc`
  harness(`tests/ncc_workspace.rs` と同型)・p50/p95 wall 計時・
  best-effort RSS(Linux `/proc/self/status`、Windows `tasklist`
  LIST 形式の最終行)を追加。
- `measure_three_paths` で3経路を warmup 3 + timed 30 iterations
  で比較し、`f64::to_bits` による bit-identical を assert。
- 証拠は `target/pcm-view-evidence/pcm-view-measurement-<fixture>.md`
  に記録(ignored 領域)。36MB は常時実行、368MB は `#[ignore]` ゲート。

### Decisions During Implementation

- **job cache 不採用**(計測根拠): mmap `open` は p50 で 81-86µs、
  materialize の 0.00-0.04%。cache で節約できるのは open syscall
  のみで、支配コストは consumer が必要とする全長 materialize 自体。
  `Mutex<HashMap<ArtifactId, Arc<PcmView>>>` + bounded eviction の
  追加複雑性・Stage 契約上の shared-state 管理コストに見合わない。
- RSS は spawn した `tasklist` の LIST 形式最終行を parse
  (日本語ロケールで見出しが「メモリ使用量」になるためラベル名に
  依存しない実装)。

## Related Experiments

### Adopted

計測結果(Windows x86_64、debug build、N=30・warmup 3):

| fixture | path | open p50 | total p50 | total p95 | allocs | alloc bytes |
| --- | --- | --- | --- | --- | --- | --- |
| 36MB | baseline | - | 248ms | 277ms | 6 | 108MB |
| 36MB | mmap view | 81µs | 219ms | 226ms | 4 | 36MB |
| 36MB | owned view | 12.5ms | 161ms | 218ms | 5 | 72MB |
| 368MB | baseline | - | 2548ms | 2597ms | 6 | 1104MB |
| 368MB | mmap view | 86µs | 2242ms | 2308ms | 4 | 368MB |
| 368MB | owned view | 126ms | 1584ms | 1617ms | 5 | 736MB |

- 3経路の `channel_samples` は `to_bits` で bit-identical(asserted)。
- baseline → mmap view で alloc bytes が 1/3(中間 interleaved
  `Vec<f64>` + 読み込み buffer の除去)、p50 で 1.13-1.14x。
- 観測: 本環境(Windows・debug)の全長 materialize では Owned が
  mmap より p50 で高速(161ms vs 219ms / 1584ms vs 2242ms)。
  連続 `fs::read` が page-fault 駆動のアクセスより速いため。
  ただし mmap は alloc bytes が半分(36MB vs 72MB / 368MB vs 736MB)
  で、ADR の mmap-first 方針(メモリ上限・部分読み取りの将来余地)
  を維持する。release profile・他 OS での逆転確認は将来課題。

### Rejected

- job-scoped `Mutex<HashMap<ArtifactId, Arc<PcmView>>>` bounded cache:
  open コストが閾値を大きく下回り、Stage 契約の hidden shared state
  を持ち込む複雑性に見合わないため不採用。

## Validation

- `cargo test -p al-locate decode::tests`: 計測テスト含め全件パス。
- `cargo test -p al-locate decode::tests::pcm_view_measure --
  --include-ignored`: 36MB・368MB 両 fixture でパス、
  `target/pcm-view-evidence/` に証拠出力。
- 証拠: `target/pcm-view-evidence/pcm-view-measurement-
  fixture-stereo-{36mb,368mb}.md`(ignored 領域)。

## Risks

- debug build の計測であり、release では比率が変わり得る。
  結論(open が materialize の 10% 未満)は 3 桁差があり、profile
  依存では覆らないと判断。
- Owned が全長読みで高速という観測は本環境依存。部分読み取り
  対応(将来)では mmap の優位が変わる。

## Follow-ups

- roadmap の #56 Stage を実装完了に更新し、issue へ結果を記録。
- #54 並列化の際に per-worker scratch のメモリ予算と併せて、
  Owned vs mmap の再評価余地を残す。

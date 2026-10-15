---
id: "DISC-034G5xCv4iLN5jsNmiiY8u"
type: "discovery"
status: "active"
title: "Issue 56 decoded PCM read-only view の適用境界"
created: "2026-10-15"
updated: "2026-10-15"
owners: ["scarecrow173"]
relations:
  source:
    - "remaining-issues-roadmap.md"
    - "https://github.com/scarecrow173/audio-lens/issues/56"
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
  references:
    - "../specs/analysis-dataset.md"
    - "../adr/analysis-dataset-storage.md"
  defers: []
  deferred-by: []
---

# Issue 56 decoded PCM read-only view の適用境界

## Exploration Goal

ロードマップ §#56 の2026-10-14確定判断を、専用の spec/ADR に記録する。
Entry Decision: A-4。要求・選択肢・採用判断は確定済みで、view の所有境界と
API 詳細は次の design review が所有する。

## Key Issues and Assumptions

- `decode_input`(`crates/al-locate/src/decode.rs:311-389`)は codec の再
  デコードではなく、ingest 済み `decoded_pcm` artifact(`decoded.f64le`、
  interleaved f64 LE)を `fs::read` → 8byte 毎の f64 変換+scale 除算 →
  `deinterleave` → `Vec<Vec<f64>>` と3段の全長確保を行う。
- 1 run あたりの `decoded.f64le` 読み出し回数は input 毎に3回:
  CheapAnalysisStage(1回/input)、LocateStage(sources+captures で1回/
  input)、ExpensiveAnalysisStage(1回/input)。さらに `read_alignment_pcm`
  経由の normalized `.f64le` が locate_result 毎に2回
  (ReferenceAnalysis + ExpensiveAnalysis)。residual は
  `al_dataset::read_wav` 経由。
- 消費側は現状すべて materialize 済みの全長 `channel_samples` を使う
  (cheap 解析・`prepare_input` resample・residual 複製)。部分参照は
  `best_window_match` の窓と reference/expensive の range slice で、
  いずれも materialize 後の slice。
- `StageContext` は read-only(`manifest`・`registry`・`config` のみ)。
  `Stage` trait は「stage 間通信は manifest 上の Artifact ID のみ、
  隠れた共有状態禁止」を明記する(`crates/al-job/src/stage.rs:21-24`)。
- 仮説: OS ページキャッシュにより物理ディスク再読は既に回避されている
  可能性があり、実コストは byte→f64 変換 CPU・deinterleave・多重確保の
  allocation churn 側に寄っている。計測で切り分ける。

## Alternatives and Comparison

| Option | Pros | Cons | Lean |
| --- | --- | --- | --- |
| A. read-only PCM view(mmap 第一候補・fs::read fallback)+ 消費側 materialize | 確定判断どおり。不変・artifact 由来で Stage 契約と整合。部分読みの将来拡張を残す | mmap は新依存(memmap2 等)で追加承認が必要。結局 materialize は残る | yes |
| B. deinterleave 済み `Vec<Vec<f64>>` の job 共有 | 変換 CPU を1回に削減 | 全長 f64 PCM の常駐で RSS 大幅増。#54 worker 数分の materialize と重なる。確定判断で棄却済み | no |
| C. stage 間で `DecodedInput` 直接受け渡し | 再読ゼロ | Stage 契約(manifest Artifact ID のみ)違反。resume 単独復元が壊れる。棄却済み | no |
| D. per-stage mmap view のみ(job cache なし) | 依存最小・設計最薄。計測で利益が小さい場合の着地として確定判断が許容 | fs::read との差がなければ最適化効果は限定的 | maybe |

## Tentative Conclusions and Hypotheses

- confirmed(2026-10-14): 永続正本は manifest の `decoded_pcm` artifact
  で Artifact ID が識別子。共有表現は Artifact ID に紐づく read-only
  PCM view(mmap 第一候補、fs::read fallback)。deinterleave 済みは
  キャッシュしない。消費側が必要 ch・範囲を正規化+deinterleave する
  (全長必要ならその範囲だけ materialize)。
- confirmed: Stage 契約を維持し `DecodedInput` の stage 間受け渡しは
  しない。共有するのは不変 PCM view のみ。cache hit/miss・eviction・
  resume で解析結果は不変。materialize PCM と worker scratch には
  別メモリ予算。
- confirmed: `fs::read` vs mmap vs 直接 deinterleave を実測比較し、
  job-scoped cache の利益が小さければ cache 管理を追加せず
  per-stage mmap view のみも許容(計測ファースト)。
- hypothesis: view の正規配置は `al-locate::decode` 層かその依存先
  (`al-dataset`/`al-job`)。`StageContext` を拡張せず関数レベルの
  view 取得に留めるか、registry 側に job-scoped cache を持たせるかは
  design の論点。

## Open Questions

- ~~mmap 実装の依存(memmap2 等)追加は未承認。~~ → **2026-10-15 解決**:
  memmap2 依存追加を承認済み。fs::read fallback も併設する。
- ~~normalized `.f64le` に同じ view を適用する範囲~~ →
  **2026-10-15 解決**: decoded + normalized の両方を対象とする。
  residual WAV(`al_dataset::read_wav`)は対象外。
- job-scoped cache の置き場所(`ArtifactRegistry` 拡張か別オブジェクト
  か)と bounded eviction ポリシー — 計測結果が必要な場合のみ設計。

## Promotion Candidates

- [x] spec-doc needed: read-only PCM view の公開契約(正本・キー・
  materialize 境界・不変性保証)
- [x] adr-doc needed: view の所有境界と cache 導入判定基準
  (mmap 依存承認・計測手順を含む)

## Sources and Relations

- roadmap 確定決定(2026-10-14): `remaining-issues-roadmap.md` §#56
- issue: https://github.com/scarecrow173/audio-lens/issues/56
- dispatch-specific override: 親からの限定文書作成指示に従い、
  briefing-flow の SUBAGENT-STOP を適用し、profile 生成・追加委譲・
  共有 index 更新を行わない。

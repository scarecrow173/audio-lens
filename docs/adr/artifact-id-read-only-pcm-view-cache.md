---
id: "ADR-034G5M7bOVOeKSPQVOhN14"
type: "adr"
status: "accepted"
title: "Artifact ID 紐づけ read-only PCM view と計測ファーストの cache 判定"
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
  implements: []
  implemented-by: []
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "../specs/decoded-pcm-artifact-view.md"
  refines: []
  refined-by: []
  derives-from:
    - "../discovery/decoded-pcm-artifact-view.md"
  derived-by: []
  verifies: []
  verified-by: []
  references:
    - "analysis-dataset-storage.md"
    - "../specs/analysis-dataset.md"
  defers: []
  deferred-by: []
metadata:
  adr:
    decision-makers: []
    consulted: []
    informed: []
---

# Artifact ID 紐づけ read-only PCM view と計測ファーストの cache 判定

## Context and Problem Statement

`decoded.f64le`/`normalized.f64le` artifact が run 内で複数 stage から
`fs::read` → f64 変換 → deinterleave の3段全長確保で読み直されている。
`Stage` 契約は stage 間通信を manifest Artifact ID に限定し、
`StageContext` は read-only のため、共有できるのは artifact 由来の
不変派生表現のみである。本ADRは2026-10-14確定判断と2026-10-15の
memmap2 承認を記録するdraftであり、文書承認は別reviewで行う。

## Decision Drivers

- 再読・変換・確保の繰り返しを削減しつつ、Stage 契約と resume
  独立性を破らない。
- OS ページキャッシュで物理 I/O が既に消えている可能性があり、
  最適化の形は実測で決める(計測ファースト)。
- deinterleave 済み全長 PCM の常駐は RSS 上限を壊すため避ける。

## Considered Options

- A: Artifact ID 紐づけ read-only PCM view(mmap 第一候補・fs::read
  fallback)+ 消費側 materialize。job cache は計測判定。
- B: deinterleave 済み `Vec<Vec<f64>>` の job 共有キャッシュ。
- C: `DecodedInput` の stage 間直接受け渡し(StageContext 拡張)。
- D: 現状維持(関数ごとの `fs::read`)。

## Decision Outcome

Chosen option: A。view は永続正本ではなく Artifact ID を識別子とする
派生表現とし、mmap を第一候補・fs::read owned buffer を fallback と
する。memmap2 依存追加は 2026-10-15 に承認済み。対象は
`decoded_pcm` と `normalized_pcm` の f64le artifact で、residual WAV
は含めない。job-scoped bounded cache は `fs::read` vs mmap vs 直接
deinterleave の比較計測(alloc 量・変換 CPU・peak RSS・e2e)で利益が
示された場合のみ追加し、小さければ per-stage view のみで閉じる。
B/C は Stage 契約・RSS 境界に反するため棄却。D は確定判断に反する。
view の所有境界・mmap 詳細・eviction ポリシーは次の design review
で確定する。

### Consequences

- 不変 view の共有は Stage 契約と整合し、resume は manifest+artifact
  から独立して復元できる。
- materialize の値・スケール・deinterleave 順は現行と bit-identical
  でなければならず、view 導入は数値面で中立である。
- memmap2 の新依存が増えるが、fallback により mmap 不可環境でも
  同一結果を返せる。
- mmap 参照中のファイル変更は UB リスク(SIGBUS 等)を持つため、
  「durable rename 以後の artifact は不変」という artifact store の
  既存前提を view の安全性条件として design に明記する。

### Confirmation

spec の bit-identical 等価性・fallback 同一性・resume 独立性・計測
記録を確認する。cache 導入可否は計測後に決定し、未計測の性能 claim
は行わない。

## Implementation Plan

- `crates/al-locate/src/decode.rs`: `decode_input`・
  `read_normalized_record` の読み出しを view 経由に置き換える。
- view 実装の配置(al-locate::decode か共通層か)は design で確定する。
- `crates/al-dataset`/`al-job`: 必要なら view の共有型・job 境界を配置。
- 計測: fs::read/mmap/直接 deinterleave の比較。fixture・環境・shape
  を記録する。
- 避ける pattern: deinterleave 済み全長の常駐、stage 間直接受け渡し、
  部分読み取りへの全面改修、TLS、未計測の性能 claim。

## Verification

- [ ] view 経路と現行経路の `DecodedInput`/`PreparedInput` が
  bit-identical であること(decoded/normalized 双方)。
- [ ] mmap 不可時 fallback が同一結果を返すこと。
- [ ] `fs::read`/mmap/直接 deinterleave の比較計測を記録し、cache
  導入判定の根拠を残すこと。
- [ ] resume 境界で manifest+artifact から独立復元できること。
- [ ] 既存の manifest/artifact 検証が同じエラーを返すこと。

## More Information

共通 discovery anchor と関連 spec に scope・証拠を記録。部分読み取り
対応・residual WAV・codec 再デコードは将来別 Issue で判断する。

## 2026-10-15 Decision Review

spec と同一 review セッション。Judge ACCEPT WITH NON-BLOCKING NOTES
(mmap 参照中のファイル変更リスクを Consequences に反映済み)。
coordinator が意思決定のみ承認。

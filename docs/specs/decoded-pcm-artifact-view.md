---
id: "SPEC-034G5M6Djv4xJOZKHXQniH"
type: "spec"
status: "approved"
title: "decoded/normalized PCM read-only view"
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
    - "../adr/artifact-id-read-only-pcm-view-cache.md"
  refines: []
  refined-by: []
  derives-from:
    - "../discovery/decoded-pcm-artifact-view.md"
  derived-by: []
  verifies: []
  verified-by: []
  references:
    - "analysis-dataset.md"
    - "../adr/analysis-dataset-storage.md"
  defers: []
  deferred-by: []
---

# decoded/normalized PCM read-only view

## What Should Be Built

`decoded_pcm` および `normalized_pcm` artifact(f64le interleaved)を、
run 内で繰り返し読む stage 境界に、Artifact ID に紐づく read-only の
PCM view を導入する。view は永続正本ではなく artifact の派生表現で、
消費側が必要なチャンネル・範囲を正規化+deinterleave して materialize
する。view の取得は mmap を第一候補とし、利用不可環境では fs::read
ベースの owned buffer に fallback する。

## Why Now

`decoded.f64le` は1 run あたり input 毎に3回(Cheap/Locate/Expensive)、
normalized `.f64le` は locate_result 毎に2回(Reference/Expensive)
`fs::read` → f64 変換 → deinterleave の全長3段確保で読み直されている。
137MB 級の入力で変換 CPU と allocation churn が繰り返し発生する。
ロードマップ#56の2026-10-14確定判断を実装可能な契約に落とす。

## Users and Value

解析利用者は同一入力の再読コスト削減による e2e 短縮と peak RSS 低下を
得る。実装者は「artifact が正本・view は派生」という寿命境界を明示
でき、将来の部分読み取り対応の土台を得る。

## Scope

### In Scope

- `decoded_pcm`/`normalized_pcm` の f64le artifact を対象にした
  read-only PCM view(mmap 第一候補・fs::read fallback、memmap2 追加は
  2026-10-15 承認済み)。
- `decode_input`・`read_normalized_record` の読み出し経路を view 経由に
  置き換え、scale 正規化・deinterleave・有限性検証の現契約を維持する。
- `fs::read` vs mmap vs 直接 deinterleave の実測比較(alloc 量・変換
  CPU・peak RSS・e2e)。job-scoped bounded cache は計測で利益が
  示された場合のみ追加する。

### Out of Scope

- deinterleave 済み `Vec<Vec<f64>>` のキャッシュ。
- `DecodedInput` の stage 間直接受け渡し・`StageContext` の共有可変状態。
- 解析アルゴリズムの部分読み取り対応への改修。
- residual WAV(`al_dataset::read_wav`)経路。
- codec 再デコード(別 Issue #36/#42)・TLS キャッシュ・scratch pool。

## Requirements

- 永続正本は manifest の `decoded_pcm`/`normalized_pcm` artifact で
  Artifact ID が識別子。view は read-only で、書き込み面を公開しない。
- `Stage` trait 契約(manifest Artifact ID のみの通信・隠れた共有状態
  禁止)を維持する。共有するのは artifact 由来の不変 view のみ。
- cache hit/miss・eviction・mmap 利用可否・resume 経路で解析結果が
  不変であること。resume は manifest+artifact から独立して復元する。
- materialize した PCM と worker scratch には別メモリ予算を設ける。
  mmap だけでは RSS 上限を保証しないことを契約に明記する。
- mmap が利用できない環境では fs::read fallback で同等の結果を返す。
- 消費側 materialize の数学的結果(値・スケール・deinterleave 順・
  有限性検証)は現行と bit-identical であること。

## Acceptance Criteria

- [ ] `fs::read`/mmap/直接 deinterleave の比較計測を実施し、
  alloc 量・変換 CPU・peak RSS・e2e を fixture・環境付きで記録する。
- [ ] view 経路と現行 `fs::read` 経路の `DecodedInput`/`PreparedInput`
  が bit-identical であることを検証する(decoded/normalized 双方)。
- [ ] mmap 不可時の fallback が同一結果を返すことを検証する。
- [ ] job-scoped cache を導入する場合、その keyed-by-Artifact-ID・
  bounded・eviction 後も結果不変であることを検証する。導入しない
  場合は計測根拠を記録する。
- [ ] resume 境界で view/cache に依存せず manifest+artifact から
  復元できることを確認する。
- [ ] 既存の manifest/artifact 検証(path・lineage・有限性)が view
  導入後も同じエラーを返すことを既存テストで確認する。

## Deferred Design Concerns

view の所有境界(registry 拡張か関数レベルか)、cache eviction ポリシー、
mmap 長・alignment の扱い、測定 fixture・判定閾値の具体化は次の
design review の必須項目。実装開始前に確定する。
mmap の安全性条件(「durable rename 以後の artifact は不変」)を
design で明文化する。

## Sources and Relations

共通 discovery anchor に確定判断と計測前提を記録。analysis-dataset は
上流制約の参照であり、既存文書全体を supersede しない。
memmap2 依存は 2026-10-15 に承認済み。対象は decoded + normalized の
f64le artifact で、residual WAV は含めない(同日承認)。

## 2026-10-15 Decision Review

Reviewer A: 確定判断の網羅性・内部整合を確認。Reviewer B: 3候補
(mmap 中のファイル変更・byte/空/BE 等価性・alignment 全経路)。
Validator: IC-1 CONFIRMED(design で不変性前提を明文化、非ブロッキング)、
IC-2/IC-3 は既存 acceptance でカバー済みのため REJECTED。
Judge ACCEPT WITH NON-BLOCKING NOTES(IC-1 を Deferred Design Concerns
に反映済み)。coordinator が意思決定のみ承認。bit-identical・計測・
resume の実証は未実施であり、この承認に含まれない。

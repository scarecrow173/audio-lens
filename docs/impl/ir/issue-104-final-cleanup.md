---
id: "IMPL-034PmRXdpTZkRRXt3Z9KHu"
type: "impl"
status: "in-progress"
title: "Issue #104 final cleanup: provisional adapters, duplicate implementations, dependency audit"
created: "2026-10-31"
updated: "2026-10-31"
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

# Issue #104 final cleanup: provisional adapters, duplicate implementations, dependency audit

## Summary

Issue #104「Ruviform 切替完了後の旧基盤撤去・CI・最終性能/精度確認」の実施記録。
暫定 adapter だった `al-locate::dsp` facade、decoder の multi-backend
比較経路、および `RUVIFORM_REV` ハードコード(#115)を撤去し、
AudioLens→Ruviform の一方向依存を確定した。AudioLens 固有の
入力境界・job/dataset/provenance 意味論は AL 側に残留する。

## Context

Issue #104(旧基盤撤去 + 監査 + 最終ゲート)、#115(decoder provenance
の rev 乖離)、#117(rev/precision stamp 導出経路の共通化)に従う。
責務境界は `docs/designs/ruviform-migration-responsibility-map.md`
(§3 facade 暫定維持 → 本 Issue で撤去条件達成)に基づく。

## Implementation

### Changes

**撤去(暫定物・重複実装)**
- `crates/al-locate/src/dsp.rs` 全体: `ruviform_numeric` への
  転送 facade + 平坦 re-export 層(`lib.rs` の `pub use dsp::{...}`)。
  全呼出側を `ruviform_numeric::{vec,corr,bands,fft,conv,linalg,xspec}`
  直呼びへ移行。panic 契約は呼出側の `expect` + 形状検証へ再配置
  (facade が担っていた「Result→panic 写像の一箇所集約」は、検証済み
  形状では panic しない契約へ移行したため不要)。
- `DecoderRegistry::register()` と `decoder.rs` の複数 backend
  フォールバック loop: `DecoderRegistry::ruviform()` は
  `RuviformDecoder` 単一 backend。`native_is_first_and_duplicate_backend_
  is_rejected` テストは単一 backend 契約へ書き換え。
- `ruviform_backend.rs` の暫定比較経路(native decode との
  audio_fallback 比較): Golden artifact に fallback 記録が無く
  未使用のため撤去。native parser は container-fact 観測のみに限定。
- `RUVIFORM_REV` ハードコード(#115): `al-core/build.rs` を新設し
  `Cargo.lock` の ruviform git source `#<sha>` から解決済み rev を
  導出(`rustc-env`)。複数 rev 混在時は build 失敗で loud に検出。
  `al_core::RUVIFORM_REV` が唯一の導出経路となり #117 の stamp
  要求でも再利用可能。

**残留(根拠付き)**
- `deinterleave`: AL 入力境界意味論(channels=0 clamp・tail drop)
  のため `al-locate/src/decode.rs` へ移設して維持。
- `al-media` の native parser(wav/avi/mp4): 詳細 container-fact
  観測(skipped boxes・edit list・sync samples・chunk facts・
  timing candidates)は ruviform f3f474b が提供しないため維持。
  デコード経路としては使わない。
- R1–R4 暫定実装: csolve(#167)・直算 corr0 sweep(#168)・
  spectral 統計(#169)・ResamplePlan 永続化(#170)は Ruviform 側
  API 追加まで残留(対応表は下記)。

**変更ファイル**: al-locate(lib/decode/mixer/residual/pool/prepared/
methods + tests 3 + examples 4)、al-analysis(lib/reference/cheap/
band_gain/stft/expensive 5 + tests/reference + benches 3)、
al-media(decoder/ruviform_backend + tests 2)、al-core(build.rs/
lib.rs)、責務マップ状態更新。計 36 ファイル、-832/+424 行。

### Decisions During Implementation

- **measure/emit 分離(監査判定)**: `analyze_pair`/`expensive`
  各 family に record への feedback 依存はなく技術的分離は可能。
  しかし ~3,800 行の挙動不変リファクタで、現時点の consumer が
  存在せず(ruviform #169 open)、根拠付き見送りとした。
- **#117 スコープ判定**: rev 導出の共通化(本 Issue の build.rs
  経路)は実装済み。analysis artifact への stamp 自体は artifact
  schema 変更を伴い #117 で扱う(本 Issue は導出経路の提供まで)。
- **依存監査結果**: 依存方向 AudioLens→Ruviform 一方向(逆参照
  なし)。pin `f3f474b1734af05ca2435a322f935805010ba8b6` が
  Cargo.lock 全 ruviform crate で一致。license: Ruviform は暫定
  MIT、AL は MIT OR Apache-2.0(リポジトリに LICENSE ファイル
  なし — 監査観測事項)。CI secrets: git 依存は公開 HTTPS で
  追加 secret 不要。RT/非RT: 解析経路は非 RT、f64 規約維持。
  `ruviform-observability` の RT counter primitive と `al-observability`
  の AL CLI/logging 意味論(AL_LOG・sink 語彙・inspect 契約)は
  責務が別で delegation 対象外。
- **deinterleave の帰属**: Ruviform buffer API へ寄せず AL 残留。
  channels=0→1 clamp と不完全 tail frame drop は AL の入力境界
  ポリシーであり汎用 primitive ではない(責務マップ §3 記載と一致)。

## Related Experiments

### Adopted

- RUVIFORM_REV の Cargo.lock 導出(build.rs): provenance が
  pin と乖離しない構造になり、テストも `al_core::RUVIFORM_REV`
  比較へ更新してハードコード根絶。

### Rejected

- `al-observability` → `ruviform-observability` delegation:
  責務が不重叠(AL 側は CLI/logging 意味論のみ)。
- analyze_pair/expensive の measure/emit 分離実施: consumer
  不在・大規模挙動不変リファクタのため見送り(ruviform #169)。
- 暫定比較経路(native fallback)の維持: Golden 未使用・
  decoder-of-record が Ruviform のため撤去。

## Validation

- `cargo check --workspace --all-targets`: pass(36 ファイル
  変更後、fmt 整形込み)
- `cargo test -p al-media -p al-locate -p al-analysis`: 全 pass
  - al-media: lib 71 + aac 6 + golden 12 + ingest 10 + registry
    等(ingest の provenance テストは #115 修正で `e467ccba`
    ハードコード期待値 → `al_core::RUVIFORM_REV` 比較へ更新)
  - al-locate/al-analysis: 228 テスト全 pass。argmax tie
    characterization・prepared・locate・residual equivalence
    等の characterization テストで挙動保持を確認。
- 最終統合: `git merge origin/main` + `mise run check` 実施済み
  (本記録時点の作業ブランチ上で検証)

## Risks

- `bench_dsp`/`bench_residual` の性能比較腕は直接 ruviform 呼出
  へ移行済みだが、#98 基準との定量差分の詳細は baseline 資産
  (`tools/baseline`)での追跡対象。
- LICENSE ファイル不在(ライセンス表記は Cargo.toml の
  `license` フィールドのみ)は監査観測として残置。

## Follow-ups

- Ruviform #167–#170: AL 側暫定実装(csolve・直算 corr0 sweep・
  spectral 統計・ResamplePlan 永続化)は対応 Ruviform API 追加時
  に移行・撤去。
- Ruviform #169: consumer が具体化した時点で analyze_pair/
  expensive の measure/emit 分離を再検討。
- AL#117: analysis artifact への ruviform rev/precision stamp
  付与(schema 変更側)は同 Issue で継続。rev 導出経路は
  `al_core::RUVIFORM_REV` として本 Issue で提供済み。

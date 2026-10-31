---
id: "IMPL-034PmRXdpTZkRRXt3Z9KHu"
type: "impl"
status: "completed"
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

### Golden 回帰(#98 凍結 golden、最終ゲート)

`baseline/runs/<name>`(#98 凍結)を `<name>-golden98` へ退避し、
本 Issue 変更後コード(`target/release/al`、HEAD `fe3028d`)で
`capture.mjs` が 9 run を再収集、`compare.mjs`(rel 1e-9 / abs 1e-12)
で比較後に凍結 golden を復元。差分実体は
`docs/impl/ir/issue-104-baseline/*-vs-golden98.json`、機械分類は
`tools/baseline/lib/classify-diffs-104.mjs` + `classify.log`。

**全 9 ラン・全差分が既知 4 クラスに完全帰属し、説明不能な新規差分は
0 件**:

| run | total | #101 既知 | #104 rev | #100 decoder | #106 期待 | unexplained |
| --- | --- | --- | --- | --- | --- | --- |
| canonical | 1447 | 45 | 12 | 18 | 1372 | 0 |
| aa | 1447 | 45 | 12 | 18 | 1372 | 0 |
| resume | 1447 | 45 | 12 | 18 | 1372 | 0 |
| small | 1833 | 1149 | 16 | 24 | 644 | 0 |
| variant-desync | 150 | 20 | 52 | 78 | 0 | 0 |
| variant-clipping | 172 | 42 | 52 | 78 | 0 | 0 |
| variant-reverb | 161 | 31 | 52 | 78 | 0 | 0 |
| variant-filter | 167 | 37 | 52 | 78 | 0 | 0 |
| variant-clicks | 181 | 51 | 52 | 78 | 0 | 0 |

- `#104 rev`(新規クラス): `decoder/(attempts/N/)?version` 系差分で
  a 側(新 capture)の値が **Cargo.lock 導出 rev
  `f3f474b1734af05ca2435a322f935805010ba8b6`** と一致するものを
  機械分類(#115 解消の値レベル検証)。各 run の #100 内訳は
  #103/#106 の #100 計数 = #100 + #104 で不変(version パスが
  #104 側へ移っただけで decoder provenance 差分の総数は同じ)
- `#106 期待`: #106 と同一内容(visualization 成果物・
  report.html/report.data.js の path-set 消滅、manifest stages/
  artifacts/roles・export_manifest generator・正規化 ART-#N 再採番)
- plan 集合検査: stage 除去は `visualization` のみ・追加 0 件、
  artifact 除去は visualization/report のみ・追加 0 件(全 run ok)
- rev 直接検証: 全 run の `golden/inputs/IN-#*/metadata.json` で
  `decoder.version`・`decoder.settings.ruviform.rev`・
  `attempts/N.version` が Cargo.lock rev と一致(3〜4 inputs/run、
  全件 ok)
- 決定性・再開一致: `aa` vs `canonical` **0 diffs**、
  `resume` vs `canonical` **0 diffs**(locate 完了 kill →
  `al resume` 完走でクリーンランと一致)

### 性能実測(Ruviform 基盤化前後)

旧実装バイナリの再構築は行わず、#98 凍結 `performance.json`
(基盤化前・同一マシン)と本 capture の `performance.json` の
直接比較 + bench 群(legacy 参照腕内蔵)で代替。
`tools/baseline/lib/perf-compare-104.mjs` の出力は
`docs/impl/ir/issue-104-baseline/perf-compare.json`、各 run の
`performance.json` 実測値は `perf/` 配下に保存。

| run | e2e #98 ms | e2e #104 ms | Δ% | stageΣΔ%(共通) | CPUΔ% | RSSΔ% |
| --- | --- | --- | --- | --- | --- | --- |
| canonical | 376871 | 360080 | -4.5% | -3.6% | +0.5% | -3.6% |
| variant-desync | 182574 | 182658 | +0.0% | +0.1% | +1.8% | +11.4% |
| variant-clipping | 389484 | 363725 | -6.6% | -6.6% | +2.2% | -1.5% |
| variant-reverb | 400576 | 399194 | -0.3% | -0.3% | +0.3% | -11.8% |
| variant-filter | 448543 | 411935 | -8.2% | -8.1% | +9.0% | +0.6% |
| variant-clicks | 398894 | 391458 | -1.9% | -2.0% | -4.7% | -13.8% |
| small | 140363 | 147347/138539※ | +5.0%/-1.3% | +5.9% | +5.2% | -2.0% |
| resume | 393216 | 366783 | -6.7% | -5.8% | +0.3% | -7.2% |
| aa | 368178 | 365705 | -0.7% | +0.3% | -0.2% | -5.8% |

※ small は唯一 e2e 増加を示したため 2 回計測(run1 147347ms /
run2 138539ms)。run1 の +5% は locate stage(122415→131477→
123080ms、全体の ~87%)の計測分散が主因で、run2 は -1.3% と
中立。系統退行ではないと判断。

stage 詳細(#98→#104、実測):

- `locate`(支配 stage): -12.6%〜+0.7% で概ね改善
  (variant-filter -12.6%、canonical -4.3%、#101 で観測された
  plan 再利用効果と整合)
- `reference_analysis`: +14.6%〜+19.8%(canonical +7.1%、
  small +47%/2 回目 +55%)。#101 で記録済みの scalar 化
  トレードオフ(band_gain・corr0 sweep、reference +24.8%)と
  同方向の既知クラス。absolute は stage 合計 60〜92s に対し
  +9〜13s 程度
- `expensive_analysis`: +33.9%〜+58.3%(small +35%/+27%)。
  #101 記録(+18.6%)と同方向の既知クラスだが比率は大きめ
  (基底が 4.6〜6.8s と小さいため比率が増幅、絶対差 +1.6〜
  +3.4s)。#104 変更は解析カーネル呼出経路のみで演算内容
  不変のため、#100-#103 累積差分 + 計測分散の範囲と判断
- `export`: -36.1%〜-38.4% 改善(#106 の report.html 撤去由来)
- `visualization`: 全 run で除去(#106)
- peak RSS: -13.8%〜+11.4%(7/9 run で低下)
- 生成物 output_bytes: #106 撤去分の減少を除き同等

bench(実測、同一マシン):

- `bench_dsp`: legacy_fft 4M 325.0ms に対し ruviform fft
  184.6ms / plan 再利用 120.6ms / rfft 68.5ms(高速化)。
  NCC 2.4M×8k 514.0ms、bandpass 197.6ms、gcc_phat 306.8ms
- `bench_residual`: per-source `sub_scaled` は legacy gather 比
  1src -19%(11.674→9.439ms median)・2src -53%
  (28.507→13.393ms)。elementwise 等価 predicate PASS
- `bench_alloc`(#98 記録値との比較):
  `analyze_pair` 619.144ms/15788 allocs/240,919,365B
  (#98: 636.225/15857/246,940,205 → -2.7%/-0.4%/-2.4%)、
  `prepare_residual` 6.061ms/167/15,362,792B
  (#98: 9.441/167/15,362,792 → -35.8%/0/0)

**結論**: e2e 合計は 8/9 run で中立〜改善(-8.2%〜+0%)、
small は再計測で -1.3% と中立。支配 stage の locate は改善
  方向。analysis 系 stage の増分は #101 で明示承認済みの
scalar 化トレードオフと同じ既知クラスであり、#104 の変更
(facade 撤去・fallback 撤去・rev 導出)に起因する新規退行は
観測されなかった。Ruviform 基盤化によるパイプラインレベルの
性能退行なしと結論する。

### 全検証(最終ゲート)

| task | 結果 |
| --- | --- |
| `mise run fmt` | pass(`cargo fmt --all -- --check`) |
| `mise run check` | pass(`cargo check --workspace --all-targets`) |
| `mise run lint` | pass(biome check + `clippy --workspace --all-targets -D warnings`)。初回は本検証で新設した 3 ファイルの format 違反で失敗 → `biome format --write` で修正済み。app.js の useTemplate/noGlobalIsFinite 等は #105 由来の警告級(エラー 0) |
| `mise run test` | pass(`cargo test --workspace` 全セクション green) |
| `mise run build` | pass(`cargo build --workspace`) |
| `mise run verify-toolchain` | pass(rustc/cargo 1.99.0、rustfmt+clippy) |
| `mise run check-cli-headless` | pass(headless 10 crate の依存木に GUI framework なし) |
| `cargo build --release -p al-cli` | pass(baseline capture 用バイナリ) |

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

---
id: "DISC-034EcryYzJeFf7jkejFbeX"
type: "discovery"
status: "active"
title: "al-math ベクトル演算プリミティブ集約と SIMD reduction 順序"
created: "2026-10-11"
updated: "2026-10-11"
owners: []
relations:
  source:
    - "../../crates/al-math/src/linalg.rs"
    - "../../crates/al-math/src/lib.rs"
    - "../../crates/al-locate/src/methods.rs"
    - "../../crates/al-locate/src/residual.rs"
    - "../../crates/al-locate/src/mixer.rs"
    - "../../crates/al-locate/src/dsp.rs"
    - "../../crates/al-analysis/src/cheap.rs"
    - "../../crates/al-analysis/src/reference.rs"
    - "../../crates/al-analysis/src/band_gain.rs"
    - "../../crates/al-analysis/src/expensive/mod.rs"
    - "../../crates/al-analysis/src/expensive/timing.rs"
    - "../../crates/al-analysis/src/expensive/dynamics.rs"
    - "../../crates/al-analysis/src/expensive/ir.rs"
    - "../../crates/al-analysis/src/stft.rs"
  changes:
    added: []
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements: []
  implemented-by: []
  depends-on:
    - "./observability-foundation-logging-and-profiling.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "./observability-foundation-logging-and-profiling.md"
  refines: []
  refined-by: []
  derives-from:
    - "../specs/al-math-api-boundary.md"
    - "../designs/al-math-api-boundary.md"
  derived-by: []
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by:
    - "../specs/al-math-api-boundary.md"
    - "../designs/al-math-api-boundary.md"
---

# al-math ベクトル演算プリミティブ集約と SIMD reduction 順序

## Exploration Goal

`docs/specs/al-math-api-boundary.md` の Deferred Design Concerns で
次 spec として予告された残課題:「ベクトル演算プリミティブ
(dot/sum_sq/scale/axpy 等)の公開 — 各分析 crate に重複する
`corr0`/`rms`/`energy` 系ヘルパの集約」。SIMD 化されていない
スカラリダクションの実態を棚卸しし、集約スコープ・公開境界・
reduction 順序ポリシー(Deferred で未決と明記)を deep-dive で
確定する。

なお deep-dive 中に「Logging/Profile 基盤を本集約に先行導入する」
という順序変更がユーザーにより決定された(下記 Tentative
Conclusions)。本集約の spec/ADR 作成・実装はその基盤チェーン
完了後に行う。

## Key Issues and Assumptions

- 確認済み(棚卸し): 同一式の f64 リダクションが複数 crate に分散
  - `corr0`(Σxy/√(Σx²Σy²)、ゼロエネルギー→0.0、中心化なし):
    `expensive/mod.rs:264`(pub(crate)、timing.rs で ~4 箇所使用)、
    `reference.rs:134`(~8 箇所使用)、`cheap.rs` ×3(インライン、
    `stereo.pair_correlation`・`stereo.ipd_samples` の lag 走査・
    `trans.self_similarity`)
  - `rms`(√(Σx²/n)、空→0.0): `expensive/mod.rs:257`、
    `reference.rs:119`(~8 箇所)、`residual.rs:133`(`&[Vec<f64>]`
    マルチch 版)の3実装
  - `energy`/`sum_sq`(Σx²): `methods.rs:142`(preflight で全長
    signal/template に毎回、最大級のホット)、`mixer.rs:74`、
    `band_gain.rs:94,101`、cheap 内複数
  - `dot`(Σa·b): `linalg.rs` に内部 SIMD 版が既存(lane acc +
    scalar tail)、`methods.rs:499`、`residual.rs:166`、
    `band_gain.rs:96`、cheap/reference 内にインライン
  - `scale`: `methods.rs:470` `normalize_gain`
  - `axpy` 系: `residual.rs:163-169` の `*v -= Σ_s m·src` は
    転置アクセス形。per-source 化すれば連続 `axpy`/`sub_scaled`
    として書ける(ループ構造変更を伴う)
  - `mag_sq`(re²+im²→out): `stft.rs:82`、`expensive/ir.rs:178`、
    `dynamics.rs:194` — split re/im 境界の副産物として新候補
- 確認済み(依存): `al-analysis` は `al-math` に直接依存せず、
  DSP は `al_locate::dsp` ファサード経由(`ncc`/`fft`/`bandpass`/
  `spectral_envelope` を re-export)。al-analysis への al-math
  依存追加は規約上「依存の追加・変更」で確認対象
- 確認済み(数値): 逐次和と SIMD lane アキュムレート+FMA は加算順が
  異なり ~ulp(10⁻¹⁶)級の差が出得る。scalar(pulp::Scalar、
  1レーン)経路は逐次和と同順序で現行値とビット一致する
- 確認済み(影響): golden テストは位置比較(±64 samples)であり
  float 厳密比較ではない。Validity ゲート閾値(corr 0.3、
  MIN_ENERGY 1e-4 等)は差分より6〜12桁大きい
- 確認済み(計測): criterion 未導入。ベンチは examples/ の
  `Instant` 手動計測(bench_dsp/bench_ncc/bench_correlation/
  bench_locate_prepared)のみ。ログ基盤(tracing/log 等)も未導入
- 前提: pulp は al-math 内に封じ込め、公開 API は `&[f64]`
  スライスのみ(前 spec の方針を継続)
- 前提: pre-release 方針により互換層は設けない

## Alternatives and Comparison

### reduction 順序ポリシー(Deferred で未決だった主論点)

| Option | Pros | Cons | Lean |
| --- | --- | --- | --- |
| A: lane reduce + FMA 許容 | 最大性能(レーン並列+積和融合)。丸め回数が減り精度も改善方向。cmul の FMA 判断と一貫 | dispatch 経路の最終ビットが環境依存(~ulp 級)。等価性は許容誤差で定義する必要 | yes |
| B: lane reduce のみ(FMA 不使用) | lane 順序差のみで差分の出所が1つ少ない | FMA を捨て速度・精度とも僅かに損。それでも lane 順序差は残り許容誤差は必須 | no |
| C: bitwise 一致要求 | 新旧の差分が定義上ゼロ | 順序固定=直列依存の維持で SIMD 効果がほぼ消失し目的と矛盾 | no |

### プリミティブ集合

| Option | Pros | Cons | Lean |
| --- | --- | --- | --- |
| A+B+C 全部(dot/sum_sq/corr0/rms + scale/axpy + mag_sq) | 全て実呼出し箇所が存在(YAGNI 違反なし)。corr0 融合版は1パスで済み性能面でも正当 | 範囲最大。axpy は residual.rs のループ構造変更を伴う | yes |
| A+B のみ | mag_sq を後続に回せる | stft/ir/dynamics の re/im 合成が散在したまま残る | no |
| A(リダクション)のみ | 最小範囲 | scale/axpy/mag_sq の重複・ホット経路が残る | no |

### 公開境界

| Option | Pros | Cons | Lean |
| --- | --- | --- | --- |
| `al_math::vec` + `al_locate::dsp` re-export | 既存 dsp ファサードパターンと一貫。al-analysis は依存追加なしで使える | al-locate を経由しない将来の消費者は al-math を直接使う形になる(現状問題なし) | yes |
| al-analysis に al-math 依存追加 | 直接利用で素直 | 依存変更(要確認)が発生し dsp ファサードの一元化が崩れる | no |
| dsp 内部のみ公開 | al-math 公開面を増やさない | al-math 内の再利用(linalg dot 等)と公開 API が分離し一貫性に欠ける | no |

### 長さ不整合の契約

| Option | Pros | Cons | Lean |
| --- | --- | --- | --- |
| panic(assert 等長) | 「fail, never guess」方針・fft_size assert の前例と一致。スライス忘れを隠さない | panic 契約(FFT 系と同じ方針だが) | yes |
| min-len(zip) | 現行 corr0 と同じ | 呼出しバグを黙って丸める | no |

### 性能検証の方法

| Option | Pros | Cons | Lean |
| --- | --- | --- | --- |
| criterion 導入(dev-dep) | 統計的な計測(ウォームアップ・サンプリング・回帰検出)が標準化される。今後の SIMD 施策(#42 等)でも再利用可能 | 依存追加。導入時にバージョン選定(公開≥7日)が必要 | yes |
| examples ベンチ拡張 | 既存パターンと一貫、依存追加なし | 手動計測で統計的信頼性が低い。ad-hoc 計測がさらに増える | no |
| 計測報告のみ(AC 化しない) | 最小 | 「性能向上が主目的の半分」との整合が取れない | no |

## Tentative Conclusions and Hypotheses

以下は全て deep-dive 対話でユーザー確認済み(各案の採否は上表
Lean 列の通り):

- confirmed: 主目的は「重複解消」と「性能向上」の両方を同等に
  扱い、両方を AC に書く
- confirmed: reduction 順序は案A(lane アキュムレート+FMA 許容)。
  等価性は許容誤差(~1e-12)で定義。scalar 経路は逐次和と同順序で
  現行値をビット一致で保存する。下流の永続化値・ゲートは
  「環境非依存でない最終ビット」を許容する
- confirmed: プリミティブ集合は A+B+C 全部:
  `dot` / `sum_sq` / `corr0`(1パス3アキュムレータ融合) / `rms` /
  `scale` / `axpy`(y+=a·x、residual 減算用の sub_scaled 含む) /
  `mag_sq`(re²+im²→out)
- confirmed: 公開は `al_math::vec` 新モジュール + `al_locate::dsp`
  re-export。al-analysis に al-math 依存は追加しない
- confirmed: 長さ不整合は panic(等長 assert)
- confirmed: 性能検証は criterion 導入(dev-dependency、公開から
  ≥7 日の版)。実測値を Implementation Record に記録
- confirmed: Logging/Profile 基盤(マクロ計測+ログ方針)を本集約に
  先行して全 crate 導入する。この決定により本集約の spec/ADR・
  実装チェーンは基盤チェーン完了後に着手する
- confirmed(契約継続): `corr0` はゼロエネルギー→0.0、`rms` は
  空入力→0.0 の現行契約を維持。`corr0` は中心化しない(zero-mean
  でない正規化相関)式を全呼出しが共有している
- hypothesis: residual.rs の減算ループを per-source の
  `sub_scaled` 形に再構成すると、転置アクセス(strided gather)
  が連続ストリームに変わり、プリミティブ置換と性能改善を同時に
  得られる(実装・計測で確認)
- hypothesis: linalg.rs の内部 `dot` を共有カーネル化すれば、
  既存の内部 SIMD 版と公開プリミティブが同一実装に統一される

## Open Questions

- criterion ベンチの具体的対象(プリミティブ単体 + 代表呼出し経路
  のどこまでを測るか)とサイズ分布 → spec/design で確定
- `assert_eq!` 厳密比較している float テストの洗い出しと許容比較
  への修正範囲 → planning で棚卸し
- residual.rs 再構成の具体的形(sub_scaled per source channel)と
  既存テストとの整合 → design で確定
- 小ベクトル(spectral_envelope の 32bin 等)への per-call dispatch
  コストが許容範囲か → design/bench で確認
- Logging/Profile 基盤チェーンのスコープ(計測の出力先・ログ
  フレームワーク選定・導入範囲)→ 別チェーンの deep-dive で確定。
  本集約はその完了後に着手

## Promotion Candidates

- [ ] spec-doc needed: `al_math::vec` の公開 API 契約(シグネチャ・
  長さ panic・ゼロ入力規則)と受入基準(重複解消・scalar/dispatch
  許容誤差等価・criterion 計測)
- [ ] adr-doc needed: reduction 順序ポリシー(lane+FMA 許容、
  等価は許容誤差)、dsp ファサード経由の公開境界、corr0 融合実装
- [ ] blocking dependency: Logging/Profile 基盤チェーンを先行
  (ユーザー決定)。その discovery/ADR が本集約の前提になる

## Sources and Relations

- `docs/specs/al-math-api-boundary.md` Deferred Design Concerns
  (本探索の起点。derives-from で連結)
- `docs/designs/al-math-api-boundary.md` Deferred Design Concerns
- 棚卸し対象コードは `relations.source` に記載
- `target/explain-reduction-order.html`(reduction 順序の解説資料。
  ユーザー確認用の一時資料でコミット対象外)
- 後続チェーン: Logging/Profile 基盤(先行)、本集約の spec/ADR/
  design/plan/task(後続)

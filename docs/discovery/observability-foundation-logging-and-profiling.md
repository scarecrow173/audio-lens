---
id: "DISC-034EdbeKlegRxkqHB283iU"
type: "discovery"
status: "active"
title: "observability foundation: tracing 基盤と stage 計時"
created: "2026-10-11"
updated: "2026-10-13"
owners: []
relations:
  source:
    - "../../crates/al-cli/src/main.rs"
    - "../../crates/al-job/src/runner.rs"
    - "../../crates/al-job/src/manifest.rs"
    - "../../crates/al-locate/examples/bench_dsp.rs"
    - "../../crates/al-locate/examples/bench_locate_prepared.rs"
    - "../../crates/al-locate/examples/bench_ncc.rs"
    - "../../crates/al-math/examples/bench_correlation.rs"
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
  blocks:
    - "./al-math-simd-reduction.md"
  supersedes: []
  superseded-by: []
  related:
    - "./al-math-simd-reduction.md"
    - "./caller-side-optimizations.md"
    - "./mechanical-caller-cleanup.md"
  refines: []
  refined-by: []
  derives-from: []
  derived-by:
    - "../specs/observability-foundation.md"
    - "../adr/observability-dedicated-crate.md"
    - "../adr/logging-sink-format-activation-contract.md"
    - "../adr/stage-timing-manifest-persistence.md"
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# observability foundation: tracing 基盤と stage 計時

## Exploration Goal

`al-math-simd-reduction` の deep-dive 中、「criterion ベンチ導入」が
論点になった際にユーザーが「先に Logging と Profile の方針を決めて
全体に導入してからの方がいい」と順序変更を決定。本探索は全 crate へ
の logging + profiling 基盤導入方針を確定するもので、vec プリミティブ
集約の先行チェーンとして位置づく。

課題フレーミング: AudioLens の observability は構造化 artifacts
(manifest・feature records)による「結果」の記録のみで、「過程」の
観測経路が存在しない。(1) 進行・診断のためのログ経路がない
(production 経路の stderr 出力は `fail()` の error 行のみ)、
(2) ステージ単位の時間配分が計測不能で、全体俯瞰からボトルネックを
追う証拠チャネルがない。最適化施策(#42 等)の優先度付け・効果検証に
使う計測基盤も未整備。

**計測の目的は「job 単位デバッグ」ではなく「システム全体のボトル
ネック調査」であり、job 横断で俯瞰してから詳細にドリルダウンする
使い方を前提とする**(ユーザー明示)。この目的が sink 集約・format・
activation の各決定を方向づけた。

## Key Issues and Assumptions

- 確認済み(ログ基盤皆無): 全 crate の Cargo.toml に `tracing`/`log`
  系依存は 0 件。production 経路の stderr 出力は `al-cli/src/main.rs`
  `fail()` の `error: {message}` 1系統のみ
- 確認済み(stdout の契約強度): cheaper との比較調査で洗い出し。
  al-cli の stdout は `run`→`job_id=`/`job_dir=` key=value 行、
  `resume`→stage 状態行、`inspect`→manifest 全体の JSON、
  `--help`→USAGE。cheaper のような「envelope 1 JSON + byte-parity」
  の hard contract はないが、`inspect` の JSON を tracing 行で
  汚すと機械処理が壊れる実害がある → 「log を stdout に流さない」は
  契約違反ではなく実害回避の規約として必要
- 確認済み(計時注入点): `StageRunner::execute` が全ステージ実行の
  唯一の通過点。ここで囲めば全 stage の計時を注入できる
- 確認済み(構造化永続先): `StageRecord` が manifest に既存し、
  `duration_ms` を追加すれば計時を job 成果物として永続化できる
  (cheaper は相当する構造化 record を持たず log stream に流した。
  AudioLens は「計時=manifest、診断=log」の分離が可能で良い形)
- 確認済み(ベンチ現状): criterion 未導入。examples/ の `Instant`
  手動計測が4本のみ(bench_dsp/bench_ncc/bench_correlation/
  bench_locate_prepared)。profiler 連携・CI ベンチもなし
- 確認済み(手動 arg parse): al-cli は clap 等を使わない手書き引数
  解析のため flag 実装は自前になる
- 前提: secret(credential 等)は扱わない。redaction 対象は
  「raw PCM/巨大データを field に載せない」規約に置き換え
- 前提: pre-release 方針により互換層は設けないが、旧 manifest の
  読込が壊れないよう `#[serde(default)]` は無難に入れる
- 参照実装: `F:\repositry\cheaper` の observability foundation
  (DISC-034DUx01YRjOAf4gTaftO7 / ADR-034DUFA70cQdAyG7Lbuuay /
  ADR-034DUFzrXJxOwfl1qveXWR)で同テーマを確定済み。9 軸の比較は
  `target/report-logging-profile-survey.html` §5.5 に記録

## Alternatives and Comparison

### D1: 全体構成規模

| Option | Pros | Cons | Lean |
| --- | --- | --- | --- |
| 縮小構成(tracing+stderr 既定、init は al-cli、file sink は opt-in) | crate 増なし、最小差分 | sink/format/activation 方針の単一の所有境界がなく肥大時に cli に集中 | no |
| **cheaper 同等構成(専用 `al-observability` crate が sink init/format/routing/activation を所有、dual sink 常時)** | backend 切替・sink 追加を1境界に吸収。横展開で一貫。cheaper で検証済みの形 | crate +1、全 crate への tracing 依存追加と init 配線の初期コスト | **yes(採用)** |
| 中間(al-cli init + file sink 既定有効) | crate は増えない | init 方針が cli に分散する兆候 | no |

### D2: file sink の置き場

| Option | Pros | Cons | Lean |
| --- | --- | --- | --- |
| **`<base_dir>/logs/`(output root 直下に集約)** | job 横断の全 run の log が一箇所に集約され「全体俯瞰→詳細」の使い方に合致。cheaper の project 集約と同型 | 集約 dir が肥大するため retention 管理が要る | **yes(採用)** |
| `<job_dir>/logs/` | job 単位で証跡が完結 | job 横断の俯瞰ができず目的と矛盾 | no |
| 両方(stderr + job file + global file) | — | sink 3系統で過剰 | no |

### D3: sink ごとの format

| Option | Pros | Cons | Lean |
| --- | --- | --- | --- |
| **両方切替 & 同時併用可能(各 sink が独立に text/JSON を設定可)** | 俯瞰の機械処理(file=JSON)と実行中の人間向け確認(stderr=text)を同時に成立させられる。ユーザー明示 | sink ごとの format 配線 | **yes(採用)** |
| 全 sink 一律切替(cheaper 型) | 実装単純 | 用途別最適化なし | no |
| 固定(file=JSON, stderr=text、切替機構なし) | 最小 | 柔軟性なし | no |

### D4: 計装範囲(初回)

| Option | Pros | Cons | Lean |
| --- | --- | --- | --- |
| **cheaper 同等の網羅(全 crate の主要境界 + stage 内部の m・family 等)** | 「盲点は未計測で選定根拠がない」。stage 内がブラックボックスのまま残らず俯瞰→ドリルダウンが初回から成立 | instrumentation 差分が広い。span 命名・field 規約の統一が要る | **yes(採用)** |
| stage 境界のみ(runner+cli) | 差分最小 | stage 内がブラックボックスのまま | no |
| stage 境界 + 代表 internal のみ | 折衷 | 「代表」の選定根拠が未計測 | no |

### D5: activation と既定状態

| Option | Pros | Cons | Lean |
| --- | --- | --- | --- |
| **既定 OFF + 有効化(`AL_LOG` env + `--log` flag、flag 優先)** | cheaper と同型。OFF 時 ~zero overhead。計時自体は manifest に常時記録されるため OFF でも俯瞰データは残る | activation 解釈の単一所有が要る(al-observability) | **yes(採用)** |
| 既定 ON(info) | 俯瞰データが自動蓄積 | overhead が常時コスト、log 量増 | no |
| 既定 file のみ ON | 折衷 | stderr/file で既定が非対称になり複雑 | no |

### D6: フィルタ粒度

| Option | Pros | Cons | Lean |
| --- | --- | --- | --- |
| **EnvFilter 型 + 実行時引数でも指定可能(`AL_LOG=al_locate=debug,info` 形式、flag も同じ directive を受ける)** | 全体=info・特定 crate=debug のドリルダウンが1経路で可能。tracing-subscriber の EnvFilter がそのまま使える。ユーザー明示 | directive 解釈を env/flag 両経路で統一する必要 | **yes(採用)** |
| 単純 level のみ | 実装最小 | 特定箇所だけ深く見るには全体の log 量を上げる必要 | no |
| level + 別 --log-target | 表面分離 | 経路が2系統 | no |

### D7: L2 計時の出力先

| Option | Pros | Cons | Lean |
| --- | --- | --- | --- |
| **manifest 永続化 + stderr 両方(`StageRecord.duration_ms` + runner が stage 完了を tracing event でも emit)** | 構造化(manifest)=永続・機械処理用、log=実行中の人間向け。両立する | record 拡張と event 定義の2箇所 | **yes(採用)** |
| manifest のみ | 最小 | 実行中のリアルタイム確認ができない | no |
| stderr/log のみ | — | 計時が文字列パース頼みになり俯瞰分析が困難 | no |

### D8: L1/L3 計測基盤

| Option | Pros | Cons | Lean |
| --- | --- | --- | --- |
| **L1=criterion(al-math dev-dep)、L3=samply 随時(計装なし・文書化のみ)** | criterion は統計的計測+baseline 比較が標準。samply は Windows/macOS/Linux 対応の sampling profiler で計装不要 | criterion は依存追加(≥7d 経過版を選定) | **yes(採用)** |
| L1=divan / L3=flamegraph,tracy | — | divan は baseline 機能で劣り、iai は Windows 非対応。tracy は常時計装が要る | no |

### D9: 既存 examples ベンチ・CI

| Option | Pros | Cons | Lean |
| --- | --- | --- | --- |
| **当面維持(粗い E2E として)、CI ベンチ/回帰は見送り** | CI 基盤自体がないため。必要時に別検討 | criterion と役割が重複する腕が残る | **yes(採用)** |
| criterion へ移行 / CI 導入 | 統一 | 現時点で過剰(YAGNI) | no |

### D10: crate topology の詳細

| Option | Pros | Cons | Lean |
| --- | --- | --- | --- |
| **leaf crate は `tracing` 直接依存で emit のみ、`al-observability` は sink init/format/routing/activation のみ所有、init は al-cli が呼ぶ** | cheaper T-B と同型。leaf は subscriber の存在を知らなくてよく、facade wrap は標準 API を隠すだけで実益なし(採用しなかった T-C と同じ理由) | 全 crate への tracing 依存追加 | **yes(採用)** |
| al-observability が span macro まで wrap する facade | 記法一元化 | 標準 API を隠し実益なし、cheaper で棄却済み | no |

### D11: redaction

| Option | Pros | Cons | Lean |
| --- | --- | --- | --- |
| **metadata-only 規約(len/hash/id)、raw PCM/巨大 field は不記録** | AudioLens に credential はないが raw データを log に載せない予防規約として cheaper SEC-B と同型で置く価値あり | 規約の文書化とレビュー観点 | **yes(採用)** |
| 規約なし | — | PCM 等の巨大 field が混入しうる | no |

## Tentative Conclusions and Hypotheses

以下は全て deep-dive 対話でユーザー確認済み:

- confirmed: 専用 `al-observability` crate を新設し sink init/format/
  routing/activation 解釈を所有。他 crate は `tracing` に直接依存して
  emit のみ(leaf crate は `al-observability` に依存しない)。依存
  方向は `al-cli → al-observability → tracing-subscriber` と
  `leaf → tracing` の2系統。init は al-cli が呼ぶ
- confirmed: sink は stderr + file(`<base_dir>/logs/`)の dual routing。
  format は各 sink が独立に text/JSON 切替可能で同時併用も可
  (file 既定 JSON-lines、stderr 既定 text)
- confirmed: 計装範囲は網羅的 — cli dispatch、runner、stage
  境界、stage 内部(locate method、analysis family、media ingest、
  viz、dataset)。対象外: al-core(純粋型)、al-math(ホットカーネル、
  性能計測は criterion の責務)、al-testkit(テストヘルパ)。
  span 命名・field 規約は spec で統一
- confirmed: 既定 OFF、`AL_LOG`(EnvFilter directive)+ `--log` flag
  (flag 優先)で有効化。format 指定も env var + flag
  (`AL_LOG_FORMAT` / `--log-format` 系)
- confirmed: `StageRecord.duration_ms` を manifest に常時記録
  (log 有効性と無関係)+ runner が stage 完了を tracing event でも
  emit。`#[serde(default)]` で旧 manifest 読込を維持
- confirmed: L1=criterion(al-math dev-dep、≥7d 経過版)、
  L3=samply 随時(計装なし・文書化のみ)。既存 examples ベンチは
  当面維持(粗い E2E)。CI ベンチは見送り
- confirmed: metadata-only 規約(len/hash/id)。raw PCM/巨大 field は
  log に載せない
- confirmed: 本チェーン完了後に `al-math-simd-reduction` の vec
  プリミティブ集約へ進む(criterion が先行導入される)
- hypothesis: stderr の tracing sink と既存 `fail()` error 行は
  行単位で interleave する共存規約になる(cheaper の確定ルールと
  同型)。al-cli に `--quiet` 相当は現状ないため新設しない

## Open Questions

- span 命名・field 規約(stage 名・method 名・len/hash の canonical
  naming)と JSON format の field schema → spec で確定
- rolling の既定値(daily vs per-startup、保持世代数、file 命名)
  → spec で確定
- `al-observability` の crate 名・配置の最終形 → spec/ADR で確定
- `StageRecord.duration_ms` の resume 時の意味付け(複数 run での
  合算か上書きか)→ spec で確定
- viz package への duration 同梱可否 → spec で確定
- instrumentation off 時の overhead を benchmark で許容範囲内と
  できるか → 実装時に計測

## Promotion Candidates

- [ ] spec-doc needed: observability foundation の実施スコープ —
  crate topology、sink 構成・format 規約、activation 経路、
  計装対象一覧、span/field 規則、acceptance criteria(log 無効時
  regression pass・両 sink への stage timing event・metadata-only
  regression test・off 時 overhead 計測)
- [ ] adr-doc needed: (1) `al-observability` crate topology(専用
  crate 所有 vs cli init vs facade)、(2) sink/format/activation
  contract(dual sink・per-sink format・EnvFilter・既定 OFF)、
  (3) stage timing の manifest 永続化方針
- [ ] blocking: 本チェーンが `al-math-simd-reduction` の先行条件

## Sources and Relations

- 比較調査レポート: `target/report-logging-profile-survey.html`
  (リポジトリ全体調査 + cheaper との9軸比較 §5.5 + 決定論点表 §7。
  ユーザー確認用の一時資料でコミット対象外)
- 参照実装(外部リポジトリ): `F:\repositry\cheaper` の
  `docs/discovery/observability-foundation-logging-and-profiling.md`、
  `docs/adr/logging-output-and-activation-contract.md`、
  `docs/adr/consolidate-observability-behind-a-dedicated-crate.md`
- 後続チェーン: `docs/discovery/al-math-simd-reduction.md`
  (本基盤の criterion 導入を前提とする。blocks で連結)
- 計時注入点・manifest: `crates/al-job/src/runner.rs`、
  `crates/al-job/src/manifest.rs`
- 決定軸の確定経緯: 本セッションの deep-dive(D1–D11 全て確定)

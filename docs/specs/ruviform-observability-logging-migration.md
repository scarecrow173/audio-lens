---
id: "SPEC-PJaIgGwP5EbiX0UvyC7aZ9"
type: "spec"
status: "approved"
title: "al-observability の ruviform-observability::logging 委譲移行 (AL-OBS-01)"
created: "2026-11-04"
updated: "2026-11-04"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/audio-lens/issues/138"
    - "https://github.com/scarecrow173/ruviform/issues/276"
    - "https://github.com/scarecrow173/ruviform/issues/277"
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
    - "observability-foundation.md"
    - "ruviform-foundation-migration.md"
    - "ruviform-lockfile-source-of-truth.md"
    - "../adr/logging-sink-format-activation-contract.md"
    - "../adr/observability-dedicated-crate.md"
  refines: []
  refined-by: []
  derives-from: []
  derived-by: []
  verifies: []
  verified-by: []
  references:
    - "https://github.com/scarecrow173/ruviform/issues/334"
    - "https://github.com/scarecrow173/ruviform/issues/335"
    - "https://github.com/scarecrow173/ruviform/issues/289"
  defers: []
  deferred-by: []
---

# al-observability の ruviform-observability::logging 委譲移行 (AL-OBS-01)

## What Should Be Built

`crates/al-observability` が所有する logging の共通
Subscriber/Sink 実装(subscriber 構築・sink 配線・writer)を、
Ruviform #276 で導入された `ruviform-observability` の
`logging` feature(`build_logging` / `build_logging_layers`)へ
委譲する。AudioLens 固有の CLI/env 解釈・sink 語彙・format
既定値・file 集約先は本リポジトリに残し、`al-observability`
は薄い製品 Config Adapter として残る。
Issue [#138](https://github.com/scarecrow173/audio-lens/issues/138)
の Acceptance Criteria が正本である。

## Why Now

- Ruviform #276(OBS-01 汎用 logging adapter)・#277(OBS-02
  Source 登録契約)が main にマージされ、consumer 側の委譲
  対象が利用可能になった。
- `docs/designs/ruviform-migration-responsibility-map.md` で
  「`al-observability` の Ruviform 委譲」は別判定として延期
  されていた項目であり、#138 がその判定・実施を担う。
- AudioLens 側は独立 CI(#142)・Cargo.lock 正本(#140)・
  export portable(#141)が整備済みで、最新 Ruviform main
  への追従経路が確立している。

## Users and Value

- AudioLens 保守担当: sink 構築・bounded writer・health
  計測の正本が Ruviform 側に集約され、二重実装の
  drift が消える。
- Ruviform 保守担当: consumer の実契約(AL_LOG 相当の
  有効化ポリシー、`al.YYYY-MM-DD.log` 命名、span close
  event)が委譲移行を通じて検証され、adapter gap が
  Issue として還流する。

## Scope

### In Scope

- `crates/al-observability` の `init` 実装を
  `ruviform-observability::logging` の
  `build_logging_layers` へ委譲し、host(al-cli)のみが
  Dispatch/global subscriber を登録して guard を保持する
  形にする。global 重複 init はしない。
- AudioLens 固有の製品契約を薄い adapter として保持:
  `resolve_activation`(`AL_LOG`/`AL_LOG_FORMAT`/
  `--log`/`--log-format` の優先順位・sink/format 閉集合
  語彙・fail-fast エラー語彙)、file 集約先
  `<base_dir>/logs`・日次 `al.YYYY-MM-DD.log`・最大 7 世代
  の命名契約。
- 現行挙動を固定する golden test の拡充(Issue 実装順序
  (a)): CLI/env/format、stdout、rotation/retention、error
  exit code、file 初期化失敗 degrade。
- 移行後の検証(Issue 実装順序 (d)): Logging ON/OFF での
  build/test、golden parity、解析結果(f64 精度・Job
  生成物)の非回帰、samply 手順の保持確認。
- Ruviform adapter gap の還流: `filename_suffix` 非公開
  (Ruviform #334)・span lifecycle event 非設定
  (Ruviform #335)を Issue 起票済み。解消までの暫定
  回避策を理由・解消条件付きで文書化する。

### Out of Scope

- Collector/Metrics 本格導入: Ruviform OBS-07
  Collector(#289)は未マージ。AudioLens 側に登録すべき
  Counter/Source 実体が現状存在せず、Source→Snapshot
  接続は利用可能な範囲がないため後続 Issue に委ねる。
- `al-job` の `StageRecord.duration_ms`/Progress/Manifest/
  Provenance の Metrics 化。これらは業務上の正本であり
  観測 Metrics と独立に維持する(Issue 作業範囲の明示)。
- GUI(al-gui)への logging 配線・RT 文脈の telemetry。
- Ruviform 側の実装修正(本リポジトリ外の変更は行わない。
  gap は #334/#335 で追跡する)。

## Requirements

### 維持する互換契約(Issue 変更禁止仕様・すべて必須)

1. Logging は既定 OFF。`AL_LOG`・`--log` を Directive として
   使用し、flag が env より優先、`--log-format` 単独では
   Logging を有効化しない。
2. stderr Text + file JSONL の sink 独立制御、stderr JSON
   許可。stdout へ log を混ぜない。
3. `al inspect` は stderr-only。readonly 入力ディレクトリ等へ
   log file を作らず、stdout 機械可読 JSON と終了コードを
   完全互換とする。
4. `run`/`resume` の file sink は `<base_dir>/logs`、日次
   `al.YYYY-MM-DD.log`、最大 7 世代。
5. file 初期化失敗は stderr へ明示 degrade し、解析 Job を
   不必要に失敗させない。InvalidDirective と設定異常の明示
   エラーは従前どおり。
6. ID/count/hash/stage 等の metadata-only を維持。Raw
   audio/PCM/Source 内容/Prompt/Credential を自動出力しない。
   f64 解析精度・Job 生成物の整合を回帰させない。

### 委譲契約

- `al-observability` は `ruviform-observability::logging`
  (feature `logging`)に sink 構築・bounded writer・guard
  drain の一般実装を委譲し、製品固有の有効化ポリシーと
  file 命名/集約先のみを保持する。
- leaf crate は従来どおり `tracing` 標準 API で emit のみを
  行い、`al-observability` に依存しない。
- Host は `al-cli` のみ。Dispatch 登録・guard 保持は
  host(`cmd_*` スコープ)が担い、global 重複 init をしない。
- Collector を Logging の必須前提にしない。

## Acceptance Criteria

- [ ] `al-observability` の一般実装が Ruviform へ委譲され、
      AudioLens 固有設定のみ残る。
- [ ] CLI の既定 OFF/priority/stderr/file/jsonl/inspect
      stdout/daily 7 世代/error/fallback parity が golden
      test で PASS する。
- [ ] Ruviform/AudioLens の標準 tracing が一つの Host
      Subscriber へ届き、global 重複 init がない。
- [ ] file sink init 失敗時に stderr へ warn degrade し、
      Job が成功する。
- [ ] `mise run verify-ci`(fmt→check-ci→lint-ci→test-ci)
      が green で、`check-ruviform-lock` が単一 resolved
      SHA を検証する。
- [ ] Logging ON/OFF で解析結果(manifest・dataset・export
      成果物・f64 値)が同一であることを確認する。
- [ ] Ruviform adapter gap(#334/#335)に対する暫定回避策が
      理由・解消条件付きで文書化されている。

## Constraints

- Cargo.lock は最新 Ruviform main を解決して使う(SHA 固定
  回避禁止)。`check-ruviform-lock`/`check-ruviform-rev` が
  現行 SHA を照合する。
- Counter 差分や Severity の意味変換を AudioLens で再実装
  しない。Collector/Source は Ruviform 正本を利用する。
- samply によるプロファイリング手順(`docs/guides`)を保持
  する。

## Non-Goals

- Ruviform adapter 自体の機能拡張(別リポジトリ。#334/#335
  で追跡)。
- tracing 以外の telemetry 基盤の導入。

採用設計は [design](../designs/ruviform-observability-logging-migration.md)
を参照する。

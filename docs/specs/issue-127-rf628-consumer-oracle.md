---
id: "SPEC-034Y17EML4OKyeGCYsuFS8"
type: "spec"
status: "proposed"
title: "Issue #127 RF#628 consumer oracle: file-based f64 fidelity・長尺非回帰・gap/Validity"
created: "2026-11-15"
updated: "2026-11-15"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/audio-lens/issues/127"
    - "https://github.com/scarecrow173/ruviform/issues/628"
  changes:
    added:
      - type: "file"
        path: "Cargo.toml"
      - type: "file"
        path: "Cargo.lock"
      - type: "file"
        path: "crates/al-analysis/Cargo.toml"
      - type: "file"
        path: "crates/al-analysis/tests/rf628_oracle.rs"
      - type: "file"
        path: "docs/specs/issue-127-rf628-consumer-oracle.md"
      - type: "file"
        path: "docs/designs/issue-127-rf628-consumer-oracle.md"
      - type: "file"
        path: "docs/plans/issue-127-rf628-consumer-oracle.md"
      - type: "file"
        path: "docs/tasks/issue-127-rf628-consumer-oracle.md"
      - type: "file"
        path: "docs/test-specs/issue-127-rf628-consumer-oracle.md"
      - type: "file"
        path: "docs/impl/ir/issue-127-rf628-consumer-oracle.md"
      - type: "file"
        path: "docs/specs/ruviform-abb9f372-rev-watch.md"
      - type: "file"
        path: "docs/designs/ruviform-abb9f372-rev-watch.md"
      - type: "file"
        path: "docs/tasks/ruviform-abb9f372-rev-watch.md"
      - type: "file"
        path: "docs/test-specs/ruviform-abb9f372-rev-watch.md"
      - type: "file"
        path: "docs/impl/ir/ruviform-abb9f372-rev-watch.md"
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated:
      - type: "file"
        path: "ui/vendor/ruviform"
      - type: "file"
        path: "docs/specs/README.md"
      - type: "file"
        path: "docs/designs/README.md"
      - type: "file"
        path: "docs/plans/README.md"
      - type: "file"
        path: "docs/tasks/README.md"
      - type: "file"
        path: "docs/test-specs/README.md"
      - type: "file"
        path: "docs/impl/ir/README.md"
      - type: "file"
        path: "docs/impl/exp/README.md"
  implements: []
  implemented-by: []
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/specs/ruviform-ac10a2f7-rev-watch.md"
    - "docs/specs/ruviform-c6e4f334-rev-watch.md"
  refines: []
  refined-by: []
  derives-from: []
  derived-by: []
  verifies: []
  verified-by:
    - "../test-specs/issue-127-rf628-consumer-oracle.md"
  references: []
  defers: []
  deferred-by: []
---

# Issue #127 RF#628 consumer oracle: file-based f64 fidelity・長尺非回帰・gap/Validity

## What Should Be Built

`scarecrow173/ruviform#628` が提供する deterministic PCM fixture
（`PcmFixture`/`PcmScenario`/`PcmSegment`/`PcmPattern`）と独立 oracle
（`oracle::pcm_reference_block`/`PcmCompare`/`pcm_assert_eq`）を
AudioLens 側の **file-based 解析 oracle** として消費する統合テストを
`crates/al-analysis` に新設する。Ruviform fixture が生成する f32 PCM を
f64 へ exact widen し、`al_testkit::media::pcm_f64le` で
`decoded_pcm` artifact 化、`Manifest`/`ArtifactRegistry`/`StageContext`
経由で実 `CheapAnalysisStage`/`LocateStage`/`ReferenceAnalysisStage` を
駆動し、計測 feature を **被験実装と独立な 2 系統の期待値**で照合する:

1. `oracle::pcm_reference_block` が fixture spec から独立に再計算した
   f32 ブロックを f64 widen した統計量（dc/rms/peak 等）。
2. AL 解析的参照（sine `freq_hz` の既知周波数・`Constant{value}` の
   DC/level・`Noise{seed}` の決定的統計・scenario segment 範囲から
   独立導出する validity map）。

対象脚は 3 本:

- **AL-1 f64 fidelity oracle**: fixture PCM → f64 widen → file artifact
  → stage 計測が、oracle 参照・解析的参照の期待値と許容差内で一致する
  こと。f32→f64 widen の exact 性を利用し、oracle 側 f32 期待値を f64
  へ widen して比較する。`Constant`/`Sine`/`Noise`/`Square`/ステレオ
  `Noise` 等の pattern で level/spectral/digital feature を照合する。
- **AL-2 long-run 非回帰**: 複数 `PcmSegment` からなる大フレーム
  `PcmScenario`（48kHz・数十秒〜数分規模、規模と根拠は test-spec に
  記録）を dataset 形状 artifact として流し、`LocateStage` +
  `ReferenceAnalysisStage` の参照再計算が既存 golden 許容帯域内で
  非回帰であること、長大 tensor/軌跡が破綻しないことを確認する。
- **AL-3 gap/missing-frames → NaN/Validity/Provenance**: 非連続
  `PcmScenario`（segment 間に明示 gap）を gap フレーム = `f64::NAN`
  として decoded_pcm に展開し、segment 範囲から独立導出した期待
  validity map（level 軌跡の NaN 窓位置・finite 窓数・support_count・
  summary の valid_fraction 相当・dc_offset の null 化・peak 系の
  有限性・provenance stamp）と AL の出力を照合する。

併せて、pin bump（`ac10a2f7` → `abb9f372d3ee95ee824b3c4f7dc68e8083ce2774`）
と `ruviform-audio-test` dev-dependency 追加を行い、lockfile-primary
契約（manifest `rev` なし・`Cargo.lock` 単一 resolved SHA）を維持する。
`abb9f372` の採用/保留/rollback 判断は rev-watch 文書セット
（`ruviform-abb9f372-rev-watch`）に記録する。

## Why Now

Issue #127 は Ruviform 更新を AudioLens が Synvelis と対等な独立
Consumer として継続評価・導入するための常設追跡 Issue であり、受入条件
には「RF #628 の Ruviform 固定 PCM/Clock/Signal Fixture を利用し、
AudioLens 独自の f64/長尺/PTS/gap/Validity/CLI/解析 GUI を独立 Oracle
で判定する」ことが求められている。本 slice はそのうち **file-based
解析の f64 fidelity・長尺非回帰・gap/Validity の 3 脚**を担当する。
`ruviform-audio-test` は `publish = false` の test-only crate であり、
dev-dependency としてのみ消費可能（既存前例: `ruviform-ui-test`）。
fixture/oracle は決定的（libm 非依存の整数・f64 算術、Sine/Sweep のみ
libm `f64::sin` の oracle で AbsEpsilon 比較）であり、CI 再現性を持つ。

## Users and Value

- AudioLens 開発者: RF#628 fixture/oracle を介した独立受入証拠により、
  解析精度（f64 fidelity）・長尺挙動・gap/Validity の回帰を機械的に
  検出できる。Ruviform 側の self-test 結果には依存しない。
- Coordinator / Reviewer: Issue #127 の受入条件に対応する検証結果
  （実測と NotRun/Deferred/NotApplicable の区別）を PR/Issue から
  追跡できる。
- 将来の consumer-oracle 拡張（PTS/CLI/GUI 脚）: 本 slice が確立する
  fixture→artifact→stage の経路と独立期待値の導出パターンを再利用
  できる。

## Scope

### In Scope

- `crates/al-analysis` への `ruviform-audio-test` dev-dependency 追加
  （workspace 依存経由、`rev` なし）と、それに伴う推移的依存を含む
  ruviform pin の `abb9f372d3ee95ee824b3c4f7dc68e8083ce2774` への
  lockfile-primary 一括 bump。
- `crates/al-analysis/tests/` への RF#628 consumer-oracle テスト新設
  （AL-1/AL-2/AL-3 の 3 脚。単一ファイルまたは脚別分割）。
- fixture → `pcm_f64le` → `Manifest`/`ArtifactRegistry` →
  `StageContext` → 実 stage の file-based 経路での検証。
- `oracle::pcm_reference_block`/`PcmCompare`/`pcm_assert_eq` による
  fixture 出力と独立参照の一致確認（fixture 検証として）と、
  参照ブロック統計量による AL 計測の期待値導出。
- 非連続 `PcmScenario` の gap を `f64::NAN` として decoded_pcm に
  展開し、Validity/NaN/Provenance の扱いを segment 範囲由来の
  期待 validity map と照合するテスト。
- rev-watch 文書セット（`ruviform-abb9f372-rev-watch`、specs/
  designs/tasks/test-specs/impl/ir の 5 点）による採用/保留/rollback
  判断の記録。
- 本 slice の spec/design/plan/task/test-spec/impl 記録と index 再生成。

### Out of Scope

- AL-4 deterministic clock drift: AL には live clock consumer がなく
  file-based 解析への適合が弱いため **Deferred**（`clock::VirtualClock*`
  API は文書レベルの参照まで）。
- AL-5 device-pipeline legs（`ruviform_audio_test::{backend, pipeline,
  midi}`、`ScriptedBackend`）: AL は device boundary を消費しないため
  **NotApplicable**（`Unsupported` ではない）。
- 製品コード（`crates/` の `al-analysis` 以外、`al-locate`、`al-testkit`
  の production `src/` 等）の機能変更。テストで既存 helper を import
  するのは可。
- PTS/gap の ingest・decode 側（`decode_input`/`read_normalized_record`
  の non-finite 拒否を含む locate 正規化経路）の変更・拡張。本 slice は
  decoded_pcm artifact 境界での解析側 Validity を対象とする。
- CLI/GUI/Export・3OS CI・cross-consumer・実機 GUI-L2/EXT-L3 の受入
  （Issue #127 の他 slice・CI の責務）。
- Issue #127 の Close、ruviform・synvelis リポジトリの変更。

## Requirements

- manifest の恒久固定 rev へ戻さず、lockfile の `source` fragment が
  単一 resolved SHA の正本であり続けること（lockfile-primary 契約）。
- `ruviform-audio-test` は dev-dependency のみで製品コード・配布物に
  入らないこと（test-only 境界は ruviform 側 `lib.rs` が保証）。
- `cargo fetch --locked`・`mise run check-ruviform-lock`・
  `mise run check-ruviform-rev` が単一 SHA 解決を fail-closed に検査
  すること。
- 期待値は被験実装と独立に導出すること: (a) `oracle::pcm_reference_block`
  の独立再実装、(b) AL 解析的参照（既知周波数・定数値・seed 統計・
  segment 範囲）。ruviform 側 self-test 結果を AL の Pass に転用しない。
- fixture の f32→f64 widen は exact（全 f32 は f64 で表現可能）である
  ことを利用し、oracle 側 f32 期待値を f64 へ widen して比較する。
- AL-2 の長尺規模はテスト実行時間を合理的範囲に収め、採用規模と根拠を
  test-spec に記録すること。
- AL-3 の期待 validity map は segment の `[frame_start,
  frame_start+frame_len)` 範囲と level window/hop から独立に組み立て、
  NaN・欠測・Validity・Provenance を照合すること。
- 検証不能事項・失敗ケース・許容差を結果へ明記し、未実行 gate を
  Pass に繰り上げないこと。
- 採用/保留判断と rollback 情報（`ac10a2f7` への復帰手順）を記録する
  こと。

## Acceptance Criteria

- [ ] `ruviform-audio-test` が `crates/al-analysis` の
  `[dev-dependencies]` に workspace 経由で追加され、root
  `Cargo.toml` の `[workspace.dependencies]` に git dep（`rev` なし）
  が追加されている。
- [ ] `Cargo.lock` の全 ruviform package（dev-dep の推移的依存で
  新規に入るものを含む）が単一 SHA
  `abb9f372d3ee95ee824b3c4f7dc68e8083ce2774` に解決され、
  `check-ruviform-lock`/`check-ruviform-rev`/`sync-ui-deps` の
  `REV` 一致が Pass する。lockfile の ruviform package 数変動と
  推移的依存差分が文書に記録されている。
- [ ] AL-1: `PcmFixture`/`PcmScenario` の f32 PCM を f64 widen →
  `pcm_f64le` artifact → `Manifest`/`ArtifactRegistry`/`StageContext`
  経由で実 stage を駆動し、計測 feature が oracle 参照統計量と
  AL 解析的参照の期待値に許容差内で一致するテストが存在し Pass する。
- [ ] AL-2: 複数 `PcmSegment` の大フレーム `PcmScenario` で dataset
  形状 artifact を生成し、参照再計算が既存 golden 許容帯域で
  非回帰であること、および長尺 tensor/軌跡の健全性を確認する
  テストが存在し Pass する。
- [ ] AL-3: 非連続 `PcmScenario` の gap を NaN として decoded_pcm に
  展開し、segment 範囲由来の期待 validity map と NaN/Validity/
  Provenance の扱いが一致するテストが存在し Pass する。
- [ ] AL-4 は Deferred、AL-5 は NotApplicable として test-spec・
  PR 本文に根拠つきで記録されている。
- [ ] rev-watch 文書セット（5 点）に `abb9f372` の採用判断・
  `ac10a2f7..abb9f372` 差分概要・rollback 手順が記録されている。
- [ ] 正規 mise 検証 lane が対象 revision で実行・記録されている
  （fmt/check/lint/test/lock/rev/headless/prek）。
- [ ] PR 本文に `Refs #127`（`Closes` 禁止）、resolved SHA、
  AL-1〜AL-5 の Requirement×Result 表、未検証項目が記載されている。

## Deferred Design Concerns

- AL-4 deterministic clock drift（`VirtualClock`/`VirtualClockConfig`/
  `JitterSpec`/`VirtualClockDriver`）| AL は file-based 解析であり
  live clock consumer を持たない。clock API の適合対象が存在しない
  ため受入判定が弱い | live clock consumer（device pipeline 等）の
  採用 slice 起票時 | clock 契約の見落とし | test-spec/design に
  Deferred として記録
- AL-5 device-pipeline legs（`backend::*`/`pipeline::*`/`midi::*`/
  `ScriptedBackend`）| AL は device boundary を消費しない file-based
  Consumer であり、これら module の適合判定対象が存在しない |
  device-pipeline consumer の採用 slice 起票時 | device 境界の
  回帰見落とし | test-spec/design に NotApplicable として記録
- PTS/gap の ingest 側扱い（media timestamp・decode 時の frame 欠落
  検出）| 本 slice は decoded_pcm artifact 境界の解析側 Validity を
  対象とし、ingest/decode の PTS 追跡は別境界 | PTS consumer slice
  起票時 | ingest 側 gap の未検出 | Issue #127 の他 slice・CI の
  責務として記録
- `noise.tonal_peak_hz` 等の spectral 系 feature が gap 混在入力で
  `argmax` の NaN 規則（total_cmp で NaN が最大）により縮退値を
  取り得る点 | 既存 AL 実装の意味論であり本 slice の変更対象外。
  gap 混在時の spectral 期待値は「有限窓のみの統計」として扱う |
  spectral feature の gap 対応改善 Issue 起票時 | gap 混在入力の
  tonal 判定が誤値を返す | test-spec に観測事実として記録し、
  PR 本文の発見事項に記載

## Sources and Relations

- Issue #127（常設追跡 Issue、本 slice の受入条件の出典）
- `scarecrow173/ruviform#628`（deterministic test runtime/fixture
  — `PcmFixture`/`PcmScenario`/`oracle` の提供元）
- 前段 rev-watch: `docs/specs/ruviform-ac10a2f7-rev-watch.md`（直前
  pin `ac10a2f7` の追従評価・本 slice の bump 元）
- `docs/specs/validation-strategy.md`（既存 golden 行列・許容帯域の
  出典）
- `POLICIES.md`（OS-L1 native build/link・独立 Consumer oracle・
  honest verdicts の規範）

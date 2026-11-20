---
id: "TSPEC-0A257frOzEnBeNchSLicE1xY"
type: "test-spec"
status: "draft"
title: "frozen benchmark harness 検証仕様(Issue #257 slice 1)"
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
  derives-from:
    - "baseline/contracts.json"
  derived-by: []
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# frozen benchmark harness 検証仕様(Issue #257 slice 1)

## Purpose

Issue #257 の frozen benchmark harness(`crates/al-service/benches/
frozen_pipeline.rs`、standalone `harness=false`)が「本番 2 経路の
性能を、機器差に頑健な比で繰り返し検証し、回帰を fail-closed で
検出する」ことを担保する検証面を定義する。Ruviform Issue #807
(`dsp_frozen.rs`)の契約語彙・mode 構成・evidence 形を踏襲し、
AudioLens の pipeline 固有部(JobService E2E + fingerprint)を
`baseline/contracts.json` の正規化・3 分類先例に寄せて追加する。

## Feature

`mise run bench-frozen` が、pinned baseline と照合する manual /
opt-in lane として、kernel arm(本番 locate method vs 凍結 naive
参照の in-process A/B 比)と pipeline arm(JobService::run E2E の
同一セッション参照 kernel 正規化比 + analysis payload
fingerprint 照合)を測定・判定できる。

## Rules

- Rule: PRIMARY metric は絶対時間ではなく比。kernel cell は
  `median(A)/median(B)`(凍結 naive NCC は本ファイル内で不変の
  分母)、pipeline cell は `wall/ref_kernel_median` + locate stage
  wall 正規化比(同一セッション内で機器差を相殺する)。絶対時間は
  informational として記録する
- Rule: 反復は各 arm ≥5 回、cell 内で A/B 交互(A→B→A→B…)に
  計測し、median と min/max range を報告する。既定 reps=7
- Rule: kernel error 計数は「naive 参照・本番 method 双方の既知埋込
  offset との lag 不一致(±64 sample)」+「本番 `validity !=
  valid` reject」+「NCC 系(full_ncc)の top score と naive peak の
  乖離(>1e-3)」とする。whitened/envelope 系は score 尺度が違う
  ため peak 比較対象外
- Rule: pipeline error 計数は「stage 失敗(`run` Err または
  `stage.status=failed`・fingerprint 生成不能)」+「accepted
  capture offset と埋込 offset の不一致」+「rep 間 fingerprint
  差分(A/A 非決定性)」+「pinned fingerprint との差分」とする
- Rule: work 計数は timed region で実作業が起きた証拠 — kernel は
  naive MAC 数 + 探索域 lag 数、pipeline は manifest artifact 数
  + 出力 byte 数とする
- Rule: fingerprint は job dir 配下の `*.json`(`array.json`/
  `schema.json`/`index.json`/`video_thumbs.json`/`export/`/
  `analysis/visualization/` を除く)を正規化して canonical
  直列化→sha256 する。正規化は ID(`JOB/ART/IN/ALN-<22>`)の
  canonical 置換・record-only key(`duration_ms`/`error`/
  `*_at`/`timestamp`/`command_line`/`working_dir`)除去・絶対
  パス placeholder 化・f64 の `rel=1e-9, abs=1e-12` grid 量子化
  — `baseline/contracts.json` の recordOnlyFields・
  defaultTolerance・goldenJsonSkipGlobs と同じ語彙
- Rule: gate は fail-closed。pinned baseline が読めない・昇格
  (`approval`非 null)していない・schema 不一致、cell の閾値
  逸脱、error>0 のいずれでも fail。pin に無い cell は
  `unsupported`(run を fail させない)
- Rule: mode 契約 — 引数なし=実走(gate 適用、JSONL stdout)、
  `--test`=1 反復 smoke(閾値 gate は report_only、baseline
  unreadable は limiter note に落として継続 — smoke では
  fail-closed しない。correctness 由来の fail のみ exit 1)、
  `--selftest`=回帰注入(embed 欠落 fixture + kernel B 側直列反復
  + pipeline e2e wall 床上げ)で閾値逸脱と correctness 双方の
  gate fail を実証、
  `--write-baseline <path>`=candidate 出力(`"approval": null`、
  gate 不適用)、`--baseline <path>`=pin 参照先変更、positional
  引数は cell filter(substring、0 cell 一致は gate `not_run`・
  exit 0)
- Rule: baseline 昇格は 2 段階。`--write-baseline` は candidate
  を書くだけで pin を上書きしない。人間が pin path へ複写し
  `approval` を記入して初めて有効
- Rule: repin 事由は `measured_env` に記録する — 機器
  (CPU/core)、toolchain(rustc)、al* コード SHA、ruviform
  依存 SHA。fixture 定数・正規化契約・閾値ルールの変更も repin
  事由
- Rule: lane は manual/opt-in。CI へ組み込まない(本 repo の CI
  bench lane は存在しない)。job 出力は temp dir のみに書く

## Examples

- `mise run bench-frozen` が `kernel/full_ncc` 等 4 cell +
  `pipeline/e2e` を計測し、pin 内閾値なら exit 0、逸脱なら
  `FAIL: <cell>:<reason>` で exit 1
- `mise run bench-frozen --selftest` が embed 欠落で pipeline
  locate 不成立(lag_mismatch + fingerprint_pin_diffs + stage
  失敗)と kernel validity reject を発生させ、かつ kernel B 側
  直列反復 + pipeline e2e wall 床上げの劣化注入で
  `ratio_below_threshold` / `e2e_norm_above_threshold` の閾値逸脱
  も発火させ、gate `fail` で exit 1
- `--write-baseline` 出力 candidate をそのまま pin path に置くと
  `approval` が null で gate が `baseline_not_promoted` 相当の
  fail を返す

## Guarantees

- 各 cell record が median/min/max・errors(error_detail 内訳)・
  work・verdict・reasons を JSONL で出力する
- run record が fixture 形状・measured_env(os/arch/cpu/cores/
  rustc/repo_sha/ruviform_sha)を記録する
- pipeline rep detail が artifacts/output_bytes/fingerprint 件数を
  rep 毎に記録する
- limiter(kernel: scalar naive vs FFT+pool 本番、pipeline:
  locate 支配 + I/O 小、relevance: `al run` と同一
  JobService::run 経路)を candidate と SKILL.md に記録する

## Non-goals

- CI 組込み・定期実行の自動化
- AVI/MP4 入力・export stage・GUI 経路の計測(後続 slice)
- 絶対時間での合否判定
- ruviform-math 等の共有基盤内部 kernel 計測(ruviform #807 が
  担当する面)

## Risk

- fingerprint は 1e-9 grid 量子化で float の機器差を吸収するが、
  grid 境界を跨ぐ jitter は pin 差分になり得る — 環境変更時は
  `measured_env` を見て repin 判断する運用で吸収する
- naive 参照は凍結だが最適化・simd 化しない条件 — 分母が変われば
  全 ratio が動くため、fixture/参照実装の変更は repin 必須
- pipeline wall 正規化は kernel median を分母にするため、両者の
  スケール因子が異なる機器(例: メモリ帯域だけ遅い)では比が
  動き得る — 1.5x 閾値で吸収する設計

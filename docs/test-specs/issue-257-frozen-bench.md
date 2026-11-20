---
id: "TSPEC-0A257frOzEnBeNchSLicE2xY"
type: "test-spec"
status: "draft"
title: "frozen benchmark harness 検証仕様(Issue #257 slice 2)"
created: "2026-10-31"
updated: "2026-11-21"
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

# frozen benchmark harness 検証仕様(Issue #257 slice 2)

## Purpose

Issue #257 の frozen benchmark harness(`crates/al-service/benches/
frozen_pipeline.rs`、standalone `harness=false`)が「本番 2 経路の
性能を、機器差に頑健な比で繰り返し検証し、回帰を fail-closed で
検出する」ことを担保する検証面を定義する。Ruviform Issue #807
(`dsp_frozen.rs`)の契約語彙・mode 構成・evidence 形を踏襲し、
AudioLens の pipeline 固有部(JobService E2E + fingerprint)を
`baseline/contracts.json` の正規化・3 分類先例に寄せて追加する。
slice 2 では pipeline arm をコンテナ横断(WAV/AVI/MP4)に拡張し、
stage plan を既定の `al run` plan(export 込み)へ揃える。

## Feature

`mise run bench-frozen` が、pinned baseline と照合する manual /
opt-in lane として、kernel arm(本番 locate method vs 凍結 naive
参照の in-process A/B 比)と pipeline arm(JobService::run E2E の
同一セッション参照 kernel 正規化比 + analysis payload
fingerprint 照合)を測定・判定できる。pipeline arm は 3 cell
(`pipeline/e2e`=WAV pair、`pipeline/avi`=非圧縮 AVI+PCM s16、
`pipeline/mp4`=`sowt` PCM `soun` track)を持ち、全 cell が
`RunSpec.stages=None` 経由の `default_stage_kinds()`
(`StageKind::PIPELINE_ORDER`、export 込み 7 stage)で走る。
media fixture は `al-testkit` の `write_avi`/`write_mp4` で
hermetic に生成し、外部 ffmpeg fixture に依存しない。

## Rules

- Rule: PRIMARY metric は絶対時間ではなく比。kernel cell は
  `median(A)/median(B)`(凍結 naive NCC は本ファイル内で不変の
  分母)、pipeline cell は `wall/ref_kernel_median` + locate stage
  wall 正規化比(同一セッション内で機器差を相殺する)。絶対時間は
  informational として記録する
- Rule: pipeline cell は `PIPELINE_CELLS` テーブル 1 箇所で定義
  する(`pipeline/e2e`/`pipeline/avi`/`pipeline/mp4`)。fixture の
  音声は 3 cell で bit 等価 — 同一の `to_s16` 出力を共通の
  little-endian 変換(`s16le_bytes`)で一度だけ byte 化し、
  WAV/AVI/MP4 各 container writer へ渡す。embed 構造(capture
  50s・template 1s・offset 20s・固定 seed)も cell 共通
- Rule: AVI fixture は `AviSpec::cfr` の CFR AVI(64x48 BGR24・
  10fps・30frame・video byte 固定)+ PCM s16 audio track。
  MP4 fixture は `write_mp4` の `soun` track 1 本・codec
  `sowt`(little-endian PCM)・mono/16bit/48kHz・`Mp4TableSpec::
  uniform`。両者とも symphonia/ruviform decoder の hermetic
  decode 経路を通る
- Rule: pipeline stage plan は `RunSpec.stages=None` —
  `default_stage_kinds()` が `StageKind::PIPELINE_ORDER`
  (ingest..export の 7 stage)を返し、`al run` CLI の既定 plan と
  同一になる。export stage wall は他 stage と同じ `stage_walls`
  map に記録される(特別扱いしない)。stage 数はハードコード
  せず `default_stage_kinds().len()` から数える
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
- Rule: fingerprint pin は pipeline cell 毎に持つ —
  baseline `cells["pipeline/<container>"].fingerprint` に格納する
  (schema v2)。top-level 単一 map(v1)では container 差分を
  表現できないため廃止。v1 baseline は schema 不一致として
  fail-closed で reject する(静かに差分 0 扱いしない)。
  export/ 配下は record-only なので従来通り fingerprint 対象外
  (export stage の wall は `stage_walls` で計測する)
- Rule: gate は fail-closed。pinned baseline が読めない・昇格
  (`approval`非 null)していない・schema 不一致、cell の閾値
  逸脱、error>0 のいずれでも fail。pin に無い cell は
  `unsupported`(run を fail させない)
- Rule: mode 契約 — 引数なし=実走(gate 適用、JSONL stdout)、
  `--test`=1 反復 smoke(閾値 gate は report_only、baseline
  unreadable は limiter note に落として継続 — smoke では
  fail-closed しない。correctness 由来の fail のみ exit 1)、
  `--selftest`=回帰注入(embed 欠落 fixture + kernel B 側直列反復
  + `pipeline/e2e` wall を pin `max_norm_ratio+1` 倍の ref median
  まで床上げ — pin に cell が無ければ ×8)で閾値逸脱と
  correctness 双方の gate fail を実証、
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
  `pipeline/e2e`/`pipeline/avi`/`pipeline/mp4` を計測し、pin 内
  閾値なら exit 0、逸脱なら `FAIL: <cell>:<reason>` で exit 1
- `mise run bench-frozen pipeline/avi` が cell filter で AVI
  cell のみを走らせる。`pipeline/mp4` は `sowt` decode 経路の
  回帰も拾う
- `mise run bench-frozen --selftest` が embed 欠落で全 pipeline
  cell の locate 不成立(lag_mismatch + fingerprint_pin_diffs +
  stage 失敗)と kernel validity reject を発生させ、かつ kernel
  B 側直列反復 + `pipeline/e2e` wall 床上げの劣化注入で
  `ratio_below_threshold` / `e2e_norm_above_threshold` の閾値逸脱
  も発火させ、gate `fail` で exit 1
- `--write-baseline` 出力 candidate をそのまま pin path に置くと
  `approval` が null で gate が `baseline_not_promoted` 相当の
  fail を返す

## Guarantees

- 各 cell record が median/min/max・errors(error_detail 内訳)・
  work・verdict・reasons を JSONL で出力する
- run record が fixture 形状(3 container・AVI 形状・既定 7
  stage plan)・measured_env(os/arch/cpu/cores/rustc/repo_sha/
  ruviform_sha)を記録する
- pipeline rep detail が artifacts/output_bytes/fingerprint 件数・
  export を含む stage_walls を rep 毎に記録する
- limiter(kernel: scalar naive vs FFT+pool 本番、pipeline:
  locate 支配 + export I/O 次点 + I/O・demux 小、relevance:
  `al run` と同一 JobService::run 経路 + 同一既定 plan)を
  candidate と SKILL.md に記録する

## repin runbook(2 段階・cross-machine)

repin は「candidate 生成→review→昇格」の 2 段階で行う。
`--write-baseline` は candidate を書くだけで pin を上書き
しないため、途中経過は必ず `approval: null` で fail-closed
に留まる。

1. candidate 生成: `mise run bench-frozen --write-baseline
   evidence/al257/baseline-candidate.json`。全 cell(kernel 4 +
   pipeline 3)を規定 reps で再測定し、cell 毎の fingerprint を
   含む candidate を出力する
2. review: candidate を pin(`crates/al-service/benches/frozen/
   baseline.al257.json`)と diff する。確認点:
   - `measured_env` の差分(機器/toolchain/repo+ruviform SHA)を
     記録し、repin 事由と突き合わせる
   - 各 cell の `measured_*` が閾値ルール(kernel: ceil(1+
     (measured-1)*0.5)、pipeline: ceil(measured*1.5))で妥当な
     余裕を持つか — measured が pin 閾値に張り付いている場合は
     計測ばらつきか真の劣化を切り分ける
   - pipeline cell の `fingerprint` map が cell 毎に存在し、
     ファイル数が container 差を反映しているか(AVI は video
     由来 artifact で WAV/MP4 より多い)
3. 昇格: candidate を pin path へ複写し `approval` を記入する
   (`kind`/`promoted_by`/`promoted_at`/`note` — 事由と環境差分
   を書く)。`approval: null` のままの baseline は gate が
   `baseline_not_promoted` で fail を返す
4. 実走検証: `mise run bench-frozen` が全 cell pass で exit 0
   になることを確認する。pin 後の最初の実走がその pin の受入
   証拠になる

cross-machine の考え方: 機器・toolchain・al*/ruviform SHA の
いずれかが変わった環境では、古い pin の fingerprint・正規化比が
成立するとは限らない。別機で gated run が fail したら、まず
`measured_env` 差分を確認し、正当な環境差なら当該環境で
candidate を生成・review・昇格して pin を更新する。同一機内で
`measured_env` 不変のまま閾値逸脱する場合は、それは環境差では
なく回帰として扱う(fail-closed のまま調査する)。

## Non-goals

- CI 組込み・定期実行の自動化
- AAC(`mp4a`)等の非 PCM MP4 cell — hermetic fixture 生成には
  実 ffmpeg 依存が必要で、frozen bench の hermetic 要件と
  矛盾するため対象外。real ffmpeg fixture を要する経路の
  計測は別 Issue で扱う
- JobOrchestrator arm — repo に該当型が存在しない(候補:
  `al` spawn+procmon、GUI `al.job.run`、StageRunner 直接起動)。
  slice 2 では未解決のまま後続 slice に送る
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

# Issue #257 — verification evidence

実行環境: windows/x86_64 — Intel64 Family 6 Model 158 Stepping 13 (16 cores),
rustc 1.99.1 (58cf8f9c3 2026-10-13), repo HEAD `7423546f`(slice 2 起点),
ruviform `e4c9f6f0`。bench profile。

frozen benchmark harness(`crates/al-service/benches/frozen_pipeline.rs`)の
slice 2 検証証拠。契約: `docs/test-specs/issue-257-frozen-bench.md`。
sibling 先例: ruviform `evidence/rf807/`。slice 1 の証拠は
`actions.jsonl`/`decision.tsv` の先頭側履歴に残す。

slice 2 変更点:
- pipeline arm を `PIPELINE_CELLS` テーブル 1 箇所で定義した 3 cell 化
  (`pipeline/e2e`=WAV pair、`pipeline/avi`=CFR AVI BGR24+PCM s16、
  `pipeline/mp4`=`sowt` PCM `soun` track)。fixture は `al-testkit` の
  `write_avi`/`write_mp4` で hermetic 生成、音声は共通 `to_s16` →
  `s16le_bytes` 変換で bit 等価
- `RunSpec.stages=None` により `default_stage_kinds()`
  (`StageKind::PIPELINE_ORDER`、export 込み 7 stage)を採用 —
  既定 `al run` plan と同一
- baseline schema v1→v2: fingerprint pin を top-level map から
  `cells["pipeline/<container>"].fingerprint` へ移設(v1 は fail-closed
  reject)
- selftest の pipeline wall 床上げを pin `max_norm_ratio+1` 倍の ref
  median へ(pin 追従の確定的逸脱、pin 無し時は従来の ×8)
- 2 段階 repin を実施: candidate 生成 → review → approval 記入で昇格。
  measured_env 差分は repo_sha のみ(別ファイル `measured-env-diff.json`)

## ファイル

| ファイル | 内容 |
| --- | --- |
| `actions.jsonl` | action → 観測結果の 1 行台帳(slice 1 + slice 2) |
| `run.jsonl` | `mise run bench-frozen` 実走 JSONL(7 reps・7 cells, gate pass) |
| `selftest.jsonl` | `--selftest` 回帰注入 → gate fail 証拠(exit 1) |
| `test.jsonl` | `--test` 1-rep smoke(exit 0、v2 pin 昇格後) |
| `write-baseline.jsonl` | `--write-baseline` candidate 生成 run の JSONL |
| `baseline-candidate.json` | emit された未昇格 candidate v2(`approval:null`) |
| `baseline-missing.jsonl` | 存在しない pin → fail(exit 1) |
| `baseline-unpromoted.jsonl` | 未昇格 candidate を pin 指定 → fail(exit 1) |
| `filter-nomatch.jsonl` | cell filter 0 一致 → gate `not_run`(exit 0) |
| `measured-env-diff.json` | v1 pin → v2 pin の measured_env 差分と repin 事由 |
| `decision.tsv` | hillclimb schema の判断台帳(slice 1 + slice 2) |
| `verification.md` | 本ファイル |

pinned baseline 本体は `crates/al-service/benches/frozen/baseline.al257.json`
(gate が読む正本 — evidence 側には candidate 複写のみ置く)。

## verdict(POLICIES §3 — 要求状態 × 実行結果)

| 項目 | 要求状態 | 結果 | 根拠 |
| --- | --- | --- | --- |
| pipeline container cell 拡張(e2e/avi/mp4) | Required | Pass | `run.jsonl` cells=7; `pipeline/avi`/`pipeline/mp4` が計測・gate 対象として記録される |
| hermetic media fixture(al-testkit、外部 ffmpeg 不使用) | Required | Pass | `write_avi(AviSpec::cfr)`/`write_mp4(sowt)`; `test.jsonl` 全 cell pass; `run.jsonl` rep_divergence=0 |
| 3 container で bit 等価 s16 PCM(変換 1 箇所) | Required | Pass | `s16le_bytes` 共通変換 — AVI/MP4 の locate が offset 一致で成立(errors=0, lag_mismatch=0) |
| export 込み既定 plan(`stages=None` → PIPELINE_ORDER) | Required | Pass | run record `fixture.pipeline.stages`=7(ingest..export); `stage_walls` に `export` キーが全 cell 全 rep で存在 |
| per-cell fingerprint pin(schema v2) | Required | Pass | candidate `cells[pipeline/*].fingerprint`; run で pin_diffs=0(e2e:18, avi:24, mp4:18 files) |
| v1 baseline の fail-closed reject | Required | Pass | smoke(v1 pin 残留時)が `baseline schema mismatch — expected al.frozen-pipeline-baseline/v2` を limiter note に記録 |
| error 計数・work 計数(全 cell) | Required | Pass | `error_detail` 内訳・work が 3 pipeline cell にも記録; selftest で errors>0 実証 |
| 2 段階 repin(candidate→昇格)+ measured_env 差分記録 | Required | Pass | `baseline-candidate.json` approval=null → pin 昇格 approval.kind=repin; `measured-env-diff.json`(repo_sha のみ差分) |
| `--selftest` gate-fail 実証(閾値逸脱 + correctness 両経路) | Required | Pass | exit 1; kernel 4cell `ratio_below_threshold` + `pipeline/e2e e2e_norm_above_threshold(8.930>7.93 = pin max+1)` + pipeline 3cell errors>0(embed 欠落) |
| cell filter(substring、0 一致 → not_run/exit 0) | Required | Pass | `filter-nomatch.jsonl` — gate `not_run`("no_cells_matched")・exit 0。`--test` 時の `pipeline/avi` 単 cell も動作 |
| baseline 未 read/未昇格 → fail-closed | Required | Pass | `baseline-missing.jsonl`/`baseline-unpromoted.jsonl` 共に exit 1(v2 候補でも不変) |
| pinned baseline 比較(ratio gate)実走 | Required | Pass | `run.jsonl` gate verdict=pass・`PASS: 7 cells within thresholds (reps=7)` exit 0 |
| A/A 決定性(pipeline fingerprint、全 container) | Required | Pass | 3 cell とも 7 rep `rep_divergence=0`・pin_diffs=0 |
| repin runbook 記述(cross-machine) | Required | Pass | test-spec `## repin runbook` 節 |
| `mise run fmt` | Required | Pass | exit 0(`cargo fmt --all` 適用後 --check 通過) |
| `check al-service` / `lint al-service` | Required | Pass | 各 exit 0(bench target 含め warnings 無し) |
| `test al-service` / `test al-media` | Required | Pass | al-service suite ok; al-media 154 tests all pass(sowt 系含む) |
| `prek run --all-files` | Required | Pass | exit 0 |
| AAC(`mp4a`)等の非 PCM MP4 cell | Deferred | NotRun | hermetic fixture には実 ffmpeg が必要 — frozen bench の要件と矛盾(Non-goals) |
| JobOrchestrator arm | Deferred | NotRun | repo に該当型無し — 後続 slice(Non-goals) |
| 異機器での pin 再現検証 | Deferred | NotRun | 単一機器で採取 — runbook に手順記録 |
| GUI/CLI 経由の bench 実行 | NotApplicable | — | lane は cargo bench 直接起動 |

## 採取値(実走 `run.jsonl`、pin `baseline.al257.json` v2)

| cell | 計測値 | pin 閾値 | verdict |
| --- | --- | --- | --- |
| kernel/full_ncc | ratio 62.63 | min 32.186 | pass |
| kernel/band_ncc | ratio 11.96 | min 6.627 | pass |
| kernel/gcc_phat | ratio 54.30 | min 23.649 | pass |
| kernel/spectral_envelope | ratio 11.03 | min 6.322 | pass |
| pipeline/e2e (wav) | norm 5.98 / locate 3.46 | max 7.93 / 4.64 | pass |
| pipeline/avi | norm 6.55 / locate 4.12 | max 7.96 / 4.69 | pass |
| pipeline/mp4 | norm 10.88 / locate 5.15 | max 13.05 / 5.67 | pass |

pipeline stage wall 中央値(実走、`stage_walls_ns.median_ns`):

| stage | e2e (wav) | avi | mp4 |
| --- | --- | --- | --- |
| ingest | ~0.13s | ~0.14s | ~2.0s (sowt decode) |
| locate | ~5.1s | ~5.2s | ~6.0s |
| cheap_analysis | ~0.75s | ~0.74s | ~1.1s |
| reference_analysis | ~0.42s | ~0.42s | ~0.44s |
| expensive_analysis | ~0.78s | ~0.76s | ~1.1s |
| dataset | ~0.07s | ~0.08s | ~0.74s |
| export | ~1.4s | ~1.4s | ~2.7s |

locate が依然支配的だが、export(dataset 複写 I/O)が第 2 の stage として
slice 1 で未計測だった約 15-20% の e2e wall を可視化した。

## repin 記録(2 段階)

1. `mise run bench-frozen --write-baseline
   evidence/al257/baseline-candidate.json` → v2 candidate
   (`approval:null`)生成 — `write-baseline.jsonl`
2. review: env 差分 = repo_sha のみ(`measured-env-diff.json`)。
   measured_norm(e2e 5.29/avi 5.31/mp4 8.70)が閾値ルールで妥当な
   余裕を確認
3. 昇格: pin path へ複写 + `approval.kind=repin, promoted_at=
   2026-11-21` 記入
4. 実走検証: `mise run bench-frozen` → PASS(exit 0) — 昇格 pin の
   受入証拠

## 範囲外・未対応

- AAC/`mp4a` MP4 cell・real ffmpeg fixture(Non-goals — hermetic 要件)
- JobOrchestrator arm(該当型が repo に存在しない — 後続 slice)
- `docs/**/README.md` 索引同期(coordinator 所管 — 本 worker は触れない)
- 本 verification は変更範囲のローカル検証。マージ後 main SHA での CI /
  統合検証が正式な受入証拠となる(本 lane は CI 非搭載の manual 証拠)

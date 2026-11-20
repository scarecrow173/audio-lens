# Issue #257 — verification evidence

実行環境: windows/x86_64 — Intel64 Family 6 Model 158 Stepping 13 (16 cores),
rustc 1.99.1 (58cf8f9c3 2026-10-13), repo HEAD `14d38f21`(実走時点
`origin/main` 一致), ruviform `e4c9f6f0`。bench profile。

frozen benchmark harness(`crates/al-service/benches/frozen_pipeline.rs`)の
slice 1 検証証拠。契約: `docs/test-specs/issue-257-frozen-bench.md`。
sibling 先例: ruviform `evidence/rf807/`。

## ファイル

| ファイル | 内容 |
| --- | --- |
| `actions.jsonl` | action → 観測結果の 1 行台帳 |
| `run.jsonl` | `mise run bench-frozen` 実走 JSONL(7 reps, gate pass) |
| `selftest.jsonl` | `--selftest` 回帰注入 → gate fail 証拠(exit 1) |
| `test.jsonl` | `--test` 1-rep smoke(exit 0) |
| `write-baseline.jsonl` | `--write-baseline` candidate 生成 run の JSONL |
| `baseline-candidate.json` | emit された未昇格 candidate(`approval:null`) |
| `baseline-missing.jsonl` | 存在しない pin → `baseline_unreadable` fail(exit 1) |
| `baseline-unpromoted.jsonl` | 未昇格 candidate を pin 指定 → fail(exit 1) |
| `filter-nomatch.jsonl` | cell filter 0 一致 → gate `not_run`(exit 0) |
| `decision.tsv` | hillclimb schema の判断台帳 |
| `verification.md` | 本ファイル |

pinned baseline 本体は `crates/al-service/benches/frozen/baseline.al257.json`
(gate が読む正本 — evidence 側には candidate 複写のみ置く)。

## verdict(POLICIES §3 — 要求状態 × 実行結果)

| 項目 | 要求状態 | 結果 | 根拠 |
| --- | --- | --- | --- |
| harness 実装(2 arm + CLI + gate) | Required | Pass | `cargo check -p al-service --benches` exit 0; `run.jsonl` 全 record |
| 決定的 fixture(al-testkit seed 固定 WAV + 固定 InputId) | Required | Pass | `run.jsonl` fixture record; pipeline fingerprint rep divergence=0 |
| 交互反復 ≥5 + median/range | Required | Pass | reps=7、A/B 交互; 各 cell record の stats(median/min/max) |
| error 計数(lag 不一致・validity reject・stage 失敗・fingerprint 差分) | Required | Pass | `error_detail` 内訳; `selftest.jsonl` で全経路が非零になることを実証 |
| work 計数(timed region の実作業) | Required | Pass | kernel: naive MAC+探索 lag ≈ 26.9G; pipeline: artifacts+output bytes ≈ 2.1G |
| limiter 記述 + e2e 関連性 | Required | Pass | candidate `limiter` 節; locate stage wall ≈ e2e の 70%(本番 69-79% と整合) |
| pinned baseline 比較(ratio gate) | Required | Pass | `run.jsonl` gate verdict=pass; 全 cell 閾値内 |
| `--selftest` gate-fail 実証(閾値逸脱 + correctness 両経路) | Required | Pass | exit 1; 4 kernel cells `ratio_below_threshold`(17.264<35.978 等) + `pipeline/e2e e2e_norm_above_threshold(8.000>6.56)` + 全 cell errors>0 |
| cell filter 0 一致 | Required | Pass | `filter-nomatch.jsonl` — gate `not_run`("no_cells_matched")・exit 0(rf807/cargo bench 慣習) |
| `--write-baseline` candidate(非昇格) | Required | Pass | `baseline-candidate.json` approval=null; 未昇格 pin 指定は fail |
| baseline 未 read/未昇格 → fail-closed | Required | Pass | `baseline-missing.jsonl`/`baseline-unpromoted.jsonl` 共に exit 1 |
| `mise run bench-frozen` lane(manual/opt-in) | Required | Pass | `run.jsonl` — `cargo bench --locked -p al-service --bench frozen_pipeline --` |
| A/A 決定性(pipeline fingerprint) | Required | Pass | 7 rep 全て `fingerprint_files=18`・`rep_divergence=0` |
| pipeline 正規化比(ref kernel median 分母) | Required | Pass | `ref_kernel` record; `norm_ratio_e2e=4.158`(pin 6.56)・`locate_norm=2.879`(pin 4.60) |
| `cargo fmt` | Required | Pass | `mise run fmt` exit 0 |
| `check-full` / `lint-full` / `test-full` | Required | Pass | 各 exit 0(workspace 全体) |
| `test al-service` / `test al-locate` | Required | Pass | 57+16+12+7+2 locate tests ok; al-service suite ok |
| `prek run --all-files` | Required | Pass | exit 0(全 hook passed/skipped) |
| AVI/MP4 入力・export stage 計測 | Deferred | NotRun | slice 1 範囲外(後続 slice) |
| 異機器での pin 再現検証 | Deferred | NotRun | 単一機器で採取 — repin 手順は test-spec Rules に記録 |
| GUI/CLI 経由の bench 実行 | NotApplicable | — | lane は cargo bench 直接起動 |

## 採取値(実走 `run.jsonl` より)

| cell | 計測値 | pin 閾値 | verdict |
| --- | --- | --- | --- |
| kernel/full_ncc | ratio 71.1 (A=1657ms/B=23ms) | min 35.978 | pass |
| kernel/band_ncc | ratio 13.8 (A=1653ms/B=120ms) | min 7.272 | pass |
| kernel/gcc_phat | ratio 62.5 (A=1654ms/B=26ms) | min 31.678 | pass |
| kernel/spectral_envelope | ratio 12.4 (A=1649ms/B=133ms) | min 6.538 | pass |
| pipeline/e2e | norm 4.158 (wall median 6.87s, range 6.76-7.09s) | max 6.56 | pass |
| pipeline locate stage | norm 2.879 (median 4759ms) | max 4.60 | pass |

ref kernel median = 1653ms(全 kernel cell A 側 pooled)。stage wall
内訳: locate 4759ms(69.3%) / expensive 762ms / cheap 724ms /
reference 408ms / ingest 123ms / dataset 66ms。

## 補足

- local 検証はこの変更範囲の妥当性を示すもので、merge 後の統合受入は
  merge-after-main SHA での CI 証拠が前提(POLICIES §3)。
- docs index(`docs/**/README.md`)は coordinator 所有のため今回は
  同期していない(意図的な未更新)。

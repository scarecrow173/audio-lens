# verification record — al274 proof redesign

## baseline

- worktree: `G:\orca\workspaces\audio-lens\al274-proof-redesign`
- branch: `feature/274-range-proof-redesign`
- baseline `origin/main`: `fb95c86a6a90b7b24255437de67ec923c9d09ad3`
  (PR #279 merge `17b98e3ca95b790a9e977f9a2ecb6ed87bed3532` と ruviform pin
  更新を含む)
- Kani environment: Windows host + WSL2 Ubuntu、Kani 0.68.0、CBMC 6.11.0。
  solver は runner 既定の CaDiCaL。solver 属性は使わない。

## canonical verification

| 区分 | command | exit | result |
| --- | --- | ---: | --- |
| canonical | `mise run fmt` | 0 | Pass。legacy proof 削除直後の空行 diff で一度 Fail し、cargo fmt 後に再実行 Pass |
| canonical | `mise run check al-media` | 0 | Pass |
| canonical | `mise run lint al-media` | 0 | Pass |
| canonical | `mise run test al-media` | 0 | Pass。75 unit tests + integration/golden/formal tests。mutation revert 後にも再実行 Pass |
| canonical | `mise run test-formal` | 0 | Pass。WAV proptest 6 件・containers proptest 7 件。mutation revert 後にも再実行 Pass |
| canonical | `AL_KANI_HARNESS_GROUP="wav-range" AL_KANI_TIMEOUT_MS=300000 AL_KANI_LOG_DIR=evidence/al274-proof-redesign/kani mise run test-formal-kani` | 0 | lane verdict `pass`。2/2 Required proof が `VERIFICATION:- SUCCESSFUL` |
| canonical mutation | mutation 状態で `AL_KANI_HARNESS_GROUP="wav-range" AL_KANI_TIMEOUT_MS=300000 AL_KANI_LOG_DIR=evidence/al274-proof-redesign/mutation-kani mise run test-formal-kani` | 1 | lane verdict `fail`。`wav_range_plan_valid_ranges_exact` が `VERIFICATION:- FAILED`。意図的失敗 |
| hygiene | `mise exec -- prek run --all-files` | 0 | Pass。trim whitespace/end-of-file/mixed line ending/biome/actionlint/workflow checks が通過 |
| 補助 | `mise exec -- node --check tools/run-formal-kani.mjs` | 0 | Pass(補助証拠。canonical = mise) |

## Kani verdict

最終 canonical lane raw log/verdict は `kani/` にある。

| proof | raw marker | elapsed | max RSS |
| --- | --- | ---: | ---: |
| `wav_range_plan_valid_ranges_exact` | `VERIFICATION:- SUCCESSFUL` | 10899 ms | 279020 KiB |
| `wav_range_plan_invalid_ranges_error` | `VERIFICATION:- SUCCESSFUL` | 4490 ms | 276720 KiB |

両 proof の合計 elapsed は 15389 ms で、設計目標 10〜20 分を大きく下回る。
timeout/not_run はない。exit 0 を Proof Pass と読み替えず、raw log の
`VERIFICATION:- SUCCESSFUL` と runner verdict `pass` を確認した。

## Able-to-Fail mutation

一時 mutation は production helper の byte length 計算を
`frame_count32 * block_align` から `frame_count32 + block_align` へ変える
arithmetic mutation だった。diagnostic payload mutation ではない。

- mutation lane: exit 1、lane verdict `fail`
- `wav_range_plan_valid_ranges_exact`: `VERIFICATION:- FAILED`、6671 ms、
  max RSS 279224 KiB
- `wav_range_plan_invalid_ranges_error`: `VERIFICATION:- SUCCESSFUL`、4619 ms
- mutation 後に exact string を戻し、`frame_count32) + block` が残らないこと、
  `mise run test al-media`、`mise run test-formal`、最終 wav-range lane Pass を
  確認した

## PR #281 / legacy proof との比較

PR #281 では pure helper 版 valid proof `wav_range_plan_valid_ranges_exact` が
約 300012 ms で timeout し Required NotRun、invalid proof は約 3.4〜4.5 s で
Pass だった。今回の最終 valid proof は 10899 ms、invalid proof は 4490 ms。
旧 whole-call proof は `evidence/al274/verification.md` で
`header_reads_valid_ranges_slice_exact` が 947 s timeout・2.5 GiB 到達時点でも
symex 中、`header_reads_invalid_ranges_error` が 631 s timeout・5.2 GiB 到達
時点でも symex 中と記録されている。直接同じ query ではないが、新 proof set は
writer fixture/decode call graph を proof 対象から外し、pure arithmetic helper
に限定したことで wall-clock とメモリの双方を大きく下げた。

旧 whole-call proof は exact valid replacement を含む wav-range lane Pass を
確認した後に retire した。proptest
`frame_count_and_range_reads_hold_the_contract` と
`writer_output_round_trips_losslessly` は弱めていない。spike harness は
canonical registry に入れていない。

## 計測条件と限界

`max_rss_kb` は runner が利用可能な `/usr/bin/time -v` で `cargo kani` 起動を
包み、GNU time の `Maximum resident set size` を raw log header から読んだ値。
個別 solver process だけを分離する cgroup 計測ではない。それでも 4 GiB
未満の根拠として、最終 2 proof は約 272 MiB であり十分な余裕がある。

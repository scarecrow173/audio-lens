# verification record — al274 chunk/scan proof

## baseline

- worktree: `G:\orca\workspaces\audio-lens\al274-chunk-scan-proof`
- branch: `feature/274-chunk-scan-proof`
- baseline `origin/main`: `b21f774d2bbfe1a74ffa599e3f09e03f13b2ef8b`
- PR #282 merge commit ancestor check: Pass（`git merge-base --is-ancestor b21f774... origin/main`）
- Kani environment: Windows host + WSL2 Ubuntu、Kani 0.68.0、CBMC 6.11.0。solver は runner 既定の CaDiCaL。`/usr/bin/time -v` による max RSS 計測あり。

## canonical verification

| 区分 | command | exit | result |
| --- | --- | ---: | --- |
| canonical | `mise run fmt` | 0 | Pass |
| canonical | `mise run check al-media` | 0 | Pass |
| canonical | `mise run lint al-media` | 0 | Pass |
| canonical | `mise run test al-media` | 0 | Pass。80 unit tests を含む al-media 回帰。mutation revert 後にも Pass |
| canonical | `mise run test-formal` | 0 | Pass。WAV 7 件・containers 7 件。structural regression を追加し、既存 proptest を弱めていない。mutation revert 後にも Pass |
| canonical | `AL_KANI_HARNESS_GROUP="wav-chunk-scan" AL_KANI_TIMEOUT_MS=600000 AL_KANI_LOG_DIR=evidence/al274-chunk-scan-proof/kani mise run test-formal-kani` | 0 | lane verdict `pass`。2/2 Required proof が `VERIFICATION:- SUCCESSFUL` |
| canonical regression | `AL_KANI_HARNESS_GROUP="wav-range" AL_KANI_TIMEOUT_MS=300000 AL_KANI_LOG_DIR=evidence/al274-chunk-scan-proof/kani-range-regression mise run test-formal-kani` | 0 | lane verdict `pass`。PR #282 の 2/2 proof が `VERIFICATION:- SUCCESSFUL` |
| canonical mutation（意図的失敗） | boundary mutation 状態で `AL_KANI_HARNESS_GROUP="wav-chunk-scan" AL_KANI_TIMEOUT_MS=600000 AL_KANI_LOG_DIR=evidence/al274-chunk-scan-proof/mutation-boundary-kani mise run test-formal-kani` | 1 | lane verdict `fail`。`wav_chunk_boundary_step_exact` が `VERIFICATION:- FAILED`。確認後に完全 revert |
| canonical mutation（意図的失敗） | scan mutation 状態で `AL_KANI_HARNESS_GROUP="wav-chunk-scan" AL_KANI_TIMEOUT_MS=600000 AL_KANI_LOG_DIR=evidence/al274-chunk-scan-proof/mutation-scan-kani mise run test-formal-kani` | 1 | lane verdict `fail`。`wav_chunk_scan_progress_bounded_exact` が `VERIFICATION:- FAILED`。確認後に完全 revert |
| canonical hygiene | `mise exec -- prek run --all-files` | 0 | Pass。auto-fix 差分なし |
| 補助 | `mise exec -- node --check tools/run-formal-kani.mjs` | 0 | Pass（補助証拠。canonical = mise） |
| 補助 | `git diff --name-only` / `git status --short` lease 照合 | 0 | lease 内ファイルのみ。docs index、過去 evidence、Cargo 関係に diff なし |

## Kani verdict

最終 wav-chunk-scan lane raw log/verdict は `kani/` にある。

| proof | raw marker | elapsed | max RSS |
| --- | --- | ---: | ---: |
| `wav_chunk_boundary_step_exact` | `VERIFICATION:- SUCCESSFUL` | 6922 ms | 288164 KiB |
| `wav_chunk_scan_progress_bounded_exact` | `VERIFICATION:- SUCCESSFUL` | 7592 ms | 300376 KiB |

合計 elapsed は 14514 ms で、10〜20 分の選択実行目標を下回る。timeout/not_run はない。exit 0 を Proof Pass と読み替えず、runner verdict `pass` と raw log marker を確認した。

wav-range regression raw log/verdict は `kani-range-regression/` にある。

| proof | raw marker | elapsed | max RSS |
| --- | --- | ---: | ---: |
| `wav_range_plan_valid_ranges_exact` | `VERIFICATION:- SUCCESSFUL` | 11339 ms | 284176 KiB |
| `wav_range_plan_invalid_ranges_error` | `VERIFICATION:- SUCCESSFUL` | 4490 ms | 281236 KiB |

Range proof の設計・implement・retire 判定はやり直していない。`wav.rs` を変更したため regression としてだけ再実行した。

## Able-to-Fail mutation

boundary mutation は `let pad = size & 1` を一時的に `let pad = 0` へ変える padding 加算漏れだった。lane exit 1、lane verdict `fail`、`wav_chunk_boundary_step_exact` が 6567 msで `VERIFICATION:- FAILED` になった。scan proof はこの mutation では Pass であり、padding exactness の主 oracle が boundary proof であることを確認できた。

scan mutation は `Advance.next_offset` を一時的に `offset` へ変える進行停滞だった。lane exit 1、lane verdict `fail`、`wav_chunk_scan_progress_bounded_exact` が 5713 msで `VERIFICATION:- FAILED` になった。boundary proof も exact next offset assertion で Fail した。両 mutation は exact string を戻して完全 revert し、revert 後に `mise run test al-media`、`mise run test-formal`、最終 wav-chunk-scan lane Pass を確認した。

## 旧新 cost の限定的比較

旧 whole-call proof は chunk/scan だけを分離した proof ではないため直接同値の比較ではない。参考として、`evidence/al274/verification.md` の trial では `decode_wav_never_panics_signed` が 1200 s cap・elapsed 1265 s・3.8 GiBで timeout、`wav_frame_count_never_panics_signed` が 600 s cap・elapsed 632 s・2.6 GiBで timeout、`wav_reads_never_panics_unsigned` が 300/900 s cap・elapsed 316/949 s・1.2→3.1 GiBで timeout だった。canonical 1500 s/proof lane ではこれらを含む 20 件が timeout であり、fmt-stub representative canonical では `wav_reads_never_panics_unsigned` が 13762 MiB に達しても timeout だった。新しい chunk/scan proof set は合計 14514 ms・最大 300376 KiBで verdict を返した。これは scope 内の限定的比較であり、完全な旧新 cost matrix は後続 slice に残す。

## 計測条件と限界

`max_rss_kb` は `/usr/bin/time -v` が `cargo kani` 起動を包んだ GNU time の `Maximum resident set size` であり、個別 solver process だけを分離する cgroup 計測ではない。最終 2 proof は約 282〜293 MiBで、4 GiB 未満の根拠として十分な余裕がある。legacy proof を含む全件 Kani lane は timeout が既知のため、この slice の必須検証にも成功条件にもしていない。

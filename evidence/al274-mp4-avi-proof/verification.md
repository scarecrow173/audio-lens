# MP4/AVI boundary exact proof verification

対象 branch: `feature/274-mp4-avi-proof`。baseline `origin/main` は PR #283 merge commit `244c9736bc9b00cac05963bd4e861d3becf37fb4` であり、作業開始時に `git merge-base --is-ancestor` exit 0 を確認した。実行環境は Windows worktree + WSL2 Ubuntu、Kani 0.68.0、CBMC 6.11.0、solver は Kani/CBMC default CaDiCaL。Kani lane は `/usr/bin/time -v` で各 `cargo kani` を包み、max RSS を計測した。

## canonical verification

| 区分 | コマンド | exit | 結果 |
| --- | --- | ---: | --- |
| canonical | `mise run fmt` | 0 | Pass |
| canonical | `mise run check al-media` | 0 | Pass |
| canonical | `mise run lint al-media` | 0 | Pass |
| canonical | `mise run test al-media` | 0 | Pass。87 unit test を含む対象クレート回帰 |
| canonical | `mise run test-formal` | 0 | Pass。WAV/MP4/AVI proptest を弱めていない |
| canonical mp4-avi lane | `AL_KANI_HARNESS_GROUP="mp4-avi-boundary" AL_KANI_TIMEOUT_MS=600000 AL_KANI_LOG_DIR=evidence/al274-mp4-avi-proof/kani mise run test-formal-kani` | 0 | lane verdict `pass`。2/2 Required proof が `VERIFICATION:- SUCCESSFUL` |
| canonical regression | `AL_KANI_HARNESS_GROUP="wav-chunk-scan" AL_KANI_TIMEOUT_MS=600000 AL_KANI_LOG_DIR=evidence/al274-mp4-avi-proof/kani-wav-chunk-scan-regression mise run test-formal-kani` | 0 | lane verdict `pass`。PR #283 の 2/2 proof が `VERIFICATION:- SUCCESSFUL` |
| canonical regression | `AL_KANI_HARNESS_GROUP="wav-range" AL_KANI_TIMEOUT_MS=300000 AL_KANI_LOG_DIR=evidence/al274-mp4-avi-proof/kani-wav-range-regression mise run test-formal-kani` | 0 | lane verdict `pass`。PR #282 の 2/2 proof が `VERIFICATION:- SUCCESSFUL` |
| canonical mutation（意図的失敗） | MP4 size0 off-by-one mutation 状態で mp4-avi-boundary lane、`AL_KANI_LOG_DIR=evidence/al274-mp4-avi-proof/mutation-mp4-kani` | 1 | lane verdict `fail`。`mp4_box_header_step_exact` が `VERIFICATION:- FAILED`。確認後に完全 revert |
| canonical mutation（意図的失敗） | AVI pad omission mutation 状態で mp4-avi-boundary lane、`AL_KANI_LOG_DIR=evidence/al274-mp4-avi-proof/mutation-avi-kani` | 1 | lane verdict `fail`。`avi_chunk_step_exact` が `VERIFICATION:- FAILED`。確認後に完全 revert |
| canonical hygiene | `mise exec -- prek run --all-files` | 0 | Pass。auto-fix 差分は lease 内だけ |
| 補助証拠（canonical = mise） | `mise exec -- node --check tools/run-formal-kani.mjs` | 0 | Pass。runner 構文確認 |
| 補助証拠（canonical = mise） | `git diff --name-only` / `git status --short` lease 照合 | 0 | lease 内ファイルだけ。docs index、過去 evidence、Cargo 関係、`wav.rs`、`formal_containers.rs` に diff なし |

最終的な数値は次の通りである。

| lane | proof | verdict marker | elapsed ms | max RSS KiB |
| --- | --- | --- | ---: | ---: |
| mp4-avi-boundary final | `mp4_box_header_step_exact` | `VERIFICATION:- SUCCESSFUL` | 12251 | 293580 |
| mp4-avi-boundary final | `avi_chunk_step_exact` | `VERIFICATION:- SUCCESSFUL` | 6119 | 294528 |
| wav-chunk-scan regression | `wav_chunk_boundary_step_exact` | `VERIFICATION:- SUCCESSFUL` | 6083 | 291768 |
| wav-chunk-scan regression | `wav_chunk_scan_progress_bounded_exact` | `VERIFICATION:- SUCCESSFUL` | 7496 | 304400 |
| wav-range regression | `wav_range_plan_valid_ranges_exact` | `VERIFICATION:- SUCCESSFUL` | 10796 | 288672 |
| wav-range regression | `wav_range_plan_invalid_ranges_error` | `VERIFICATION:- SUCCESSFUL` | 4668 | 285844 |

mp4-avi-boundary final lane の `cost_summary` は selected 2、verified 2、total elapsed 18370ms、max observed RSS 294528KiB、2/2 計測済みである。設計予算（各 proof 10 分以内・4GiB 未満、選択 lane 10〜20 分）を大きく下回る。timeout、exit 0、CI green を Proof Pass には使っていない。unwinding assertions は無効化していない。

## mutation detection

- MP4 mutation: size0 body length を `(limit - at) - 8` から `(limit - at) - 7` へ変えた。`mp4_box_header_step_exact` が 11320ms / 293404KiB で `VERIFICATION:- FAILED`、lane verdict `fail`。`avi_chunk_step_exact` は Pass だった。確認後に完全 revert した。
- AVI mutation: pad を `size & 1` から `0` へ変えた。`avi_chunk_step_exact` が 5539ms / 294696KiB で `VERIFICATION:- FAILED`、lane verdict `fail`。`mp4_box_header_step_exact` は Pass だった。確認後に完全 revert し、最終 laneで再検証した。

## 限定的な旧新 cost 比較

完全な旧新 cost matrix は後続 slice に残す。MP4/AVI に対応する legacy whole-call/container proof との限定的比較だけを記録する。

| legacy proof | trial evidence (`evidence/al274/verification.md`) | canonical 1500s/proof lane | 新 local proof |
| --- | --- | --- | --- |
| `mp4_writer_output_decodes` | 1200s cap timeout、elapsed 1265s、peak 4.5GiB | timeout。SIGTERM のため `/usr/bin/time` max RSS 未取得 | `mp4_box_header_step_exact` 12.251s / 293580KiB |
| `avi_writer_output_decodes` | 1200s cap timeout、elapsed 1267s、peak 3.9GiB | timeout。SIGTERM のため max RSS 未取得 | `avi_chunk_step_exact` 6.119s / 294528KiB |
| `mutated_mp4_never_panics` | 1200s cap timeout、elapsed 1266s、peak 2.4GiB | timeout。max RSS 未取得 | 同上（MP4 boundary arithmetic のみ） |
| `truncated_mp4_never_panics` | 900s cap timeout、elapsed 948s、peak 2.5GiB | timeout。max RSS 未取得 | 同上 |
| `mutated_avi_never_panics` | 1200s cap timeout、elapsed 1266s、peak 2.9GiB | timeout。max RSS 未取得 | 同上（AVI chunk boundary arithmetic のみ） |
| `truncated_avi_never_panics` | 900s cap timeout、elapsed 948s、peak 2.9GiB | timeout。max RSS 未取得 | 同上 |

この比較は保証範囲の等価性を意味しない。legacy proof は writer fixture、container decode、allocation、fmt、multi-branch parser を含む whole-call symbolic proof であり、timeout で形式保証を取得できていない。新 proof は production pure helper の boundary arithmetic に限定した exact proofであり、公開 API 全体 no-panic、sample table、movi timing、PCM/video decode、writer round-trip を証明しない。再実行コストは selected lane 約 18.4 秒で、legacy の 900〜1200 秒 trial timeout より小さいが、完全な代替ではない。

## 未証明領域

MP4-SCAN-003、MP4-TABLE-005、MP4-PCM declaration/extraction、AVI-MOVI-003、AVI-TIME-004、AVI-PCM、signature 再構成、writer/corruption whole-call proof、公開 API 全体 no-panic、I/O、allocation、`Vec` decode、diagnostic formatting、無制限 scan termination はこの slice の保証外である。one-step measure decrease を無制限 scan termination の機械証明と読み替えない。

# MP4 scan exact proof verification

対象 branch: `feature/274-mp4-scan-proof`。baseline `origin/main` は PR #284 merge commit `e163676ae3270ea64d10d9a8a502c642d6fc20e9` であり、作業開始時に `git merge-base --is-ancestor` exit 0 を確認した。実行環境は Windows worktree + WSL2 Ubuntu、Kani 0.68.0、CBMC 6.11.0、solver は Kani/CBMC default CaDiCaL。Kani lane は `/usr/bin/time -v` で各 `cargo kani` を包み、max RSS を計測した。

## canonical verification

| 区分 | コマンド | exit | 結果 |
| --- | --- | ---: | --- |
| canonical | `mise run fmt` | 0 | Pass |
| canonical | `mise run check al-media` | 0 | Pass |
| canonical | `mise run lint al-media` | 0 | Pass |
| canonical | `mise run test al-media` | 0 | Pass。90 unit test を含む対象クレート回帰。mutation revert 後に再実行 |
| canonical | `mise run test-formal` | 0 | Pass。WAV/MP4/AVI proptest を弱めていない。mutation revert 後に再実行 |
| canonical mp4-scan lane | `AL_KANI_HARNESS_GROUP="mp4-scan" AL_KANI_TIMEOUT_MS=600000 AL_KANI_LOG_DIR=evidence/al274-mp4-scan-proof/kani mise run test-formal-kani` | 0 | lane verdict `pass`。1/1 Required proof が `VERIFICATION:- SUCCESSFUL`。mutation revert 後の最終 lane |
| canonical regression | `AL_KANI_HARNESS_GROUP="mp4-avi-boundary" AL_KANI_TIMEOUT_MS=600000 AL_KANI_LOG_DIR=evidence/al274-mp4-scan-proof/kani-mp4-avi-boundary-regression mise run test-formal-kani` | 0 | lane verdict `pass`。PR #284 の 2/2 proof が `VERIFICATION:- SUCCESSFUL` |
| canonical regression | `AL_KANI_HARNESS_GROUP="wav-chunk-scan" AL_KANI_TIMEOUT_MS=600000 AL_KANI_LOG_DIR=evidence/al274-mp4-scan-proof/kani-wav-chunk-scan-regression mise run test-formal-kani` | 0 | lane verdict `pass`。PR #283 の 2/2 proof が `VERIFICATION:- SUCCESSFUL` |
| canonical regression | `AL_KANI_HARNESS_GROUP="wav-range" AL_KANI_TIMEOUT_MS=300000 AL_KANI_LOG_DIR=evidence/al274-mp4-scan-proof/kani-wav-range-regression mise run test-formal-kani` | 0 | lane verdict `pass`。PR #282 の 2/2 proof が `VERIFICATION:- SUCCESSFUL` |
| canonical mutation（意図的失敗） | scan progress mutation 状態で mp4-scan lane、`AL_KANI_LOG_DIR=evidence/al274-mp4-scan-proof/mutation-scan-kani` | 1 | lane verdict `fail`。`mp4_box_scan_progress_bounded_exact` が `VERIFICATION:- FAILED`。確認後に完全 revert |
| canonical hygiene | `mise exec -- prek run --all-files` | 0 | Pass |
| 補助証拠（canonical = mise） | `mise exec -- node --check tools/run-formal-kani.mjs` | 0 | Pass。runner 構文確認 |
| 補助証拠（canonical = mise） | peak memory 測定 | 0 | runner が各 Kani proof を `/usr/bin/time -v` で包み、max RSS を verdict detail に記録 |
| 補助証拠（canonical = mise） | `git diff --name-only` / `git status --short` lease 照合 | 0 | lease 内ファイルだけ。docs index、過去 evidence、Cargo 関係、`wav.rs`、`avi.rs`、`format.rs`、`formal_containers.rs` に diff なし |

最終的な Kani 数値は次の通りである。

| lane | proof | verdict marker | elapsed ms | max RSS KiB |
| --- | --- | --- | ---: | ---: |
| mp4-scan final | `mp4_box_scan_progress_bounded_exact` | `VERIFICATION:- SUCCESSFUL` | 36060 | 243584 |
| mp4-avi-boundary regression | `mp4_box_header_step_exact` | `VERIFICATION:- SUCCESSFUL` | 17007 | 298392 |
| mp4-avi-boundary regression | `avi_chunk_step_exact` | `VERIFICATION:- SUCCESSFUL` | 15050 | 296324 |
| wav-chunk-scan regression | `wav_chunk_boundary_step_exact` | `VERIFICATION:- SUCCESSFUL` | 13966 | 294304 |
| wav-chunk-scan regression | `wav_chunk_scan_progress_bounded_exact` | `VERIFICATION:- SUCCESSFUL` | 7831 | 305636 |
| wav-range regression | `wav_range_plan_valid_ranges_exact` | `VERIFICATION:- SUCCESSFUL` | 10907 | 290648 |
| wav-range regression | `wav_range_plan_invalid_ranges_error` | `VERIFICATION:- SUCCESSFUL` | 4530 | 288096 |

mp4-scan final lane の `cost_summary` は selected 1、verified 1、total elapsed 36060ms、max observed RSS 243584KiB、1/1 計測済みである。設計予算（proof 10 分以内・4GiB 未満、選択 lane 10〜20 分）を大きく下回る。timeout、exit 0、CI green を Proof Pass には使っていない。unwinding assertions は無効化していない。legacy proof を含む全件 Kani lane は既存 timeout が確定しているため、この slice の必須検証にも成功条件にもしていない。

## mutation detection

scan progress mutation では production helper `mp4_box_scan_next_offset` が返す next offset を `body_end` から `body_end - 1` へ一時的に変えた。mp4-scan lane は exit 1 / lane verdict `fail` で、`mp4_box_scan_progress_bounded_exact` が 7536ms / 247100KiB で `VERIFICATION:- FAILED` となった。失敗した check は `assertion failed: next_offset == advance.body_end` であり、boundary-only mutation や diagnostic payload だけの mutation ではなく、scan composition の進行規則を壊す mutation を scan oracle が検出した。確認後に mutation を完全 revert し、最終 mp4-scan lane、`mise run test al-media`、`mise run test-formal` を再実行して Pass を確認した。

## 限定的な旧新 cost 比較

完全な旧新 cost matrix は後続 slice に残す。MP4 scan/container walk に対応する legacy whole-call/container proof との限定的比較だけを記録する。

| legacy proof | trial evidence (`evidence/al274/verification.md`) | canonical 1500s/proof lane | 新 local proof |
| --- | --- | --- | --- |
| `mp4_writer_output_decodes` | 1200s cap timeout、elapsed 1265s、peak 4.5GiB | timeout。SIGTERM のため `/usr/bin/time` max RSS 未取得 | `mp4_box_scan_progress_bounded_exact` 36.060s / 243584KiB |
| `mutated_mp4_never_panics` | 1200s cap timeout、elapsed 1266s、peak 2.4GiB | timeout。max RSS 未取得 | 同上（scan position/transition arithmetic のみ） |
| `truncated_mp4_never_panics` | 900s cap timeout、elapsed 948s、peak 2.5GiB | timeout。max RSS 未取得 | 同上 |

この比較は保証範囲の等価性を意味しない。legacy proof は writer fixture、container decode、allocation、fmt、multi-branch parser を含む whole-call symbolic proof であり、timeout で形式保証を取得できていない。新 proof は production pure helper に接続した scan position/transition arithmetic に限定した bounded exact proofであり、公開 API 全体 no-panic、sample table、PCM decode、writer round-trip、AVI、無制限 scan termination を証明しない。再実行コストは selected lane 約 36.1 秒で、legacy の 900〜1200 秒 trial timeout より小さいが、完全な代替ではない。

## 未証明領域

4 個以上の valid box を受理する scan、MP4 semantic box type、duplicate/required box、skipped box path、sample table、PCM declaration/extraction、writer round-trip、deep nesting/多数 box scan、I/O、allocation、`Vec` decode、diagnostic formatting、公開 API 全体 no-panic、AVI、signature 再構成、writer/corruption whole-call proof はこの slice の保証外である。bounded scan model の成功を無制限 scan termination の機械証明と読み替えない。

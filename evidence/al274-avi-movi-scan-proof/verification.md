# AVI movi scan collection bounded exact proof verification

対象 branch: `feature/274-avi-movi-scan-proof`。baseline `origin/main` は PR #287 merge commit `eb3f034f52b9453f62b96d05bd2555f7a9138464` であり、作業開始時に `git merge-base --is-ancestor` exit 0 を確認した。実行環境は Windows worktree + WSL2 Ubuntu、Kani 0.68.0、CBMC 6.11.0、solver は Kani/CBMC default CaDiCaL。Kani lane は `/usr/bin/time -v` で各 `cargo kani` を包み、max RSS を計測した。

## canonical verification

| 区分 | コマンド | exit | 結果 |
| --- | --- | ---: | --- |
| canonical | `mise run fmt` | 0 | Pass。mutation revert 後の最終状態でも差分なし |
| canonical | `mise run check al-media` | 0 | Pass |
| canonical | `mise run lint al-media` | 0 | Pass |
| canonical | `mise run test al-media` | 0 | Pass。99 unit test を含む対象クレート回帰。mutation revert 後に再実行 |
| canonical | `mise run test-formal` | 0 | Pass。WAV/MP4/AVI proptest を弱めていない。mutation revert 後に再実行 |
| canonical avi-movi-scan lane | `AL_KANI_HARNESS_GROUP="avi-movi-scan" AL_KANI_TIMEOUT_MS=600000 AL_KANI_LOG_DIR=evidence/al274-avi-movi-scan-proof/kani mise run test-formal-kani` | 0 | lane verdict `pass`。1/1 Required proof が `VERIFICATION:- SUCCESSFUL`。mutation revert 後の最終 lane |
| canonical regression | `AL_KANI_HARNESS_GROUP="avi-movi-composition" AL_KANI_TIMEOUT_MS=600000 AL_KANI_LOG_DIR=evidence/al274-avi-movi-scan-proof/kani-avi-movi-composition-regression mise run test-formal-kani` | 0 | lane verdict `pass`。PR #287 の 1/1 proof が `VERIFICATION:- SUCCESSFUL` |
| canonical regression | `AL_KANI_HARNESS_GROUP="avi-movi-tag" AL_KANI_TIMEOUT_MS=600000 AL_KANI_LOG_DIR=evidence/al274-avi-movi-scan-proof/kani-avi-movi-tag-regression mise run test-formal-kani` | 0 | lane verdict `pass`。PR #286 の 1/1 proof が `VERIFICATION:- SUCCESSFUL` |
| canonical regression | `AL_KANI_HARNESS_GROUP="mp4-avi-boundary" AL_KANI_TIMEOUT_MS=600000 AL_KANI_LOG_DIR=evidence/al274-avi-movi-scan-proof/kani-mp4-avi-boundary-regression mise run test-formal-kani` | 0 | lane verdict `pass`。PR #284 の 2/2 proof が `VERIFICATION:- SUCCESSFUL` |
| canonical regression | `AL_KANI_HARNESS_GROUP="mp4-scan" AL_KANI_TIMEOUT_MS=600000 AL_KANI_LOG_DIR=evidence/al274-avi-movi-scan-proof/kani-mp4-scan-regression mise run test-formal-kani` | 0 | lane verdict `pass`。PR #285 の 1/1 proof が `VERIFICATION:- SUCCESSFUL` |
| canonical regression | `AL_KANI_HARNESS_GROUP="wav-chunk-scan" AL_KANI_TIMEOUT_MS=600000 AL_KANI_LOG_DIR=evidence/al274-avi-movi-scan-proof/kani-wav-chunk-scan-regression mise run test-formal-kani` | 0 | lane verdict `pass`。PR #283 の 2/2 proof が `VERIFICATION:- SUCCESSFUL` |
| canonical regression | `AL_KANI_HARNESS_GROUP="wav-range" AL_KANI_TIMEOUT_MS=300000 AL_KANI_LOG_DIR=evidence/al274-avi-movi-scan-proof/kani-wav-range-regression mise run test-formal-kani` | 0 | lane verdict `pass`。PR #282 の 2/2 proof が `VERIFICATION:- SUCCESSFUL` |
| canonical mutation（意図的失敗） | packet collect 欠落 mutation 状態で avi-movi-scan lane、`AL_KANI_LOG_DIR=evidence/al274-avi-movi-scan-proof/mutation-scan-kani` | 1 | lane verdict `fail`。`avi_movi_scan_collection_bounded_exact` が `VERIFICATION:- FAILED`。確認後に完全 revert |
| canonical hygiene | `mise exec -- prek run --all-files` | 0 | Pass |
| 補助証拠（canonical = mise） | `mise exec -- node --check tools/run-formal-kani.mjs` | 0 | Pass。runner 構文確認 |
| 補助証拠（canonical = mise） | peak memory 測定 | 0 | runner が各 Kani proof を `/usr/bin/time -v` で包み、max RSS を verdict detail に記録 |
| 補助証拠（canonical = mise） | `git diff --name-only` / `git status --short` lease 照合 | 0 | lease 内ファイルだけ。docs index、過去 evidence、Cargo 関係、`wav.rs`、`mp4.rs`、`format.rs`、`formal_containers.rs` に diff なし |

最終的な Kani 数値は次の通りである。

| lane | proof | verdict marker | elapsed ms | max RSS KiB |
| --- | --- | --- | ---: | ---: |
| avi-movi-scan final | `avi_movi_scan_collection_bounded_exact` | `VERIFICATION:- SUCCESSFUL` | 9962 | 244632 |
| avi-movi-composition regression | `avi_movi_entry_decision_exact` | `VERIFICATION:- SUCCESSFUL` | 5635 | 300096 |
| avi-movi-tag regression | `avi_movi_packet_tag_exact` | `VERIFICATION:- SUCCESSFUL` | 5181 | 294180 |
| mp4-avi-boundary regression | `mp4_box_header_step_exact` | `VERIFICATION:- SUCCESSFUL` | 10601 | 302712 |
| mp4-avi-boundary regression | `avi_chunk_step_exact` | `VERIFICATION:- SUCCESSFUL` | 6882 | 300992 |
| mp4-scan regression | `mp4_box_scan_progress_bounded_exact` | `VERIFICATION:- SUCCESSFUL` | 22944 | 305356 |
| wav-chunk-scan regression | `wav_chunk_boundary_step_exact` | `VERIFICATION:- SUCCESSFUL` | 6450 | 297952 |
| wav-chunk-scan regression | `wav_chunk_scan_progress_bounded_exact` | `VERIFICATION:- SUCCESSFUL` | 7559 | 309404 |
| wav-range regression | `wav_range_plan_valid_ranges_exact` | `VERIFICATION:- SUCCESSFUL` | 11302 | 294480 |
| wav-range regression | `wav_range_plan_invalid_ranges_error` | `VERIFICATION:- SUCCESSFUL` | 5234 | 291764 |

avi-movi-scan final lane の `cost_summary` は selected 1、verified 1、total elapsed 9962ms、max observed RSS 244632KiB、1/1 計測済みである。初回 cold-cache lane は 22557ms / 318068KiB であり、final lane は `kani-initial/` と区別して保存した。設計予算（proof 10 分以内・4GiB 未満、選択 lane 10〜20 分）を下回る。timeout、exit 0、CI green を Proof Pass には使っていない。unwinding assertions は無効化していない。legacy proof を含む全件 Kani lane は既存 timeout が確定しているため、この slice の必須検証にも成功条件にもしていない。

## mutation detection

packet collect 欠落 mutation では production scan-step helper `avi_movi_scan_step` の `Packet` arm を `Collect(tag)` から `Skip` へ一時的に変えた。avi-movi-scan lane は exit 1 / lane verdict `fail` で、`avi_movi_scan_collection_bounded_exact` が 9870ms / 245196KiB で `VERIFICATION:- FAILED` となった。失敗した check は `assertion failed: actual.packet_count == expected.packet_count` であり、boundary-only/tag-only/entry-decision-only mutation や diagnostic payload だけの mutation ではなく、scan collection の収集意味を壊す mutationを scan collection oracle が検出した。確認後に mutation を完全 revert し、最終 avi-movi-scan lane、`mise run test al-media`、`mise run test-formal` を再実行して Pass を確認した。

## 限定的な旧新 cost 比較

完全な旧新 cost matrix は後続 slice に残す。AVI movi/container walk に対応する legacy whole-call/container proof との限定的比較だけを記録する。legacy 数値は `evidence/al274-avi-movi-composition-proof/ledger.json` の limited cost comparison を read-only 引用した。

| legacy proof | trial evidence | canonical 1500s/proof lane | 新 local proof |
| --- | --- | --- | --- |
| `avi_writer_output_decodes` | 1200s cap timeout、elapsed 1267s、peak 3.9GiB | timeout。SIGTERM のため `/usr/bin/time` max RSS 未取得 | `avi_movi_scan_collection_bounded_exact` 9.962s / 244632KiB |
| `mutated_avi_never_panics` | 1200s cap timeout、elapsed 1266s、peak 2.9GiB | timeout。max RSS 未取得 | 同上（固定 3 件の scan collection のみ） |
| `truncated_avi_never_panics` | 900s cap timeout、elapsed 948s、peak 2.9GiB | timeout。max RSS 未取得 | 同上 |

local AVI proof 内訳として、PR #286 の `avi_movi_packet_tag_exact` regression は 5.181s / 294180KiB、PR #287 の `avi_movi_entry_decision_exact` regression は 5.635s / 300096KiB、今回の `avi_movi_scan_collection_bounded_exact` final lane は 9.962s / 244632KiB、初回 cold-cache lane は 22.557s / 318068KiB である。この比較は保証範囲の等価性を意味しない。legacy proof は writer fixture、container decode、allocation、fmt、multi-branch parser を含む whole-call symbolic proof であり、timeout で形式保証を取得できていない。新 proof は固定 3 件の direct child entry 列に対する scan collection composition に限定した bounded exact proofであり、unbounded iteration、N+1 件以上の列、実 `Vec` accumulation、nested walker、stream declaration、kind validity、packet body payload、timing accumulation、PCM/video decode、公開 API 全体 no-panic、writer round-trip を証明しない。再実行コストは selected lane 約 10 秒で、legacy の 900〜1200 秒 trial timeout より小さいが、完全な代替ではない。

## 未証明領域

unbounded finite iteration と termination、N+1 件以上の entry 列、任意長の entry 列、実 `Vec<MoviPacket>` accumulation、body slice/lifetime、outer RIFF/LIST walker、missing `movi`、`Chunks` iterator error precedence と文言生成、nested non-rec LIST の内容、OpenDML `rec ` の構造的解釈、stream declaration（`strh`/`strf`）、stream number と実在 stream の対応、stream existence、kind validity matrix、packet body payload、timing accumulation、remainder、PCM/video decode、partial ingest、writer round-trip、corruption whole-call proof、I/O、allocation、diagnostic formatting、公開 API 全体 no-panic、MP4、signature 再構成はこの slice の保証外である。scan collection proof Pass を AVI-MOVI-003 全体や AVI-TIME-004 の機械証明と読み替えない。

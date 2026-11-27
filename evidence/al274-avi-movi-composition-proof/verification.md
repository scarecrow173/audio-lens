# AVI movi entry composition exact proof verification

対象 branch: `feature/274-avi-movi-composition-proof`。baseline `origin/main` は PR #286 merge commit `0f3ebbf248fc2c398749456b674342506d789fcf` であり、作業開始時に `git merge-base --is-ancestor` exit 0 を確認した。実行環境は Windows worktree + WSL2 Ubuntu、Kani 0.68.0、CBMC 6.11.0、solver は Kani/CBMC default CaDiCaL。Kani lane は `/usr/bin/time -v` で各 `cargo kani` を包み、max RSS を計測した。

## canonical verification

| 区分 | コマンド | exit | 結果 |
| --- | --- | ---: | --- |
| canonical | `mise run fmt` | 0 | Pass。mutation revert 後の最終状態でも差分なし |
| canonical | `mise run check al-media` | 0 | Pass |
| canonical | `mise run lint al-media` | 0 | Pass |
| canonical | `mise run test al-media` | 0 | Pass。97 unit test を含む対象クレート回帰。mutation revert 後に再実行 |
| canonical | `mise run test-formal` | 0 | Pass。WAV/MP4/AVI proptest を弱めていない。mutation revert 後に再実行 |
| canonical avi-movi-composition lane | `AL_KANI_HARNESS_GROUP="avi-movi-composition" AL_KANI_TIMEOUT_MS=600000 AL_KANI_LOG_DIR=evidence/al274-avi-movi-composition-proof/kani mise run test-formal-kani` | 0 | lane verdict `pass`。1/1 Required proof が `VERIFICATION:- SUCCESSFUL`。mutation revert 後の最終 lane |
| canonical regression | `AL_KANI_HARNESS_GROUP="avi-movi-tag" AL_KANI_TIMEOUT_MS=600000 AL_KANI_LOG_DIR=evidence/al274-avi-movi-composition-proof/kani-avi-movi-tag-regression mise run test-formal-kani` | 0 | lane verdict `pass`。PR #286 の 1/1 proof が `VERIFICATION:- SUCCESSFUL` |
| canonical regression | `AL_KANI_HARNESS_GROUP="mp4-avi-boundary" AL_KANI_TIMEOUT_MS=600000 AL_KANI_LOG_DIR=evidence/al274-avi-movi-composition-proof/kani-mp4-avi-boundary-regression mise run test-formal-kani` | 0 | lane verdict `pass`。PR #284 の 2/2 proof が `VERIFICATION:- SUCCESSFUL` |
| canonical regression | `AL_KANI_HARNESS_GROUP="mp4-scan" AL_KANI_TIMEOUT_MS=600000 AL_KANI_LOG_DIR=evidence/al274-avi-movi-composition-proof/kani-mp4-scan-regression mise run test-formal-kani` | 0 | lane verdict `pass`。PR #285 の 1/1 proof が `VERIFICATION:- SUCCESSFUL` |
| canonical regression | `AL_KANI_HARNESS_GROUP="wav-chunk-scan" AL_KANI_TIMEOUT_MS=600000 AL_KANI_LOG_DIR=evidence/al274-avi-movi-composition-proof/kani-wav-chunk-scan-regression mise run test-formal-kani` | 0 | lane verdict `pass`。PR #283 の 2/2 proof が `VERIFICATION:- SUCCESSFUL` |
| canonical regression | `AL_KANI_HARNESS_GROUP="wav-range" AL_KANI_TIMEOUT_MS=300000 AL_KANI_LOG_DIR=evidence/al274-avi-movi-composition-proof/kani-wav-range-regression mise run test-formal-kani` | 0 | lane verdict `pass`。PR #282 の 2/2 proof が `VERIFICATION:- SUCCESSFUL` |
| canonical mutation（意図的失敗） | short-list reject 欠落 mutation 状態で avi-movi-composition lane、`AL_KANI_LOG_DIR=evidence/al274-avi-movi-composition-proof/mutation-composition-kani` | 1 | lane verdict `fail`。`avi_movi_entry_decision_exact` が `VERIFICATION:- FAILED`。確認後に完全 revert |
| canonical hygiene | `mise exec -- prek run --all-files` | 0 | Pass |
| 補助証拠（canonical = mise） | `mise exec -- node --check tools/run-formal-kani.mjs` | 0 | Pass。runner 構文確認 |
| 補助証拠（canonical = mise） | peak memory 測定 | 0 | runner が各 Kani proof を `/usr/bin/time -v` で包み、max RSS を verdict detail に記録 |
| 補助証拠（canonical = mise） | `git diff --name-only` / `git status --short` lease 照合 | 0 | lease 内ファイルだけ。docs index、過去 evidence、Cargo 関係、`wav.rs`、`mp4.rs`、`format.rs`、`formal_containers.rs` に diff なし |

最終的な Kani 数値は次の通りである。

| lane | proof | verdict marker | elapsed ms | max RSS KiB |
| --- | --- | --- | ---: | ---: |
| avi-movi-composition final | `avi_movi_entry_decision_exact` | `VERIFICATION:- SUCCESSFUL` | 3914 | 245116 |
| avi-movi-tag regression | `avi_movi_packet_tag_exact` | `VERIFICATION:- SUCCESSFUL` | 4720 | 290216 |
| mp4-avi-boundary regression | `mp4_box_header_step_exact` | `VERIFICATION:- SUCCESSFUL` | 9965 | 299020 |
| mp4-avi-boundary regression | `avi_chunk_step_exact` | `VERIFICATION:- SUCCESSFUL` | 6034 | 296940 |
| mp4-scan regression | `mp4_box_scan_progress_bounded_exact` | `VERIFICATION:- SUCCESSFUL` | 20154 | 301612 |
| wav-chunk-scan regression | `wav_chunk_boundary_step_exact` | `VERIFICATION:- SUCCESSFUL` | 6393 | 294464 |
| wav-chunk-scan regression | `wav_chunk_scan_progress_bounded_exact` | `VERIFICATION:- SUCCESSFUL` | 7596 | 305820 |
| wav-range regression | `wav_range_plan_valid_ranges_exact` | `VERIFICATION:- SUCCESSFUL` | 10611 | 290656 |
| wav-range regression | `wav_range_plan_invalid_ranges_error` | `VERIFICATION:- SUCCESSFUL` | 4414 | 288536 |

avi-movi-composition final lane の `cost_summary` は selected 1、verified 1、total elapsed 3914ms、max observed RSS 245116KiB、1/1 計測済みである。初回 cold-cache lane は 9458ms / 316100KiB であり、final lane は `kani-initial/` と区別して保存した。設計予算（proof 10 分以内・4GiB 未満、選択 lane 10〜20 分）を下回る。timeout、exit 0、CI green を Proof Pass には使っていない。unwinding assertions は無効化していない。legacy proof を含む全件 Kani lane は既存 timeout が確定しているため、この slice の必須検証にも成功条件にもしていない。

## mutation detection

short-list reject 欠落 mutation では production helper `avi_movi_entry_decision` の `None` arm を `RejectShortListType` から `Ignore` へ一時的に変えた。avi-movi-composition lane は exit 1 / lane verdict `fail` で、`avi_movi_entry_decision_exact` が 5935ms / 245716KiB で `VERIFICATION:- FAILED` となった。失敗した check は `assertion failed: matches!(decision, MoviEntryDecision::RejectShortListType) == short_expected` であり、boundary-only/tag-only mutation や diagnostic payload だけの mutation ではなく、movi entry composition semantic を壊す mutation を composition oracle が検出した。確認後に mutation を完全 revert し、最終 avi-movi-composition lane、`mise run test al-media`、`mise run test-formal` を再実行して Pass を確認した。

## 限定的な旧新 cost 比較

完全な旧新 cost matrix は後続 slice に残す。AVI movi/container walk に対応する legacy whole-call/container proof との限定的比較だけを記録する。

| legacy proof | trial evidence (`evidence/al274/verification.md`) | canonical 1500s/proof lane | 新 local proof |
| --- | --- | --- | --- |
| `avi_writer_output_decodes` | 1200s cap timeout、elapsed 1267s、peak 3.9GiB | timeout。SIGTERM のため `/usr/bin/time` max RSS 未取得 | `avi_movi_entry_decision_exact` 3.914s / 245116KiB |
| `mutated_avi_never_panics` | 1200s cap timeout、elapsed 1266s、peak 2.9GiB | timeout。max RSS 未取得 | 同上（direct child entry decision のみ） |
| `truncated_avi_never_panics` | 900s cap timeout、elapsed 948s、peak 2.9GiB | timeout。max RSS 未取得 | 同上 |

local AVI proof 内訳として、PR #286 の `avi_movi_packet_tag_exact` regression は 4.720s / 290216KiB、今回の `avi_movi_entry_decision_exact` final lane は 3.914s / 245116KiB、初回 cold-cache lane は 9.458s / 316100KiB である。この比較は保証範囲の等価性を意味しない。legacy proof は writer fixture、container decode、allocation、fmt、multi-branch parser を含む whole-call symbolic proof であり、timeout で形式保証を取得できていない。新 proof は production pure helper に接続した direct child entry semantic decision に限定した exact proofであり、finite iteration、`Vec` accumulation、nested walker、stream declaration、kind validity、packet body payload、timing accumulation、PCM/video decode、公開 API 全体 no-panic、writer round-trip を証明しない。再実行コストは selected lane 約 3.9 秒で、legacy の 900〜1200 秒 trial timeout より小さいが、完全な代替ではない。

## 未証明領域

finite movi iteration と termination、複数 entry ordering、`Vec<MoviPacket>` accumulation、body slice/lifetime、outer RIFF/LIST walker、missing `movi`、`Chunks` iterator error precedence と文言生成、nested non-rec LIST の内容、OpenDML `rec ` の構造的解釈、stream declaration（`strh`/`strf`）、stream number と実在 stream の対応、kind validity matrix、packet body payload、timing accumulation、remainder、PCM/video decode、partial ingest、writer round-trip、corruption whole-call proof、I/O、allocation、diagnostic formatting、公開 API 全体 no-panic、MP4、signature 再構成はこの slice の保証外である。entry composition proof Pass を AVI-MOVI-003 全体や AVI-TIME-004 の機械証明と読み替えない。

# Verification evidence: AVI video `strf` declaration proof

このファイルは `feature/274-avi-strf-declaration-proof` の検証記録。対象は Issue #274 全体ではなく、AVI-STREAM-002 の video `strf` BITMAPINFOHEADER declaration validation narrow sub-boundary bounded exact proof である。base `origin/main` は `614c0de439be4e1554668cf2a70f5910ac1c49e6` で、最終検証前に fetch して同じ SHA であることを確認した。

## 環境

- worktree: `G:/orca/workspaces/audio-lens/al274-strf-declaration`（Windows host path）
- mise canonical lane: Windows host。`cargo` 直接実行は canonical verdict として扱わない。
- Kani: runner の WSL mirror 経由の Ubuntu。peak memory は runner の `/usr/bin/time -v` 計測。
- timeout: mutation 前後の avi-strf-declaration lane と AVI/MP4/WAV regression は 600000 ms/proof。wav-range regression だけ 300000 ms/proof。

## canonical verification

| 区分 | コマンド | exit | 結果 |
| --- | --- | ---: | --- |
| canonical | `mise run fmt` | 0 | Pass。`cargo fmt --all -- --check` clean |
| canonical | `mise run check al-media` | 0 | Pass。対象 crate 型検査 |
| canonical | `mise run lint al-media` | 0 | Pass。lint 警告なし |
| canonical | `mise run test al-media` | 0 | Pass。unit/integration/doc test 全件 Pass。mutation revert後にも再実行して Pass |
| canonical | `mise run test-formal` | 0 | Pass。`formal_wav` 7/7、`formal_containers` 7/7。mutation revert後にも再実行して Pass |
| canonical Required proof | `AL_KANI_HARNESS_GROUP="avi-strf-declaration" AL_KANI_TIMEOUT_MS=600000 AL_KANI_LOG_DIR=evidence/al274-avi-strf-declaration-proof2/kani mise run test-formal-kani` | 0 | lane verdict `pass`。`avi_video_strf_declaration_bounded_exact` は `VERIFICATION:- SUCCESSFUL`。5903 ms / max RSS 250172 KiB。raw log/verdict は `kani/` |
| canonical mutation run（意図的失敗） | width offset を 4..8 から 5..9 へ一時 mutation 後、`AL_KANI_LOG_DIR=evidence/al274-avi-strf-declaration-proof2/mutation-strf-declaration-kani` で avi-strf-declaration lane | 1 | lane verdict `fail`。`VERIFICATION:- FAILED`、失敗 assertion は `fields.width == u32::from_le_bytes([prefix[4], prefix[5], prefix[6], prefix[7]])`。5766 ms / 250700 KiB。mutation は完全 revert 済み |
| canonical hygiene | `mise exec -- prek run --all-files` | 0 | Pass。auto-fix residue なし |
| 補助証拠（canonical = mise） | `mise exec -- node --check tools/run-formal-kani.mjs` | 0 | Pass。runner syntax check |
| 補助証拠（canonical = mise） | runner `/usr/bin/time -v` 記録確認 | 0 | 測定あり。max RSS は各 verdict JSON。4GiB 未満の測定値を確認 |

## Kani regression lanes

| group | exit | lane verdict | proof / cost |
| --- | ---: | --- | --- |
| avi-strh-declaration | 0 | pass | 1/1 Pass。5732 ms / max RSS 301356 KiB。`kani-avi-strh-declaration-regression/` |
| avi-stream-kind | 0 | pass | 1/1 Pass。14554 ms / max RSS 499884 KiB。`kani-avi-stream-kind-regression/` |
| avi-movi-scan | 0 | pass | 1/1 Pass。9217 ms / max RSS 314208 KiB。`kani-avi-movi-scan-regression/` |
| avi-movi-composition | 0 | pass | 1/1 Pass。5093 ms / max RSS 305244 KiB。`kani-avi-movi-composition-regression/` |
| avi-movi-tag | 0 | pass | 1/1 Pass。4647 ms / max RSS 298600 KiB。`kani-avi-movi-tag-regression/` |
| mp4-avi-boundary | 0 | pass | 2/2 Pass。15519 ms / max RSS 307588 KiB。`kani-mp4-avi-boundary-regression/` |
| mp4-scan | 0 | pass | 1/1 Pass。19551 ms / max RSS 310080 KiB。`kani-mp4-scan-regression/` |
| wav-chunk-scan | 0 | pass | 2/2 Pass。13123 ms / max RSS 314388 KiB。`kani-wav-chunk-scan-regression/` |
| wav-range | 0 | pass | 2/2 Pass。15014 ms / max RSS 299148 KiB。`kani-wav-range-regression/` |

legacy whole-call proof を含む全件 Kani lane は既存 timeout が確定しているため、この slice の必須検証でも成功条件でもない。上記 selective group を regression として実行した。

## 保証範囲と限定的 cost comparison

新しい Required proof は full `usize` length scalar と fixed symbolic `[u8; 40]` prefix に対して、length gate の exact complement、width/height/bits-per-pixel/compression の exact offset/endianness/decode、height の signed/`unsigned_abs` semantics、算術安全性を証明した。任意長 body slice composition、実 `Vec<StreamHead>`/`strl` 走査、`strf` presence/copy/ordering、missing-vs-empty、declaration correspondence、audio `strf`、decode suitability matrix、pixel/audio decode、partial ingest、公開 API 全体 no-panic、MP4、writer round trip、AVI-TIME-004 は証明しない。

限定的比較（read-only引用）では、関連 legacy whole-call AVI proof は `avi_writer_output_decodes` trial 1200s cap timeout/1267s/3.9GiB、`mutated_avi_never_panics` 1200s/1266s/2.9GiB、`truncated_avi_never_panics` 900s/948s/2.9GiB で、1500s canonical では timeout/RSS unavailable であった。今回の local proof は 5903 ms/250172 KiB、既存 AVI local proof regression は strh 5732 ms/301356 KiB、stream kind 14554 ms/499884 KiB、movi scan 9217 ms/314208 KiB、composition 5093 ms/305244 KiB、tag 4647 ms/298600 KiB。これは保証範囲が狭い local proof の再実行コスト比較であり、等価保証や完全 cost matrix ではない。

## ファイル

- machine-readable ledger: `evidence/al274-avi-strf-declaration-proof2/ledger.json`
- boundary 設計: `evidence/al274-avi-strf-declaration-proof2/boundary-analysis.md`
- action log: `evidence/al274-avi-strf-declaration-proof2/actions.jsonl`

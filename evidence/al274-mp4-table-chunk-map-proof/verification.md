# 検証結果: MP4 sample table stsc/chunk-map compositional proof

基点は `0e89c27690e7ab0e046d4ba48547969eeed3c213`（作業開始時の `origin/main`）。PR #302 merge commit が ancestor であることを `git merge-base --is-ancestor` exit 0 で確認した。最終検証前に `origin/main` `9b4a3545f95ea1cef1e26132f4adb06434e8df9c` へ fast-forward した。追加 commit は al-cli/docs/evidence であり、本 slice の `al-media` / Kani runner とは重ならないが、post-revert lane は merge 後に再実行した。

[境界分析](boundary-analysis.md) を実装前に確定し、direct multi-chunk materialization の solver/memory explosion を製品反例と区別して記録したうえで、最終 compositional boundary へ改訂した。ローカル検証は変更範囲の検証であり、統合検証は merge 後の対象 main SHA に対する CI 証拠で判定する。

## Runtime Verification

Windows x64 / MINGW64、rustc 1.99.1、mise 2026.11.2 で実行した。生ログは `native/` にある。

| コマンド | exit | 結果 |
| --- | ---: | --- |
| `mise run fmt` | 0 | Pass（`fmt.final.txt`） |
| `mise run check al-media` | 0 | Pass（`check-final.txt`） |
| `mise run lint al-media` | 0 | Pass（`lint.final.txt`） |
| `mise run test al-media` | 0 | Pass（`test-final.txt`） |
| `cargo test --locked -p al-media` | 0 | Pass（補助記録。127 lib tests、29 MP4 integration tests、13 registry tests、4 ruviform dep tests） |
| `mise run test-formal` | 0 | Pass。`formal_containers` 7 件、`formal_wav` 7 件 |
| `mise exec -- node --check tools/run-formal-kani.mjs` | 0 | Pass |
| `mise exec -- prek run --all-files` | 0 | Pass |

## Formal Verification

Ubuntu / WSL2、Kani Rust Verifier 0.68.0、CBMC 6.11.0 で実行した。runner は WSL mirror 内で実行し、Windows repo の target-dir 設定や `CARGO_TARGET_DIR` を変更していない。`AL_KANI_TIMEOUT_MS=600000`、unwinding assertions を無効化する runner flag はなく、新規 proof には `kani::assume`、fmt/alloc/IO stub がない。新規 proof の unwinding assertion failure は 0 件だった。

### Required proof group

コマンド:

```bash
AL_KANI_HARNESS_GROUP=mp4-table-chunk-map AL_KANI_TIMEOUT_MS=600000 AL_KANI_LOG_DIR=evidence/al274-mp4-table-chunk-map-proof/kani-canonical node tools/run-formal-kani.mjs
```

exit 0、lane verdict `pass`。14/14 raw log に `VERIFICATION:- SUCCESSFUL` がある。

| harness | verdict | elapsed | max RSS |
| --- | --- | ---: | ---: |
| `mp4_stsc_chunk_samples_len0_bounded_exact` | SUCCESSFUL | 40.540 s | 372368 KiB |
| `mp4_stsc_chunk_samples_len1_bounded_exact` | SUCCESSFUL | 93.919 s | 618720 KiB |
| `mp4_stsc_chunk_samples_len2_chunks0_bounded_exact` | SUCCESSFUL | 14.123 s | 856872 KiB |
| `mp4_stsc_chunk_samples_len2_chunks1_bounded_exact` | SUCCESSFUL | 18.581 s | 1411064 KiB |
| `mp4_stsc_chunk_samples_len2_chunks2_bounded_exact` | SUCCESSFUL | 13.626 s | 773948 KiB |
| `mp4_stsc_chunk_samples_len2_chunks3_bounded_exact` | SUCCESSFUL | 28.191 s | 2602632 KiB |
| `mp4_chunk_map_helper_spc1_len0_reject_bounded_exact` | SUCCESSFUL | 6.403 s | 369824 KiB |
| `mp4_chunk_map_helper_spc1_len1_bounded_exact` | SUCCESSFUL | 7.802 s | 371676 KiB |
| `mp4_chunk_map_helper_spc2_len1_reject_bounded_exact` | SUCCESSFUL | 7.978 s | 371812 KiB |
| `mp4_chunk_map_helper_spc2_len2_bounded_exact` | SUCCESSFUL | 8.901 s | 372772 KiB |
| `mp4_chunk_map_end_to_end_empty_bounded_exact` | SUCCESSFUL | 9.342 s | 572968 KiB |
| `mp4_chunk_map_end_to_end_single_sample_bounded_exact` | SUCCESSFUL | 15.643 s | 437036 KiB |
| `mp4_chunk_map_end_to_end_short_sizes_reject_bounded_exact` | SUCCESSFUL | 9.304 s | 397496 KiB |
| `mp4_chunk_map_end_to_end_overlong_sizes_reject_bounded_exact` | SUCCESSFUL | 14.931 s | 439896 KiB |

合計 289.284 s、最大観測 RSS 2602632 KiB。設計予算の 1 proof 600 秒以内・4 GiB 未満を満たす。

### Regression proof groups

| group | exit / verdict | harness | elapsed | max RSS |
| --- | --- | --- | ---: | ---: |
| `mp4-scan` | 0 / pass | `mp4_box_scan_progress_bounded_exact` | 20.388 s | 321624 KiB |
| `mp4-avi-boundary` | 0 / pass | `mp4_box_header_step_exact` | 10.056 s | 318520 KiB |
| `mp4-avi-boundary` | 0 / pass | `avi_chunk_step_exact` | 6.390 s | 316608 KiB |
| `mp4-pcm-declaration` | 0 / pass | `mp4_pcm_declaration_classification_exact` | 5.240 s | 316016 KiB |
| `mp4-pcm-declaration` | 0 / pass | `mp4_pcm_capability_width_exact` | 5.305 s | 320236 KiB |

raw log と verdict は `kani-mp4-scan-regression/`、`kani-mp4-avi-boundary-regression/`、`kani-mp4-pcm-declaration-regression/` にある。

## Mutation detection と byte-for-byte revert

semantic mutation として、production `append_chunk_samples` の `checked_add` overflow rejection を除去し、`wrapping_add` へ変更した。対象は chunk offset overflow semantics であり、diagnostic-only mutation ではない。

- mutation lane verdict: `fail`。
- failed harness: `mp4_chunk_map_helper_spc1_len1_bounded_exact, mp4_chunk_map_helper_spc2_len1_reject_bounded_exact, mp4_chunk_map_helper_spc2_len2_bounded_exact, mp4_chunk_map_end_to_end_single_sample_bounded_exact`。
- lane summary: 4 harness failed verification / 10 verified。
- mutation raw log: `mutation/kani/`。
- 復元前後 SHA-256: `28bd3995103ae41aed919953db6ec3ec8d2c344c5c9d4bb856755c963ba03e66`。
- `cmp` exit 0。byte-for-byte copy `mutation/mp4.rs.before-mutation` で復元した。

post-revert canonical lane は origin/main `9b4a3545f95ea1cef1e26132f4adb06434e8df9c` への fast-forward 後に再実行した。

```bash
AL_KANI_HARNESS_GROUP=mp4-table-chunk-map AL_KANI_TIMEOUT_MS=600000 AL_KANI_LOG_DIR=evidence/al274-mp4-table-chunk-map-proof/kani-post-revert node tools/run-formal-kani.mjs
```

exit 0、lane verdict `pass`。14/14 verified、合計 151.571 s、最大観測 RSS 2596700 KiB。

## Production seam と保証範囲

proof harness は production `chunk_samples`、`append_chunk_samples`、`build_chunk_map` を直接呼ぶ。`append_chunk_samples` は behavior-preserving な production refactor として `build_chunk_map` から抽出し、production path から実際に呼ばれる。proof-only implementation で本物の判定を迂回していない。

証明範囲は boundary-analysis の compositional boundary に限る。multi-chunk end-to-end materialization、spc 3/4、sizes len 3/4、3 件以上 sample、stts/ctts、parser integration、PCM extraction/conversion、diagnostics、I/O、allocation、公開 API 全体 no-panic は証明していない。

## Cost comparison

限定比較は [cost-comparison.md](cost-comparison.md) に記録した。direct multi-chunk materialization は verdict 前に memory budget を超えた一方、最終 compositional Required lane は 14/14 pass、289.284 s、最大 RSS 2602632 KiB だった。

## NotRun / Unsupported / NotApplicable

- 未選択 Kani group、legacy timeout proof を含む全件 lane: NotRun。この slice の必須検証ではない。
- 一度 `AL_KANI_HARNESS_GROUP` 指定漏れで全件 lane を開始した操作ミスは即時停止し、対象 group の証拠に使っていない。`actions.jsonl` に記録済み。
- 統合 CI: NotRun。merge 後の main SHA に対する CI 証拠は Coordinator が確認する。
- GUI/hardware/audio-device gate: NotApplicable。
- Windows `os error 206`: 発生せず。
- `/usr/bin/time -v`: WSL Ubuntu で利用可能であり、全 Kani proof で max RSS を測定した。

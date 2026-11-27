# 検証結果: AVI first-audio `wb` 連結

検証対象は `cf284b1924c9a7520d716e2be851b49284ebbd97`、開始時と
最終検証直前の `origin/main` は `70e979044adb1a829bcd5ae4baed54629653f714`。
ローカル検証は変更範囲を対象とし、統合検証は merge 後の対象 main SHA の
Daily CI 証拠で判定する。

## Runtime Verification

Windows x64、mise 2026.11.2、rustc 1.99.1。mutation を復元した後の実行。
コマンド別の全出力と exit は [final-canonical.txt](final-canonical.txt)。

| 区分 | コマンド | exit | 結果 |
| --- | --- | ---: | --- |
| canonical hygiene | `mise run fmt` | 0 | Pass |
| canonical runtime | `mise run check al-media` | 0 | Pass |
| canonical runtime | `mise run lint al-media` | 0 | Pass |
| canonical runtime | `mise run test al-media` | 0 | Pass、unit 118/118、integration/doc suites も Pass |
| canonical runtime | `mise run test-formal` | 0 | Pass、`formal_wav` 7/7、`formal_containers` 7/7 |
| canonical hygiene | `mise exec -- prek run --all-files` | 0 | Pass、[raw output](prek.txt)。[初回](prek-initial.txt)の整形後に再実行 |
| 補助（canonical = mise） | `mise exec -- node --check tools/run-formal-kani.mjs` | 0 | Pass |

`only_first_auds_stream_is_decoded` は他 `auds`、選択 stream の `zz`、空
`wb` を、16-bit sample の途中で分割した二つの選択 `wb` と混在させる。
decode 後の samples は既知の mono PCM と一致する。空 `wb` の fixture は
要求時の従来の unavailable 文言と、audio 非要求時の bypass を確認する。
`formal_containers` の missing/partial ingest、round-trip、mutation/truncation
と `formal_wav` は無変更。Kani の定理とは独立した実行証拠である。

## Formal Verification

Windows host から runner 経由で Ubuntu WSL2、Kani 0.68.0、CBMC 6.11.0
の既定 CaDiCaL を使用した。全 group は逐次実行。各コマンドの正確な引数・
exit・wall time は [canonical-formal.txt](canonical-formal.txt) に記録した。
各表の `dir` に runner の raw log と `verdict.json` がある。`exit 0`
のみを Pass とせず、lane `pass` と各 raw log の `VERIFICATION:- SUCCESSFUL`
を照合する。unwinding assertions は有効。`cfg(kani)` 中の依存
`al-testkit/src/expect.rs` の unused-variable warning は隠していない。
prek が raw log の行末空白と verdict/ledger の JSON 書式のみを整えた。
diff と JSON オブジェクトの照合で結果が変わっていないことを確認した。

| group | timeout ms | exit / verdict | proofs | wall ms | elapsed ms | max RSS KiB | dir |
| --- | ---: | --- | ---: | ---: | ---: | ---: | --- |
| `avi-wb-concatenation` | 600000 | 0 / `pass` | 1/1 | 45262 | 41666 | 822924 | [`kani/`](kani/) |
| `avi-strf-capture` | 600000 | 0 / `pass` | 3/3 | 31954 | 28116 | 435160 | [`kani-avi-strf-capture-regression/`](kani-avi-strf-capture-regression/) |
| `format-signature` | 600000 | 0 / `pass` | 1/1 | 13463 | 10797 | 435704 | [`kani-format-signature-regression/`](kani-format-signature-regression/) |
| `avi-strf-audio-declaration` | 600000 | 0 / `pass` | 1/1 | 12906 | 6209 | 305824 | [`kani-avi-strf-audio-declaration-regression/`](kani-avi-strf-audio-declaration-regression/) |
| `avi-strf-declaration` | 600000 | 0 / `pass` | 1/1 | 14105 | 6214 | 304664 | [`kani-avi-strf-declaration-regression/`](kani-avi-strf-declaration-regression/) |
| `avi-strh-declaration` | 600000 | 0 / `pass` | 1/1 | 10342 | 7899 | 307372 | [`kani-avi-strh-declaration-regression/`](kani-avi-strh-declaration-regression/) |
| `avi-stream-kind` | 600000 | 0 / `pass` | 1/1 | 22375 | 19985 | 500020 | [`kani-avi-stream-kind-regression/`](kani-avi-stream-kind-regression/) |
| `avi-movi-scan` | 600000 | 0 / `pass` | 1/1 | 13587 | 11124 | 319760 | [`kani-avi-movi-scan-regression/`](kani-avi-movi-scan-regression/) |
| `avi-movi-composition` | 600000 | 0 / `pass` | 1/1 | 21367 | 17735 | 310584 | [`kani-avi-movi-composition-regression/`](kani-avi-movi-composition-regression/) |
| `avi-movi-tag` | 600000 | 0 / `pass` | 1/1 | 51079 | 25442 | 304052 | [`kani-avi-movi-tag-regression/`](kani-avi-movi-tag-regression/) |
| `mp4-avi-boundary` | 600000 | 0 / `pass` | 2/2 | 27313 | 20520 | 313388 | [`kani-mp4-avi-boundary-regression/`](kani-mp4-avi-boundary-regression/) |
| `mp4-scan` | 600000 | 0 / `pass` | 1/1 | 27640 | 25119 | 316280 | [`kani-mp4-scan-regression/`](kani-mp4-scan-regression/) |
| `wav-chunk-scan` | 600000 | 0 / `pass` | 2/2 | 22572 | 19509 | 319516 | [`kani-wav-chunk-scan-regression/`](kani-wav-chunk-scan-regression/) |
| `wav-range` | 300000 | 0 / `pass` | 2/2 | 21860 | 18978 | 304736 | [`kani-wav-range-regression/`](kani-wav-range-regression/) |

回帰は 13 group、18/18 proof が Pass。合計 wall は 290563 ms、
合計 harness elapsed は 217647 ms、最大 RSS は 500020 KiB。
新規 proof の復元後の wall は 45262 ms、elapsed は 41666 ms、
最大 RSS は 822924 KiB で設計予算内。legacy を含む全件 lane は実行しない。

| 局所 proof 内訳 | elapsed ms | peak RSS KiB |
| --- | ---: | ---: |
| [`avi_audio_wb_concatenation_bounded_exact`](kani/kani-avi_audio_wb_concatenation_bounded_exact.txt) | 41666 | 822924 |
| [`avi_strl_entry_classification_exact`](kani-avi-strf-capture-regression/kani-avi_strl_entry_classification_exact.txt) | 9388 | 303628 |
| [`avi_strf_capture_ordering_bounded_exact`](kani-avi-strf-capture-regression/kani-avi_strf_capture_ordering_bounded_exact.txt) | 7976 | 305176 |
| [`avi_strf_materialize_bounded_exact`](kani-avi-strf-capture-regression/kani-avi_strf_materialize_bounded_exact.txt) | 10752 | 435160 |
| [`format_signature_detection_bounded_exact`](kani-format-signature-regression/kani-format_signature_detection_bounded_exact.txt) | 10797 | 435704 |
| [`avi_audio_strf_declaration_bounded_exact`](kani-avi-strf-audio-declaration-regression/kani-avi_audio_strf_declaration_bounded_exact.txt) | 6209 | 305824 |
| [`avi_video_strf_declaration_bounded_exact`](kani-avi-strf-declaration-regression/kani-avi_video_strf_declaration_bounded_exact.txt) | 6214 | 304664 |
| [`avi_strh_declaration_bounded_exact`](kani-avi-strh-declaration-regression/kani-avi_strh_declaration_bounded_exact.txt) | 7899 | 307372 |
| [`avi_stream_kind_classification_bounded_exact`](kani-avi-stream-kind-regression/kani-avi_stream_kind_classification_bounded_exact.txt) | 19985 | 500020 |
| [`avi_movi_scan_collection_bounded_exact`](kani-avi-movi-scan-regression/kani-avi_movi_scan_collection_bounded_exact.txt) | 11124 | 319760 |

### Able to Fail

`avi_selected_audio_pcm` の走査を一時的に `packets.iter().rev()` へ変更した。
同じ canonical group/600000 ms で exit 1、lane `fail`、
`VERIFICATION:- FAILED`。全長の一致は保たれ、`first selected payload byte
and order` と `second selected payload byte and order` が失敗した。
[raw log / verdict](mutation-wb-concatenation-kani/) と
[コマンド出力](mutation-wb-concatenation-lane.txt) を保持した。復元後に
`avi.rs` の blob が mutation 前と一致することを確認し、Runtime Verification
と final canonical lane を再実行した。

初回は oracle の可変 loop を CBMC が 858 回まで展開し、設計予算の
120 秒を超えたため SIGTERM で中断した。runner の exit 1 / `fail` と
`VERIFICATION:- FAILED` は中断によるもので、製品反例ではない。
[初回 raw log](kani-initial/) を残した。oracle を 3 × 2 byte に静的展開し、
同じ domain と本番 loop で [再試行](kani-static-oracle/) した。
2分・4 GiB は設計予算であり、600000 ms の runner timeout と区別する。

### 限定的なコスト比較

新 lane と全関連回帰の値は上の表に示す。elapsed は runner の per-harness
計測、wall は WSL の probe/mirror を含む mise コマンド全体。
peak RSS は GNU time `-v` の最大値で、合計ではない。旧 whole-call の
trial 値は [既存証跡](../al274/verification.md) からの引用であり、
同じ保証・実行環境として扱わない。

| legacy proof | trial timeout / wall | trial peak RSS | 旧 canonical |
| --- | --- | ---: | --- |
| `avi_writer_output_decodes` | 1200 s / 1267 s | 3.9 GiB | 1500 s timeout |
| `mutated_avi_never_panics` | 1200 s / 1266 s | 2.9 GiB | 1500 s timeout |
| `truncated_avi_never_panics` | 900 s / 948 s | 2.9 GiB | 1500 s timeout |

旧 canonical は [既存 verdict](../al274/residual/verdict.json) では
`not_run`、各 harness の canonical peak RSS は未記録。新旧の限定的な比較
であり、完全 cost matrix、legacy retire/supersede は未対応。

## Scope と未対応

証明は fixed 3 stream/3 packet と 0〜2 byte の `wb` 連結に限る。
任意長 copy・一般の `Vec` growth・nested walker・実 stream/packet table
蓄積・full declaration と missing `strh`/error precedence・missing-vs-empty
製品意味論・PCM decode/layout・first `vids`/映像 extraction・timing・
MP4/WAV の別 Property・公開 API 全体の no-panic には拡張しない。
詳細な exactness・接続・重い後続 Property は [境界分析](boundary-analysis.md)
と台帳に記録した。RF に還元する候補はなかった。

PR #293 gate で観測された format signature design 境界・evidence README
の欠落は本 slice の対象外。現在の base には PR #294 後の両記述が存在する。
旧 design 節・evidence は本 slice で変更していない。docs index は
Coordinator が統合時に整備する。

## Branch CI

[PR #297](https://github.com/scarecrow173/audio-lens/pull/297) 作成後、
branch head `bcfbb84dd04415105edec49e6511ce0f3fb53fb0` に対して
`gh run list --branch … --commit …` は `[]`（exit 0）、
`gh pr checks 297` は `no checks reported`（exit 1）だった。
raw output は [branch-ci.txt](branch-ci.txt)。Full CI は manual dispatch
のみ、Daily CI は `main` 向けのため、branch CI は **Unsupported**。
manual dispatch は実行していない。この証跡だけを足した最終 head の
再照合は PR 本文に記載する。

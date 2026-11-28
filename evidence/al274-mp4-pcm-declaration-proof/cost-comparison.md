# 限定 cost comparison: MP4 PCM declaration/capability

この比較は保証範囲の等価性を意味しない。旧 whole-call/container proof は writer fixture、container decode、allocation、fmt、sample table、PCM extraction を含む symbolic end-to-end proof であり、timeout で形式保証を取得できていない。今回の proof は fixed scalar declaration/capability seam に限定した exact proofであり、parser integration、sample table、PCM extraction/conversion、payload semantics を証明しない。

## 既存 evidence からの引用

| 対象 | 既存 evidence の cost | 結果 |
| --- | --- | --- |
| `mp4_writer_output_decodes` | 1200 s cap、elapsed 1265 s、peak 4.5 GiB（symex 中 sample） | timeout |
| `mutated_mp4_never_panics` | 1200 s cap、elapsed 1266 s、peak 2.4 GiB（symex 中 sample） | timeout |
| `truncated_mp4_never_panics` | 900 s cap、elapsed 948 s、peak 2.5 GiB（symex 中 sample） | timeout |
| `mp4_box_scan_progress_bounded_exact`（MP4 scan slice evidence） | 36.060 s / 243584 KiB | Pass |
| `mp4_box_header_step_exact`（MP4/AVI boundary slice evidence） | 12.251 s / 293580 KiB | Pass |

旧 RSS は CBMC/Kani driver の進行中サンプル、今回は `/usr/bin/time -v` の最大 resident set sizeである。計測方法と保証範囲が違うため、速度比・メモリ削減率は計算しない。

## 今回の proof family

初回 canonical lane（`kani/verdict.json`）:

| harness | elapsed | max RSS | verdict |
| --- | ---: | ---: | --- |
| `mp4_pcm_declaration_classification_exact` | 15.733 s | 336772 KiB | SUCCESSFUL |
| `mp4_pcm_capability_width_exact` | 5.750 s | 316184 KiB | SUCCESSFUL |
| 合計/最大 | 21.483 s | 336772 KiB | pass |

post-revert canonical lane（`kani-post-revert/verdict.json`）:

| harness | elapsed | max RSS | verdict |
| --- | ---: | ---: | --- |
| `mp4_pcm_declaration_classification_exact` | 7.976 s | 312888 KiB | SUCCESSFUL |
| `mp4_pcm_capability_width_exact` | 11.397 s | 315984 KiB | SUCCESSFUL |
| 合計/最大 | 19.373 s | 315984 KiB | pass |

時間には Cargo と Kani を含み、runner の env-detect と WSL mirror 準備は含まない。いずれも 1 proof 600 秒以内・ピーク RSS 4 GiB 未満の設計予算内である。関連 regression を含めた選択 lane の合計は mp4-scan 24.864 s、mp4-avi-boundary 19.552 sだった。

## 再実行コスト

`mp4-pcm-declaration` group は 2 proof だけを選択できるため、declaration/capability 変更時の再実行は全 42 件ではなく約 20 秒級である。MP4 scan/boundary を共有する変更では、本 slice と同じく `mp4-scan` と `mp4-avi-boundary` を選択再実行する。完全な旧新 cost matrix は後続 slice に残す。

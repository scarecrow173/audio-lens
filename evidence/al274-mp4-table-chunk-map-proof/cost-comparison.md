# Cost comparison: MP4 table chunk-map proof

この表は同じ保証範囲どうしの速度比ではない。direct end-to-end materialization は verdict を返さず手動停止した resource trial、最終 compositional family は Pass した Required proof である。速度比ではなく、boundary 設計がなぜ compositional になったかを示す。

## Direct/早期 structural trial（manual abortまたは診断）

| trial | 結果 | 観測 cost |
| --- | --- | --- |
| unified dynamic validation, unwind8 | verdict なし・manual abort | 約 4 分で RSS 約 3.2 GiB |
| unified dynamic validation, unwind6 | verdict なし・manual abort | 1m42s / 17,764,508 KiB |
| len2 all-chunks combined validation | verdict なし・manual abort | 約 1 分 / 14,092,868 KiB |
| len2/chunks3 validation, unwind6 | verdict なし・manual abort | 33 s / 11,881,524 KiB |
| len2/chunks3 validation, unwind4 | PASS | 30.104 s / 2,587,916 KiB |
| direct multi-chunk materialization 各種 | verdict なし・manual abort | 2 分台で 8.7〜20.4 GiB 級 |
| fixed-shape/full-width/direct-spc/count-match/smallnum/static-oracle/exact-length diagnostics | verdict なし・manual abort | いずれも急増し budget 非適合 |
| combined lower-bound diagnostic | verdict なし・manual abort | 50 s / 4,626,496 KiB |
| canonical trial len2 chunks0+1, unwind4 | verdict なし・manual abort | 28 s / 4,891,052 KiB |
| canonical trial len2 chunks2, unwind4 | verdict なし・manual abort | 95 s / 4,373,808 KiB |

これらの abort は製品反例ではない。`kani::assume`、stub、assertion 削減で cost を隠さず、structural split・production seam extraction・explicit residual NotRun に分解した。

## Final Required canonical lane

`AL_KANI_HARNESS_GROUP=mp4-table-chunk-map AL_KANI_TIMEOUT_MS=600000 AL_KANI_LOG_DIR=evidence/al274-mp4-table-chunk-map-proof/kani-canonical node tools/run-formal-kani.mjs`

| metric | value |
| --- | ---: |
| selected / verified | 14 / 14 |
| total elapsed | 289.284 s |
| max observed RSS | 2,602,632 KiB |
| per-proof timeout budget | 600,000 ms |
| RSS design ceiling | < 4,194,304 KiB |

最高 cost は `mp4_stsc_chunk_samples_len2_chunks3_bounded_exact`（28.191 s / 2,602,632 KiB）。helper materialization は 6.403〜8.901 s、約 370 MiB、end-to-end composition は 9.304〜15.643 s、約 390〜573 MiBだった。direct multi-chunk proof が budget を超えた一方、production seam への分解で full-width u32/u64 scalar arithmetic を保持したまま予算内に入った。

legacy whole-call timeout proof との直接 speedup は、保証範囲・入力構造・計測条件が違うため計算しない。

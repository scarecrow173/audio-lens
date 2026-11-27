# Verification — AVI strf capture proof

対象 branch は `feature/274-avi-strf-capture-proof`。最終確認時の `origin/main` は `ad277403c1ae7609eee29e1bfd361a9c5715fc33` で、PR #293 merge commit `a64d3566190a0442486117d32688fbbf7177e9fb` は HEAD の祖先である。canonical `fmt`/`check`/`lint`/`test`/`test-formal` は Windows host path `G:/orca/workspaces/audio-lens/al274-strf-capture`、Kani lane は runner 経由の Ubuntu (WSL2) で実行した。ローカル検証は変更範囲の検証であり、統合検証は merge 後の対象 SHA に対する CI 証拠で確認する。

## Canonical verification table

| 区分 | コマンド | exit | 結果 |
| --- | --- | ---: | --- |
| canonical | `mise run fmt` | 0 | Pass。raw output は `final-canonical.txt` |
| canonical | `mise run check al-media` | 0 | Pass |
| canonical | `mise run lint al-media` | 0 | Pass |
| canonical | `mise run test al-media` | 0 | Pass。unit 117/117、integration/doc suite 全件 Pass。mutation revert後に再実行 |
| canonical | `mise run test-formal` | 0 | Pass。`formal_wav` 7/7、`formal_containers` 7/7。mutation revert後に再実行 |
| canonical Required proof | `AL_KANI_HARNESS_GROUP="avi-strf-capture" AL_KANI_TIMEOUT_MS=600000 AL_KANI_LOG_DIR=evidence/al274-avi-strf-capture-proof/kani mise run test-formal-kani` | 0 | lane verdict `pass`。3/3 proofが `VERIFICATION:- SUCCESSFUL`。raw log/verdictは `kani/` |
| canonical post-revert Required proof | 同 lane、`AL_KANI_LOG_DIR=evidence/al274-avi-strf-capture-proof/kani-avi-strf-capture-regression` | 0 | lane verdict `pass`。3/3 Pass、16779 ms / max RSS 435464 KiB |
| canonical mutation run（意図的失敗） | `avi_strf_capture_step` を first-write-wins化した一時 mutation後、`AL_KANI_LOG_DIR=evidence/al274-avi-strf-capture-proof/mutation-strf-capture-kani` で avi-strf-capture lane | 1 | lane verdict `fail`。`avi_strf_capture_ordering_bounded_exact` が `VERIFICATION:- FAILED`。mutationは完全 revert済み |
| canonical hygiene | `mise exec -- prek run --all-files` | 0 | Pass。auto-fix residueなし。raw outputは `prek.txt` |
| 補助証拠（canonical = mise） | `mise exec -- node --check tools/run-formal-kani.mjs` | 0 | Pass。runner syntax check |
| 補助証拠（canonical = mise） | runner `/usr/bin/time -v` 記録確認 | 0 | 全Kani proofで測定あり。max RSSは各 verdict JSON。4GiB未満の測定値を確認 |

## Kani regression lanes

| group | exit | lane verdict | proof / cost |
| --- | ---: | --- | --- |
| avi-strf-capture | 0 | pass | 3/3 Pass。16779 ms / max RSS 435464 KiB。`kani-avi-strf-capture-regression/` |
| format-signature | 0 | pass | 1/1 Pass。8324 ms / max RSS 435372 KiB。`kani-format-signature-regression/` |
| avi-strf-audio-declaration | 0 | pass | 1/1 Pass。5127 ms / max RSS 304860 KiB。`kani-avi-strf-audio-declaration-regression/` |
| avi-strf-declaration | 0 | pass | 1/1 Pass。4886 ms / max RSS 304028 KiB。`kani-avi-strf-declaration-regression/` |
| avi-strh-declaration | 0 | pass | 1/1 Pass。5448 ms / max RSS 306456 KiB。`kani-avi-strh-declaration-regression/` |
| avi-stream-kind | 0 | pass | 1/1 Pass。14251 ms / max RSS 500016 KiB。`kani-avi-stream-kind-regression/` |
| avi-movi-scan | 0 | pass | 1/1 Pass。8716 ms / max RSS 318852 KiB。`kani-avi-movi-scan-regression/` |
| avi-movi-composition | 0 | pass | 1/1 Pass。5378 ms / max RSS 309644 KiB。`kani-avi-movi-composition-regression/` |
| avi-movi-tag | 0 | pass | 1/1 Pass。4854 ms / max RSS 303568 KiB。`kani-avi-movi-tag-regression/` |
| mp4-avi-boundary | 0 | pass | 2/2 Pass。16493 ms / max RSS 311908 KiB。`kani-mp4-avi-boundary-regression/` |
| mp4-scan | 0 | pass | 1/1 Pass。20080 ms / max RSS 314516 KiB。`kani-mp4-scan-regression/` |
| wav-chunk-scan | 0 | pass | 2/2 Pass。13830 ms / max RSS 319516 KiB。`kani-wav-chunk-scan-regression/` |
| wav-range | 0 | pass | 2/2 Pass。17772 ms / max RSS 304268 KiB。`kani-wav-range-regression/` |

regression group の合計 elapsed は 141938 ms だった。legacy timeout proofを含む全件 laneはこの sliceの必須検証でも成功条件でもない。

## 保証範囲と限定的 cost comparison

Required proof familyは、full `[u8;4]` idに対する exact `strh`/`strf`/other classification、固定3件の symbolic class列に対する capture absence/presence/last-write-wins/non-`strf` non-interference、payload length 0/1/2の materialize後 length/content/byte orderを証明した。class selectorは full `u8` の剰余写像による冗長 encodingであり、全 3 variantを覆い、input narrowingではない。payload byteは len 1/2で実在する位置の全 bit patternを覆う。

production capture reducerは generic tokenに対する helperとして実装され、productionでは `&[u8]`、ordering proofでは bounded payload tokenに instantiateされる。reducerが tokenを inspectしないことは実装構造で確認でき、byte copy自体は `avi_strf_materialize_bounded_exact` と最小 AVI fixtureを通る deterministic production-shell regressionが接続する。この generic instantiation と nested `Chunks` walker/実 parser shellの間には意図的な boundaryがあり、full parser correctnessへ転用しない。

この local proofは 4件以上の entry列、len 3以上の copy、任意長 body slice、実 `Vec` growth、`Chunks` iterator、error precedence、missing `strh` filter、実 `Vec<StreamHead>` accumulation、full declaration correspondence、missing-vs-empty product semantics、first stream selection、`wb` scan/concatenation、PCM、timing、writer round-trip、corruption no-panic、I/O、allocation一般、diagnostic formatting、公開 API全体 no-panic、MP4、format signature semanticsを証明しない。

限定的比較（read-only引用）では、legacy AVI writer/corruption proofは `avi_writer_output_decodes` trial 1200s cap timeout / 1267s / 3.9GiB、`mutated_avi_never_panics` 1200s / 1266s / 2.9GiB、`truncated_avi_never_panics` 900s / 948s / 2.9GiB だった。新規 avi-strf-capture familyは post-revert laneで 16779 ms / max RSS 435464 KiB、全 regression合計は 141938 ms だった。これは scope内の限定的比較であり、等価保証や完全 cost matrixではない。

## Branch CI

PR作成後に branch head SHAのCIを read-onlyで確認し、結果または Unsupported 理由を `actions.jsonl` と PR本文に記録する。manual dispatchは行わない。

## ファイル

- machine-readable ledger: `evidence/al274-avi-strf-capture-proof/ledger.json`
- boundary設計: `evidence/al274-avi-strf-capture-proof/boundary-analysis.md`
- action log: `evidence/al274-avi-strf-capture-proof/actions.jsonl`
- canonical output: `evidence/al274-avi-strf-capture-proof/final-canonical.txt`
- mutation raw log/verdict: `evidence/al274-avi-strf-capture-proof/mutation-strf-capture-kani/`
- regression raw logs/verdicts: `evidence/al274-avi-strf-capture-proof/kani-*-regression/`

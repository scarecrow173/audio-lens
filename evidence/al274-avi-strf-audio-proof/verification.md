# Verification — AVI audio `strf` WAVEFORMATEX declaration proof

対象 branch は `feature/274-avi-strf-audio-proof`。開始時・最終確認時ともに `origin/main` は PR #291 merge commit `af20701ba3ed64efc4964c9c1faaea18139eb6cb` で、`git merge-base --is-ancestor af20701ba3ed64efc4964c9c1faaea18139eb6cb HEAD` は exit 0 だった。canonical `fmt`/`check`/`lint`/`test`/`test-formal` は Windows host path `G:/orca/workspaces/audio-lens/al274-strf-audio`、Kani lane は runner 経由の Ubuntu (WSL2) で実行した。ローカル検証は変更範囲の検証であり、統合検証は merge 後の対象 SHA に対する CI 証拠で確認する。

## Canonical verification table

| 区分 | コマンド | exit | 結果 |
| --- | --- | ---: | --- |
| canonical | `mise run fmt` | 0 | Pass。mutation revert 後にも再実行 |
| canonical | `mise run check al-media` | 0 | Pass |
| canonical | `mise run lint al-media` | 0 | Pass |
| canonical | `mise run test al-media` | 0 | Pass。unit 110/110、integration/doc suite 全件 Pass。mutation revert 後にも再実行 |
| canonical | `mise run test-formal` | 0 | Pass。`formal_wav` 7/7、`formal_containers` 7/7。mutation revert 後にも再実行 |
| canonical Required proof | `AL_KANI_HARNESS_GROUP="avi-strf-audio-declaration" AL_KANI_TIMEOUT_MS=600000 AL_KANI_LOG_DIR=evidence/al274-avi-strf-audio-proof/kani mise run test-formal-kani` | 0 | lane verdict `pass`。`avi_audio_strf_declaration_bounded_exact` は `VERIFICATION:- SUCCESSFUL`。4265 ms / max RSS 249568 KiB。raw log/verdict は `kani/` |
| canonical mutation run（意図的失敗） | sample_rate LE input bytes を 4..8 から 5..9 へ一時 mutation 後、`AL_KANI_LOG_DIR=evidence/al274-avi-strf-audio-proof/mutation-audio-strf-declaration-kani` で avi-strf-audio-declaration lane | 1 | lane verdict `fail`。`VERIFICATION:- FAILED`、失敗 assertion は `fields.sample_rate == u32::from_le_bytes([prefix[4], prefix[5], prefix[6], prefix[7]])`。6667 ms / 249160 KiB。mutation は完全 revert 済み |
| canonical hygiene | `mise exec -- prek run --all-files` | 0 | Pass。auto-fix residue なし |
| 補助証拠（canonical = mise） | `mise exec -- node --check tools/run-formal-kani.mjs` | 0 | Pass。runner syntax check |
| 補助証拠（canonical = mise） | runner `/usr/bin/time -v` 記録確認 | 0 | 測定あり。max RSS は各 verdict JSON。4GiB 未満の測定値を確認 |

初期 pre-mutation lane も `pass` で 10957 ms / 299952 KiB だった。mutation/revert 後の最終 canonical lane は上表の 4265 ms / 249568 KiB であり、これを本 slice の canonical proof result とする。

## Kani regression lanes

| group | exit | lane verdict | proof / cost |
| --- | ---: | --- | --- |
| avi-strf-declaration | 0 | pass | 1/1 Pass。5873 ms / max RSS 299896 KiB。`kani-avi-strf-declaration-regression/` |
| avi-strh-declaration | 0 | pass | 1/1 Pass。10457 ms / max RSS 302784 KiB。`kani-avi-strh-declaration-regression/` |
| avi-stream-kind | 0 | pass | 1/1 Pass。23225 ms / max RSS 500144 KiB。`kani-avi-stream-kind-regression/` |
| avi-movi-scan | 0 | pass | 1/1 Pass。15527 ms / max RSS 315424 KiB。`kani-avi-movi-scan-regression/` |
| avi-movi-composition | 0 | pass | 1/1 Pass。26802 ms / max RSS 306272 KiB。`kani-avi-movi-composition-regression/` |
| avi-movi-tag | 0 | pass | 1/1 Pass。24995 ms / max RSS 299176 KiB。`kani-avi-movi-tag-regression/` |
| mp4-avi-boundary | 0 | pass | 2/2 Pass。43495 ms / max RSS 308308 KiB。`kani-mp4-avi-boundary-regression/` |
| mp4-scan | 0 | pass | 1/1 Pass。23269 ms / max RSS 311424 KiB。`kani-mp4-scan-regression/` |
| wav-chunk-scan | 0 | pass | 2/2 Pass。13614 ms / max RSS 315052 KiB。`kani-wav-chunk-scan-regression/` |
| wav-range | 0 | pass | 2/2 Pass。16341 ms / max RSS 300560 KiB。`kani-wav-range-regression/` |

regression group の合計 solver wall/elapsed は 203598 ms、新規 proof を含めた selective proof elapsed 合計は 207863 ms だった。legacy whole-call proof を含む全件 Kani lane は既存 timeout が確定しているため、この slice の必須検証でも成功条件でもない。上記 selective group を regression として実行した。

## 保証範囲と限定的 cost comparison

新しい Required proof は full `usize` length scalar と fixed symbolic `[u8; 16]` WAVEFORMATEX prefix に対して、length gate の exact complement、format tag/channels/sample rate/byte rate/block align/bits per sample の exact offset/endianness/decode、`FmtChunk` declaration field view への写像、算術安全性を証明した。fixed prefix 内に未観測 byte はない。16 byte 以降の trailing byte（`cbSize` や extension）は production fixed-prefix helper が型上観測しないため除外した exact reparameterization であり、representative sampling ではない。

この local proof は任意長 body slice composition、実 `Vec<StreamHead>`/`strl` 走査、`strf` presence/existence/copy/last-wins、missing `strf` と empty `strf` の区別、missing `strh` を含む declaration correspondence、first `auds` selection、`wb` scan/concatenation、WAVEFORMATEX codec/layout semantic validation、PCM sample decode、partial ingest upper integration、probe integration、公開 API 全体 no-panic、I/O、allocation、diagnostic formatting、MP4、writer round trip、AVI-TIME-004 を証明しない。codec/layout matrix は `sample_layout` の別責務に残る。

限定的比較（read-only引用）では、関連 legacy whole-call AVI proof は `avi_writer_output_decodes` trial 1200s cap timeout/1267s/3.9GiB、`mutated_avi_never_panics` 1200s/1266s/2.9GiB、`truncated_avi_never_panics` 900s/948s/2.9GiB で、1500s canonical では timeout/RSS unavailable であった。今回の local proof は 4265 ms/249568 KiB、既存 AVI local proof regression は video strf 5873 ms/299896 KiB、strh 10457 ms/302784 KiB、stream kind 23225 ms/500144 KiB、movi scan 15527 ms/315424 KiB、composition 26802 ms/306272 KiB、tag 24995 ms/299176 KiB。これは保証範囲が狭い local proof の再実行コスト比較であり、等価保証や完全 cost matrix ではない。

## Branch CI

ローカル evidence 作成時点では branch 未 push のため branch CI は NotRun。push 後は manual dispatch を行わず、branch head SHA に対応する CI run を read-only で確認し、結果は PR 本文と `actions.jsonl` に記録する。確認できない場合は NotRun/Unsupported と理由を記録する。main 検証は Daily CI に委譲する。

## ファイル

- machine-readable ledger: `evidence/al274-avi-strf-audio-proof/ledger.json`
- boundary 設計: `evidence/al274-avi-strf-audio-proof/boundary-analysis.md`
- action log: `evidence/al274-avi-strf-audio-proof/actions.jsonl`
- final raw log/verdict: `evidence/al274-avi-strf-audio-proof/kani/`
- mutation raw log/verdict: `evidence/al274-avi-strf-audio-proof/mutation-audio-strf-declaration-kani/`
- regression raw logs/verdicts: `evidence/al274-avi-strf-audio-proof/kani-*-regression/`

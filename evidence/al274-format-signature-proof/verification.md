# Verification — container format signature detection proof

対象 branch は `feature/274-format-signature-proof`。開始時・最終確認時ともに `origin/main` は PR #292 merge commit `0e15ee44916f6b45918e072a19a04c449169ce7d` で、作業開始前に `git merge-base --is-ancestor 0e15ee44916f6b45918e072a19a04c449169ce7d HEAD` は exit 0 だった。final verification 前に `git fetch origin main` しても `origin/main` は同じ SHA で、merge は不要だった。canonical `fmt`/`check`/`lint`/`test`/`test-formal` は Windows host path `G:/orca/workspaces/audio-lens/al274-format-signature`、Kani lane は runner 経由の Ubuntu (WSL2) で実行した。ローカル検証は変更範囲の検証であり、統合検証は merge 後の対象 SHA に対する CI 証拠で確認する。

## Canonical verification table

| 区分 | コマンド | exit | 結果 |
| --- | --- | ---: | --- |
| canonical | `mise run fmt` | 0 | Pass。mutation revert 後にも再実行 |
| canonical | `mise run check al-media` | 0 | Pass |
| canonical | `mise run lint al-media` | 0 | Pass |
| canonical | `mise run test al-media` | 0 | Pass。unit 113/113、integration/doc suite 全件 Pass。mutation revert 後にも再実行 |
| canonical | `mise run test-formal` | 0 | Pass。`formal_wav` 7/7、`formal_containers` 7/7。mutation revert 後にも再実行 |
| canonical Required proof | `AL_KANI_HARNESS_GROUP="format-signature" AL_KANI_TIMEOUT_MS=600000 AL_KANI_LOG_DIR=evidence/al274-format-signature-proof/kani mise run test-formal-kani` | 0 | lane verdict `pass`。`format_signature_detection_bounded_exact` は `VERIFICATION:- SUCCESSFUL`。12,792 ms / max RSS 431,084 KiB。raw log/verdict は `kani/` |
| canonical post-revert Required proof regression | `AL_KANI_HARNESS_GROUP="format-signature" AL_KANI_TIMEOUT_MS=600000 AL_KANI_LOG_DIR=evidence/al274-format-signature-proof/kani-format-signature-regression mise run test-formal-kani` | 0 | lane verdict `pass`。`VERIFICATION:- SUCCESSFUL`。8,885 ms / max RSS 365,012 KiB。mutation revert 後の再検証 |
| canonical mutation run（意図的失敗） | unknown RIFF form type を MP4 fallback させる一時 mutation 後、`AL_KANI_LOG_DIR=evidence/al274-format-signature-proof/kani-mutation-riff-fallback` で format-signature lane | 1 | lane verdict `fail`。`VERIFICATION:- FAILED`。失敗 assertion は production helper oracle `actual == expected` と public shell matrix `detect_format(&input[..len]) == contract_detect_format(&input[..len])`。7,150 ms / max RSS 367,324 KiB。mutation は完全 revert 済み |
| canonical hygiene | `mise exec -- prek run --all-files` | 0 | Pass。auto-fix residue なし。raw output は `prek.txt` |
| 補助証拠（canonical = mise） | `mise exec -- node --check tools/run-formal-kani.mjs` | 0 | Pass。runner syntax check |
| 補助証拠（canonical = mise） | runner `/usr/bin/time -v` 記録確認 | 0 | 測定あり。max RSS は各 verdict JSON。4GiB 未満の測定値を確認 |

初期 pre-mutation lane と post-revert regression lane はどちらも `pass` だった。12,792 ms の初期 laneは canonical proof result として保存し、8,885 ms の post-revert laneは mutation revert 後の再検証として保存する。

## Kani regression lanes

| group / subset | exit | lane verdict | proof / cost |
| --- | ---: | --- | --- |
| format-signature | 0 | pass | 1/1 Pass。8,885 ms / max RSS 365,012 KiB。`kani-format-signature-regression/` |
| wav-chunk-scan | 0 | pass | 2/2 Pass。13,526 ms / max RSS 316,112 KiB。`kani-wav-chunk-scan-regression/` |
| wav-range | 0 | pass | 2/2 Pass。16,060 ms / max RSS 301,132 KiB。`kani-wav-range-regression/` |
| mp4-avi-boundary | 0 | pass | 2/2 Pass。17,195 ms / max RSS 309,740 KiB。`kani-mp4-avi-boundary-regression/` |
| mp4-scan | 0 | pass | 1/1 Pass。19,104 ms / max RSS 312,220 KiB。`kani-mp4-scan-regression/` |
| avi-movi-tag | 0 | pass | 1/1 Pass。4,723 ms / max RSS 300,668 KiB。`kani-avi-movi-tag-regression/` |
| avi-movi-composition | 0 | pass | 1/1 Pass。5,351 ms / max RSS 307,256 KiB。`kani-avi-movi-composition-regression/` |
| avi-movi-scan | 0 | pass | 1/1 Pass。9,158 ms / max RSS 316,364 KiB。`kani-avi-movi-scan-regression/` |
| avi-stream-kind | 0 | pass | 1/1 Pass。14,452 ms / max RSS 499,760 KiB。`kani-avi-stream-kind-regression/` |
| avi-strh-declaration | 0 | pass | 1/1 Pass。5,540 ms / max RSS 303,768 KiB。`kani-avi-strh-declaration-regression/` |
| avi-strf-declaration | 0 | pass | 1/1 Pass。5,114 ms / max RSS 301,248 KiB。`kani-avi-strf-declaration-regression/` |
| avi-strf-audio-declaration | 0 | pass | 1/1 Pass。5,018 ms / max RSS 301,796 KiB。`kani-avi-strf-audio-declaration-regression/` |
| legacy subset `detect_format_signature_rules` | 0 | pass | 1/1 Pass。6,168 ms / max RSS 341,736 KiB。`kani-legacy-detect-format-signature-rules/` |

regression group の合計 solver wall/elapsed は 124,126 ms、legacy signature subset は別に 6,168 ms だった。legacy whole-call proof を含む全件 Kani lane は既存 timeout が確定しているため、この slice の必須検証でも成功条件でもない。上記 selective group と legacy signature subset を regression として実行した。

## 保証範囲と限定的 cost comparison

新しい Required proof は full `usize` length scalar と fixed symbolic `[u8; 12]` prefix に対して、length class `<8`/`8..11`/`>=12` の exact complement、exact `RIFF`/`WAVE`/`AVI `/`ftyp` byte mapping、RIFF precedence、unknown RIFF form non-fallback、`None` complement、trailing/filler byte non-dependence、算術/indexing 安全性を証明した。fixed 12 byte は現行 signature decision が観測する最大領域であり、index 12 以降の trailing byte をモデル化しないことは exact reparameterization であって representative sampling ではない。同じ harness は len 0,7,8,11,12,13 の public shell matrix と len 12/13 trailing non-dependenceを検査するが、この matrix は full length scalar proof の代替ではない。

この local proof は任意長 public input shell copy の完全合成、caller dispatch/probe/decode integration、`decode_mp4`/`decode_avi`/`decode_wav`、container semantic validation、partial ingest、corruption/mutation/truncation whole-parser safety、MP4 box/sample table、AVI chunk/movi/timing/PCM、WAV chunk/layout/PCM/Range/Writer、I/O、allocation、diagnostic formatting、公開 API 全体 no-panicを証明しない。leading `free`/`wide`/`styp`/`mdat`/`moov` の MP4 recognition拡張は現行契約変更になるため行っていない。

限定的比較（read-only引用）では、legacy `detect_format_signature_rules` は len 7/8/12 の代表境界だけを対象とし、今回の subset 再実行でも 6,168 ms / 341,736 KiB で Pass だった。新規 local proof は full `usize` length scalar + full 12-byte prefix bit domain + public shell matrixを 8,885 ms / 365,012 KiB（post-revert）で Pass した。これは保証範囲が狭い local proof の再実行コスト比較であり、等価保証や完全 cost matrix ではない。

## Branch CI

push 後に read-only で確認し、結果を記録する。`.github/workflows/ci.yml` が manual dispatch-only の既存構成なら、manual dispatch は実行せず Unsupported として記録する。

## ファイル

- machine-readable ledger: `evidence/al274-format-signature-proof/ledger.json`
- boundary 設計: `evidence/al274-format-signature-proof/boundary-analysis.md`
- action log: `evidence/al274-format-signature-proof/actions.jsonl`
- final raw log/verdict: `evidence/al274-format-signature-proof/kani/`
- post-revert proof regression raw log/verdict: `evidence/al274-format-signature-proof/kani-format-signature-regression/`
- mutation raw log/verdict: `evidence/al274-format-signature-proof/kani-mutation-riff-fallback/`
- regression raw logs/verdicts: `evidence/al274-format-signature-proof/kani-*-regression/`
- legacy signature subset raw log/verdict: `evidence/al274-format-signature-proof/kani-legacy-detect-format-signature-rules/`
- canonical command output: `evidence/al274-format-signature-proof/final-canonical.txt`

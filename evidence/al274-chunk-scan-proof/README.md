# evidence/al274-chunk-scan-proof

Issue #274 の WAV-CHUNK-003 / WAV-SCAN-004 exact proof slice の証跡である。PR #282 で Required Pass 済みの WAV-RANGE-008 は再設計せず、`wav.rs` 変更の regression として再実行した記録だけを持つ。

## 最終結果

- wav-chunk-scan canonical lane: `pass`。2/2 Required proof が `VERIFICATION:- SUCCESSFUL`。
  - `wav_chunk_boundary_step_exact`: 6922 ms, max RSS 288164 KiB
  - `wav_chunk_scan_progress_bounded_exact`: 7592 ms, max RSS 300376 KiB
- wav-range regression lane: `pass`。2/2 proof が `VERIFICATION:- SUCCESSFUL`。
- boundary mutation（`pad` 加算漏れ）: lane `fail`、boundary proof が `VERIFICATION:- FAILED`。
- scan mutation（`next_offset` 停滞）: lane `fail`、boundary/scan 両 proof が `VERIFICATION:- FAILED`。
- 両 mutation は完全 revert 済み。revert 後に `mise run test al-media`、`mise run test-formal`、最終 wav-chunk-scan lane が Pass。

## ファイル

- `boundary-analysis.md`: solver/runner より先に確定した境界設計。
- `ledger.json`: input domain、assume、coverage、接続条件、未証明領域、cost、mutation、runner 情報の machine-readable 台帳。
- `verification.md`: canonical 検証コマンドと結果。
- `actions.jsonl`: 作業アクション記録。
- `kani/`: mutation revert 後の最終 wav-chunk-scan raw log/verdict。
- `kani-range-regression/`: wav-range regression raw log/verdict。
- `mutation-boundary-kani/`・`mutation-scan-kani/`: 意図的失敗の raw log/verdict。
- `kani-initial/`: 初回実走の補助的な確認ログ。最終証拠は `kani/`。

local proof は公開 API 全体 no-panic、I/O、allocation、PCM decode、diagnostic formatting、fmt/data semantic、duplicate 判定、無制限 chunk 数の機械的 termination を証明しない。

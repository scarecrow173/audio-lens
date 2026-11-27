# AL274 container format signature detection proof evidence

Issue `scarecrow173/audio-lens#274` の narrow implementation slice で、container format signature detection の fixed 12-byte prefix と full `usize` input length に対する decision boundary を対象にした bounded exact proof の証跡である。任意長 public input shell copy の完全合成、caller dispatch/probe/decode integration、MP4/AVI/WAV parser correctness、PCM、timing、writer round-trip、public API 全体 no-panic はこの証跡の保証対象ではない。

## 主要ファイル

- `boundary-analysis.md`: input domain、exact reparameterization、接続条件、候補比較、mutation plan
- `verification.md`: canonical 検証、mutation、regression、cost、残件
- `ledger.json`: machine-readable proof ledger
- `actions.jsonl`: 実行・判断・失敗・revert の時系列記録
- `kani/`: 初期の format-signature canonical lane raw log / verdict
- `kani-format-signature-regression/`: mutation revert 後の format-signature regression lane raw log / verdict
- `kani-mutation-riff-fallback/`: unknown RIFF form を MP4 fallback させる意図的失敗 mutation lane
- `kani-legacy-detect-format-signature-rules/`: legacy signature subset regression lane
- `kani-*-regression/`: 既存 selective Kani assurance group の regression raw log / verdict

最終判定は lane verdict `pass`、proof `format_signature_detection_bounded_exact` が `VERIFICATION:- SUCCESSFUL`。mutation は unknown RIFF form type を MP4 へ fallback させる production semantic mutation であり、同 proof が `VERIFICATION:- FAILED` を返した後に完全 revert した。leading `free`/`wide`/`styp`/`mdat`/`moov` の MP4 recognition 拡張は現行契約変更になるため、この slice では行っていない。

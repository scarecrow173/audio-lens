# AL274 AVI audio `strf` WAVEFORMATEX declaration proof evidence

Issue `scarecrow173/audio-lens#274` の narrow implementation slice で、AVI audio `strf` / WAV `fmt ` に共有される WAVEFORMATEX fixed-prefix structural decode core を対象にした bounded exact proof の証跡である。AVI-STREAM-002 全体や `strf` presence/copy/ordering、codec/layout semantic validation、PCM decode、AVI-TIME-004、MP4-TABLE-005 はこの証跡の保証対象ではない。

## 主要ファイル

- `boundary-analysis.md`: input domain、exact reparameterization、接続条件、候補比較、mutation plan
- `verification.md`: canonical 検証、mutation、regression、cost、残件
- `ledger.json`: machine-readable proof ledger
- `actions.jsonl`: 実行・判断・失敗・revert の時系列記録
- `kani/`: mutation revert 後の最終 canonical lane raw log / verdict
- `mutation-audio-strf-declaration-kani/`: 意図的失敗 mutation lane raw log / verdict
- `kani-*-regression/`: 既存 selective Kani assurance group の regression raw log / verdict

最終判定は lane verdict `pass`、proof `avi_audio_strf_declaration_bounded_exact` が `VERIFICATION:- SUCCESSFUL`。mutation は sample_rate offset をずらした production semantic mutationであり、同 proof が `VERIFICATION:- FAILED` を返した後に完全 revert した。

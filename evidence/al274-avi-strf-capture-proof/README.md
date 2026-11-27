# AL274 AVI strf capture proof evidence

Issue `scarecrow173/audio-lens#274` の narrow implementation slice で、AVI `strl` 内 `strf` capture の presence/copy/ordering を対象にした bounded exact proof family の証跡である。full AVI-STREAM-002 declaration correspondence、nested walker、任意長 body slice、missing-vs-empty product semantics、first stream selection、PCM/timing、public API 全体 no-panic は保証対象外である。

## 主要ファイル

- `boundary-analysis.md`: input domain、exact decomposition、接続条件、候補比較、mutation plan
- `verification.md`: canonical 検証、mutation、regression、cost、残件
- `ledger.json`: machine-readable proof ledger
- `actions.jsonl`: 実行・判断・失敗・revert の時系列記録
- `kani/`: 初回 avi-strf-capture canonical lane raw log / verdict
- `kani-avi-strf-capture-regression/`: mutation revert 後の avi-strf-capture lane
- `mutation-strf-capture-kani/`: last-write-wins を first-write-wins へ変える意図的失敗 mutation lane
- `kani-*-regression/`: 既存 selective Kani assurance group の regression raw log / verdict

最終判定は lane verdict `pass`、3 proof が `VERIFICATION:- SUCCESSFUL`。mutation は production capture reducer を first-write-wins 化する semantic mutation であり、`avi_strf_capture_ordering_bounded_exact` が `VERIFICATION:- FAILED` を返した後に完全 revert した。

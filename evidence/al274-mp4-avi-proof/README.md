# al274 MP4/AVI boundary exact proof evidence

Issue #274 の MP4-BOX-002 / AVI-CHUNK-001 exact proof slice の証跡である。Issue 全体の完了証跡ではなく、MP4 box header boundary と AVI RIFF chunk boundary の代表 local proof、runner group、mutation detection、WAV regression を対象にする。

## 構成

- `boundary-analysis.md`: solver/runner 選択前に確定した input domain、証明性質、除外領域、接続条件、assume 分類、decomposition 比較。
- `actions.jsonl`: 実行コマンド、判断、失敗、mutation、revert、検証結果の時系列台帳。
- `verification.md`: canonical 検証コマンド、exit code、lane verdict、証跡 path、cost 比較。
- `ledger.json`: machine-readable な proof boundary、coverage、 verdict、mutation、cost、未証明領域。
- `kani/`: mutation revert 後の最終 mp4-avi-boundary canonical lane raw log / verdict。
- `kani-initial/`: assertion 明確化前の初回 successful run。最終 verdict ではない。
- `mutation-mp4-kani/`: MP4 size0 off-by-one mutation に対する意図的失敗 lane。
- `mutation-avi-kani/`: AVI pad omission mutation に対する意図的失敗 lane。
- `kani-wav-chunk-scan-regression/`: runner 変更 regression としての wav-chunk-scan lane。
- `kani-wav-range-regression/`: runner 変更 regression としての wav-range lane。

`evidence/al274-boundary/` は schema 統合未了の baseline なので、この slice では更新しない。過去の timeout/cost 証拠は read-only 引用である。

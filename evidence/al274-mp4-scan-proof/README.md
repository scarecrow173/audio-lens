# al274 MP4 scan exact proof evidence

Issue #274 の MP4-SCAN-003 exact proof slice の証跡である。Issue 全体の完了証跡ではなく、MP4 child box scan の進行性・終端判定・first invalid 停止を、宣言した small scan model で exact に検査する local proof、runner group、mutation detection、先行 exact proof regression を対象にする。

## 構成

- `boundary-analysis.md`: solver/runner 選択前に確定した input domain、証明性質、除外領域、接続条件、assume 分類、decomposition 比較。
- `actions.jsonl`: 実行コマンド、判断、失敗、mutation、revert、検証結果の時系列台帳。
- `verification.md`: canonical 検証コマンド、exit code、lane verdict、証跡 path、cost 比較。
- `ledger.json`: machine-readable な proof boundary、coverage、verdict、mutation、cost、未証明領域。
- `kani/`: mutation revert 後の最終 mp4-scan canonical lane raw log / verdict。
- `mutation-scan-kani/`: production scan next-offset mutation に対する意図的失敗 lane。
- `kani-mp4-avi-boundary-regression/`: MP4/AVI boundary group の regression lane。
- `kani-wav-chunk-scan-regression/`: WAV chunk/scan group の regression lane。
- `kani-wav-range-regression/`: WAV range group の regression lane。

`evidence/al274-boundary/` は schema 統合未了の baseline なので、この slice では更新しない。過去の timeout/cost 証拠は read-only 引用である。bounded scan model の成功を無制限 scan termination の機械証明と読み替えない。

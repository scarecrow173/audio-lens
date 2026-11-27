# al274 AVI stream existence/kind classification bounded exact proof evidence

Issue #274 の AVI stream existence/kind classification bounded exact proof slice の証跡である。Issue 全体の完了証跡ではなく、固定 3 件の declared stream type table、full `usize` stream number、full `[u8; 2]` kind に対して、stream lookup、type class、first stream selection、kind role、timeline ownership、selected-stream payload attribution を production pure helper/predicate 群へ接続して検査する local proof、runner group、mutation detection、先行 exact proof regression を対象にする。

## 構成

- `boundary-analysis.md`: solver/runner 選択前に確定した input domain、stream table length N=3、証明性質、除外領域、接続条件、assume 分類、decomposition 比較。
- `actions.jsonl`: 実行コマンド、判断、失敗、mutation、revert、検証結果の時系列台帳。
- `verification.md`: canonical 検証コマンド、exit code、lane verdict、証跡 path、cost 比較。
- `ledger.json`: machine-readable な proof boundary、coverage、verdict、mutation、cost、未証明領域。
- `kani/`: mutation revert 後の最終 avi-stream-kind canonical lane raw log / verdict。
- `kani-initial/`: mutation 前の初回 avi-stream-kind lane raw log / verdict。
- `mutation-stream-kind-kani/`: `pc` を video frame kind へ混入する mutation に対する意図的失敗 lane。
- `kani-avi-movi-scan-regression/`: AVI movi scan collection group regression。
- `kani-avi-movi-composition-regression/`: AVI movi entry composition group regression。
- `kani-avi-movi-tag-regression/`: AVI movi tag group regression。
- `kani-mp4-avi-boundary-regression/`: MP4/AVI boundary group regression。
- `kani-mp4-scan-regression/`: MP4 scan group regression。
- `kani-wav-chunk-scan-regression/`: WAV chunk/scan group regression。
- `kani-wav-range-regression/`: WAV range group regression。

`evidence/al274-boundary/` は schema 統合未了の baseline なので、この slice では更新しない。過去の timeout/cost 証拠は read-only 引用である。stream kind proof Pass を longer stream table、実 `Vec<StreamHead>`/`Vec<MoviPacket>`、declaration correspondence、scan collection、AVI-MOVI-003 全体、AVI-STREAM-002、AVI-TIME-004 の機械証明と読み替えない。

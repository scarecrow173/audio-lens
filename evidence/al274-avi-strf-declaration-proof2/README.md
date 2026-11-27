# al274 AVI video strf declaration bounded exact proof evidence

Issue #274 の AVI-STREAM-002 `strf` declaration validation残件のうち、first `vids` metadata pathにおける video `strf` BITMAPINFOHEADER size/field extraction bounded exact proof slice の証跡である。Issue 全体の完了証跡ではなく、full `usize` body length scalarと fixed `[u8; 40]` symbolic prefixに対して、40 byte length gate、width/height/bits-per-pixel/compression の exact little-endian field decode、height の signed/`unsigned_abs` metadata semantics、declaration field viewへの写像を production pure helperへ接続して検査する local proof、runner group、mutation detection、先行 exact proof regressionを対象にする。

## 構成

- `boundary-analysis.md`: solver/runner選択前に確定した input domain、length/trailing exact reparameterization、証明性質、除外領域、接続条件、assume分類、decomposition比較。
- `actions.jsonl`: 実行コマンド、判断、失敗、mutation、revert、検証結果の時系列台帳。
- `verification.md`: canonical検証コマンド、exit code、lane verdict、証跡 path、限定的cost比較。
- `ledger.json`: machine-readableな proof boundary、coverage、verdict、mutation、cost、未証明領域。
- `kani/`: mutation revert後の最終 avi-strf-declaration canonical lane raw log / verdict。
- `mutation-strf-declaration-kani/`: width offsetを1 byteずらす mutationに対する意図的失敗 lane。
- `kani-avi-strh-declaration-regression/`: AVI strh declaration group regression。
- `kani-avi-stream-kind-regression/`: AVI stream existence/kind classification group regression。
- `kani-avi-movi-scan-regression/`: AVI movi scan collection group regression。
- `kani-avi-movi-composition-regression/`: AVI movi entry composition group regression。
- `kani-avi-movi-tag-regression/`: AVI movi tag group regression。
- `kani-mp4-avi-boundary-regression/`: MP4/AVI boundary group regression。
- `kani-mp4-scan-regression/`: MP4 scan group regression。
- `kani-wav-chunk-scan-regression/`: WAV chunk/scan group regression。
- `kani-wav-range-regression/`: WAV range group regression。

`evidence/al274-avi-strf-declaration-proof/` は存在しないか再利用せず、本 sliceの新規証跡は `proof2` にのみ書く。`evidence/al274-boundary/` は schema統合未了の baselineなので更新しない。過去の timeout/cost証拠は read-only引用である。strf declaration proof Passを任意長 body slice composition、実 `Vec<StreamHead>`/`strl`走査、`strf` presence/copy/ordering、declaration correspondence、audio `strf` validation、BITMAPINFOHEADER decode suitability、AVI-STREAM-002全体、AVI-TIME-004の機械証明と読み替えない。

# al274 AVI strh declaration bounded exact proof evidence

Issue #274 の AVI-STREAM-002 `strh` declaration field extraction/validation bounded exact proof slice の証跡である。Issue 全体の完了証跡ではなく、full `usize` body length scalar と fixed `[u8; 48]` symbolic prefix に対して、length gate、`fcc_type` byte 順序保持、offset 20/24/28/32/44 の little-endian u32 decode、declaration field view への写像を production pure helper へ接続して検査する local proof、runner group、mutation detection、先行 exact proof regression を対象にする。

## 構成

- `boundary-analysis.md`: solver/runner 選択前に確定した input domain、length/trailing exact reparameterization、証明性質、除外領域、接続条件、assume 分類、decomposition 比較。
- `actions.jsonl`: 実行コマンド、判断、失敗、mutation、revert、検証結果の時系列台帳。
- `verification.md`: canonical 検証コマンド、exit code、lane verdict、証跡 path、限定的 cost 比較。
- `ledger.json`: machine-readable な proof boundary、coverage、verdict、mutation、cost、未証明領域。
- `kani/`: mutation revert 後の最終 avi-strh-declaration canonical lane raw log / verdict。
- `mutation-strh-declaration-kani/`: `scale` offset を 1 byte ずらす mutation に対する意図的失敗 lane。
- `kani-avi-stream-kind-regression/`: AVI stream existence/kind classification group regression。
- `kani-avi-movi-scan-regression/`: AVI movi scan collection group regression。
- `kani-avi-movi-composition-regression/`: AVI movi entry composition group regression。
- `kani-avi-movi-tag-regression/`: AVI movi tag group regression。
- `kani-mp4-avi-boundary-regression/`: MP4/AVI boundary group regression。
- `kani-mp4-scan-regression/`: MP4 scan group regression。
- `kani-wav-chunk-scan-regression/`: WAV chunk/scan group regression。
- `kani-wav-range-regression/`: WAV range group regression。

`evidence/al274-boundary/` は schema 統合未了の baseline なので、この slice では更新しない。過去の timeout/cost 証拠は read-only 引用である。strh declaration proof Pass を任意長 body slice composition、実 `Vec<StreamHead>`/`strl` 走査、`strf` payload validation、declaration correspondence、AVI-STREAM-002 全体、AVI-TIME-004 の機械証明と読み替えない。

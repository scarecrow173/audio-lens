# al274 AVI movi scan collection bounded exact proof evidence

Issue #274 の AVI-MOVI-003 movi scan collection bounded exact proof slice の証跡である。Issue 全体の完了証跡ではなく、AVI `movi` direct child entry 3 件の固定列について、packet 収集順、first-reject 停止、ignore 非収集、複数 reject の precedence を、full `[u8; 4]` chunk id と exact list-type view `Option<[u8; 4]>` で検査する local proof、runner group、mutation detection、先行 exact proof regression を対象にする。

## 構成

- `boundary-analysis.md`: solver/runner 選択前に確定した input domain、entry 数 N=3、証明性質、除外領域、接続条件、assume 分類、decomposition 比較。
- `actions.jsonl`: 実行コマンド、判断、失敗、mutation、revert、検証結果の時系列台帳。
- `verification.md`: canonical 検証コマンド、exit code、lane verdict、証跡 path、cost 比較。
- `ledger.json`: machine-readable な proof boundary、coverage、verdict、mutation、cost、未証明領域。
- `kani/`: mutation revert 後の最終 avi-movi-scan canonical lane raw log / verdict。
- `kani-initial/`: mutation 前の初回 avi-movi-scan lane raw log / verdict。
- `mutation-scan-kani/`: packet collect 欠落 mutation に対する意図的失敗 lane。
- `kani-avi-movi-composition-regression/`: AVI movi entry composition group の regression lane。
- `kani-avi-movi-tag-regression/`: AVI movi tag group の regression lane。
- `kani-mp4-avi-boundary-regression/`: MP4/AVI boundary group の regression lane。
- `kani-mp4-scan-regression/`: MP4 scan group の regression lane。
- `kani-wav-chunk-scan-regression/`: WAV chunk/scan group の regression lane。
- `kani-wav-range-regression/`: WAV range group の regression lane。

`evidence/al274-boundary/` は schema 統合未了の baseline なので、この slice では更新しない。過去の timeout/cost 証拠は read-only 引用である。scan collection proof Pass を unbounded finite iteration、N+1 件以上の entry 列、実 `Vec<MoviPacket>` accumulation、nested walker 全体、AVI-MOVI-003 全体、AVI-TIME-004 の機械証明と読み替えない。

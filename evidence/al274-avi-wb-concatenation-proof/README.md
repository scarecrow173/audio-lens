# AVI first-audio `wb` 連結 proof の証跡

Issue #274 のうち、固定 3 stream・3 packet、body 長 0〜2 byte における
最初の `auds` の `wb` 選択・mux 順・byte copy を対象にする。AVI
パーサー全体、PCM decode、時刻計算の証明ではない。

- [境界分析](boundary-analysis.md): 入力領域、exactness、本番との接続、予算と未証明領域。
- [検証結果](verification.md): Runtime Verification、Formal Verification、mutation、cost、branch CI。
- [台帳](ledger.json) と [作業記録](actions.jsonl): 再実行用の明細。runner の raw log と `verdict.json` は各 `kani*` / `mutation-wb-concatenation-kani` ディレクトリにある。

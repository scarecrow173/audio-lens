# AVI `strh` presence の検証証跡

AudioLens Issue #274 のうち、単一 `strl` の stream presence と短い
`strh` での停止を扱う。既存の宣言フィールド抽出、`strf` capture、
`wb` 連結を置換するものではない。

- [境界分析](boundary-analysis.md): 必要保証、production との接続、入力、
  非対象、予算と変異を記録する。
- [検証結果](verification.md): 実行コマンド、環境、Sync Commit、結果と
  測定値を記録する。
- [検証台帳](ledger.json): 入力領域、選択 group と runner の verdict を
  対応付ける。
- [判断記録](decisions.tsv): 証明境界と検証に関する判断を記録する。

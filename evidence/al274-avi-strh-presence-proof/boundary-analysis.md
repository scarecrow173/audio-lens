# AVI `strh` presence の証明境界

## 必要保証

一つの `strl` について、`strh` がなければ stream を追加しない。
正常な `strh` が一件以上あり、最後まで読み取れた場合は一件だけ追加する。
短い `strh` は先行する正常な `strh` の有無にかかわらず、その位置で
既存の `Malformed` エラーになる。空の stream を誤って追加すると後続
stream の番号がずれ、短いヘッダーを黙認すると破損入力を成功として扱う。

## 実装との接続

基点 `bd7bda931a9080b0e6233005016358f919922b11` の `avi_layout` は、
`inner?` の後に tag を分類し、`strh` の場合だけ `parse_strh` を呼ぶ。
`parse_strh` は 48 byte 未満を変更前と同じ文言で拒否し、受理時に
`StreamHead` の宣言フィールドを更新する。`has_strh` を `true` にする
のは `parse_strh` の成功後である。その後に各 entry の `strf` capture が
あり、走査完了後の `has_strh` が `streams.push` を制御する。

この分岐と presence 更新のみを一つの production step に切り出す。
Kani は同じ tag 分類・step・`parse_strh` を三回まで合成し、実際に成功で
返った presence または最初に失敗した 0-based entry index を比較する。
oracle は raw selector の最初の `Invalid` と `Valid` の有無から期待値を
直接求め、production の分類・長さ述語・更新処理を呼び出さない。
長さ述語だけの再証明では、すでに存在する `strh` が後続エラーを
隠す回帰を検出できない。

## 入力と除外

- ちょうど三つの slot を用意し、各 slot に `Other`、`Valid`、`Invalid`
  を独立に割り当てる。`u8 % 3` の全三値が到達可能なので、意味上の
  27 通りを覆う。`kani::assume` や Stub は置かない。
- `Other` は正確な `strf` または `JUNK` の空 body、`Valid` と
  `Invalid` は正確な `strh` の 48 byte と 47 byte のゼロ埋め body。
  これは代表入力への限定であり、任意の tag・長さ・payload への
  exact reparameterization ではない。全 tag 分類と全 `usize` の長さ
  述語は既存の別 proof に残す。
- 走査ループの unwind bound は 4 とし、unwinding assertions を有効に
  する。`Chunks` の構造エラーを含む全順位、任意長の `strl`、複数
  `strl` の合成、実 stream table 蓄積、全宣言対応、`movi`、timing、
  PCM、公開 API 全体の no-panic は証明しない。
- 独立した合成 RIFF の実パーサーテストで、不在時の空 stream table、
  複数の正常ヘッダーで一件だけ追加すること、正常ヘッダーの有無を
  問わず短いヘッダーが既存エラーになり後続の構造エラーに先行する
  ことを確認する。エラー文言と `strf` capture は production で維持する。

## 実行予算と変異

新規の選択 harness の予算は 120 秒、GNU time の最大 RSS 4 GiB とする。
runner の timeout 600 秒は診断用の停止上限であり、予算とは別である。
再実行時には関連する `avi-strh-declaration`、`avi-strf-capture`、
`avi-wb-concatenation` も選択できる。wall time と per-harness elapsed、
最大 RSS、実行環境と exit を分けて報告する。

`parse_strh` が失敗した場合にエラーを返さず直前の presence を返す変異を
入れる。oracle の比較が失敗することを確認し、元の blob と一致するまで
復元して同じ group を再実行する。

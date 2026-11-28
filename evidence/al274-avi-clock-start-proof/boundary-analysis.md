# AVI-TIME-004: ストリーム開始 tick の検証境界

## 必要保証と実行予算

`dwStart` と `dwScale` の全 `u32` 入力対について、数学上の積が
`i64::MAX` 以下なら正確な開始 tick を返し、超えるなら拒否する。
積を狭い型で先に計算すると初回 packet の位置がずれ、表現できない積を
受理すると不正な時刻を後続処理に渡すため、算術は入力全域の保証を求める。

1 harness を **600 秒以内・ピーク RSS 4 GiB 未満**で実行する。
`AL_KANI_HARNESS_GROUP=avi-clock-start AL_KANI_TIMEOUT_MS=600000 mise run test-formal-kani`
で全域を最初に試す。runner の GNU time による最大 RSS、harness の
経過時間、proof 結果を照合する。予算内に証明できなければ、入力を
縮めたり、Required の Pass にしたりせず、実測と具体例を残す。

## 局所証明と実装への接続

`packet_ticks` の初期化から `avi_stream_start_tick(start: u32, scale: u32)
-> Option<i64>` を抽出し、既存の `i64` への変換と checked multiplication を
維持する。Kani はこの製品 helper を呼ぶ。oracle は両入力を `u64` に
幅拡張してから掛ける。最大の積 `(2^32 - 1)^2` も `u64` に収まるので、
符号なしの積を `i64::MAX as u64` と比較して受理・拒否を判定できる。
`assume`、入力分割、反復 bound、stub、製品コードの `cfg(kani)` 置換は
使わない。ゼロを含め、実際の AVI で出現する値に制限しない。

実装への接続は構築済みの `StreamHead` と `MoviPacket` を渡す runtime
テストで確認する。非ゼロの開始値から最初の packet に正確な tick が付き、
packet がない非音声・非映像 stream でも表現不能な開始 tick は従来どおり
`Malformed` になる。これらは具体例の検証であり、stream table 全域や
parser の機械証明ではない。全 stream の初期化は従来どおり packet 処理前に行う。

AVI-TIME-004 の残り（remainder、packet 増分・累積、stream 間の分離）、
PCM、RIFF/AVI parser 全体、公開 API の no-panic は証明対象外とする。
全域 proof が成功した場合は、`i64::MAX` を拒否する境界変異で反例が
出ることを確認し、変異前の実装に戻して同じ group を再検証する。

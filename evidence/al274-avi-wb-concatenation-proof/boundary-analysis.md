# AVI first-audio `wb` 連結の検証境界

## Property と Consumer

Issue #274 のうち、最初の `auds` に属する exact `wb` の body だけを
`movi` の mux 順に連結し、長さ・内容・byte 順を保存する局所契約を対象にする。
別 stream の混入、パケット逆順、byte の欠落は `decode_waveformatex` に渡す
PCM を変え、`AviMedia.audio` と backend の `IngestResult.audio` に現れる。
不正な PCM で decode が失敗すれば、availability と `IngestResult.provenance`
の stream outcome も変わる。PR #288 は entry の収集順、PR #289 は分類と
first selection、PR #295/#296 は `StreamHead.format` の capture を扱うが、
いずれも複数 packet の選択と byte 連結の合成は扱わない。

## 定理と入力領域

- stream type table は固定 3 件、`movi` packet は mux 順の固定 3 件。
  先頭・中央・末尾の first `auds`、audio 不在、選択間への非選択 packet の
  挿入、空と非空の混在を同一の状態空間で扱う。
- 各 body は長さ `{0,1,2}`。独立な full `u8` を `% 3` で写像し、独立な
  `[u8; 2]` の prefix を実 `&[u8]` とする。実在する全 byte に全 bit pattern を
  許し、prefix 外は symbolic filler。長さ 0 は空 append、1 は単 byte、
  2 は packet 内 byte 順、複数 packet は packet 間の byte 順の検証に必要となる。
- 既存 `AviStreamTypeView` が観測するのは各 `fcc_type` のみであり、table の
  順序を変えず `StreamHead` を type-only table へ射影できる。PR #289 は
  全 `[u8; 4]` type、全 `usize` stream number、全 `[u8; 2]` kind に対して
  first selection と selected-audio predicate の exactness を検証済み。
  first selection は type class のみに依存し、append の分岐は選択 index との
  等値、in-range の `auds`、exact `wb` の有無のみに依存する。したがって
  `auds`/`vids`/`xxxx`、index `0`/`1`/`2`/`3`（全 out-of-range の同値類）、
  kind `wb`/`zz`（全 non-`wb` の同値類）へ置き換えても、選択と append
  される body 列は同じになる。`db`/`dc` も音声側の述語では `zz` と同値。
  この置換は時間計算や映像の分類には適用しない。
- 各 class/index/kind は独立な full `u8` から `% 3`、`% 4`、`% 2` で
  全射に符号化する。各組合せが表現可能で、body は変換しないため、
  bounded domain 内は代表サンプリングではなく網羅となる。`kani::assume`、
  未証明の補題、stub、`cfg(kani)` による製品置換を用いない。固定件数と
  最大長は定理の明示的な境界であり、solver 実行後の縮小ではない。

独立 oracle は type table を raw `auds` と直接照合し、最小 index を
得る。各 packet の index と raw kind で選択を判定し、非選択の寄与長を
0 とした prefix offset を求める。実出力長を寄与長の和と比較し、各選択
body に実在する byte を対応する出力位置と照合する。各寄与は 0〜2、
合計は 0〜6 で、検査は `usize` で行う。これにより選択、順序、長さ、内容、
空 append、非選択 byte の非干渉、audio 不在・選択 payload 不在での空出力を
検査する。Kani の算術・境界・panic・unwinding 検査は有効なままとする。

## 本番コードとの接続

`decode_layout` の mux-order loop を、stream view、`&[MoviPacket]`、
選択 index を受ける `avi_selected_audio_pcm` に移す。この helper が出力
`Vec<u8>` を所有し、既存 predicate が真のときだけ `extend_from_slice` する。
proof は同じ first-selection helper を呼び、固定配列から作る packet slice を
同じ連結 helper に渡す。audio 不在は本番のように連結を開始せず、proof では
空の結果として比較する。入力 packet を産出する nested walker や、戻り値を
診断・decode に変換する `decode_layout` 全体は機械証明しない。

`decode_layout` は `audio_requested=false` を引き続き bypass し、最初の
`auds` がある場合だけ連結し、戻り値に別の byte を足さず `decode_waveformatex`
に渡す。空の連結は既存の `audio stream carries no wb packets` と同じ
degradation のままであり、空 `wb` と欠落 `wb` の製品上の区別は導入しない。
映像・timing・error precedence の処理や公開 API は変えない。本番接続は
既存の複数音声 stream の fixture に non-`wb` と空 `wb` を加え、PCM
期待値を変えずに検査する。空 `wb` の unavailable/bypass と、既存の
`formal_containers` の missing/partial ingest・round-trip・異常入力を補完に使う。

## 分解と実行予算

| 候補 | 採否と根拠 |
| --- | --- |
| 固定列・bounded byte copy の single proof | 採用。選択・走査順・実 `Vec` への copy を同じ helper の呼び出しで検査し、中間接続の仮定を増やさない。 |
| 選択順と append/copy を別 proof に分割 | 予算内で合成できない場合の候補。分割時も全入力の対応を立証する必要がある。 |
| raw type/index/kind の全 bit を直接合成 | 既証明の同値類より solver が扱う bit が増える。同値性が崩れる場合の候補。 |
| tag-only / 一回の append / concrete test のみ | 複数 packet の順序・混入・全 byte の網羅を満たさない。concrete test は本番接続に使う。 |
| `decode_layout` 直接、実 dynamic table と packet 収集の合成 | walker、`Vec<StreamHead>`/`Vec<MoviPacket>`、declaration、request flag、診断・codec を同時に扱う別境界。 |
| 任意長 body・長い packet 列・一般の `Vec` growth | 出力が全 byte を観測するため、固定 prefix で任意長を代表できない。 |
| PCM semantic validation、first video、時刻計算、MP4 | byte の連結とは別の契約。codec/layout、時間算術、別 container の表は本 proof に入れない。 |

境界確定後、`avi.rs` の `#[cfg(kani)]` module に
`avi_audio_wb_concatenation_bounded_exact` を置く。正規 group は
`avi-wb-concatenation`、runner は既存の sequential 実行、既定 solver、
unwinding checks、GNU time を維持する。設計予算は 1 proof 120 秒以内、
4 GiB 未満。canonical timeout は 600 秒、計測値の判定とは別とする。
走査順を逆転する一時 mutation で length は同じでも byte oracle が失敗する
ことを確認し、復元して再実行する。既存 group は回帰、legacy 全件は対象外。

初回は oracle の可変長 loop を symex が実際の上限 2 で止められず、
設計予算の 120 秒を超過した。中断時に残った raw log では同じ loop を
858 回まで展開しており、反例は得ていない。定義域は変えず、oracle を
静的な 3 packet × 2 byte の検査に変更した。入力の slice は三つの定数長
から構成し、oracle は生成元の length と storage から出力位置を求める。
本番 loop、`Vec`、byte の入力領域、検査する性質、unwinding 検査は変えない。

## 未証明領域と RF

4 件以上の stream/packet、3 byte 以上の body、一般の allocation/
`Vec` growth、任意長 body slice の合成、nested `Chunks`/LIST walker、
実 table/packet accumulation、境界・padding、full AVI-STREAM-002
declaration correspondence、missing `strh` filter/error precedence、
missing-vs-empty の製品意味論、`strf` capture/materialize との合成、
first `auds`/`vids` の dynamic composition、video frame extraction、
AVI-TIME-004、AVI-PCM、MP4-TABLE-005、MP4-PCM、WAV-PCM/Writer、
format signature、I/O、診断整形、partial ingest 全体、unbounded scan、
whole-parser corruption、公開 API 全体の no-panic は証明しない。
legacy retire、全体 mutation/cost matrix、baseline ledger 統合も後続に残す。
選択は AVI 固有で、copy は標準 `Vec` 操作をそのまま使うため RF 還元候補はない。

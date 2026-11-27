# AVI stream existence/kind classification bounded exact proof boundary analysis

対象は Issue #274 の AVI-MOVI-003 残件のうち、post-scan の packet tag を宣言済み stream table へ対応づけ、kind byte を downstream の audio payload / video frame / timeline ownership へ分類する small pure boundary である。本書は solver 実験、runner 変更、production 変更に先立つ境界設計であり、AVI-MOVI-003 全体、AVI-STREAM-002 の `strh`/`strf` declaration validation 全体、AVI-TIME-004、MP4-TABLE-005、PCM 系を完了させるものではない。PR #282〜#288 の WAV/MP4/AVI boundary/tag/entry/scan proof は再設計せず、runner 変更の regression としてだけ再実行する。

## 1. Property の選定

production parser は `movi` scan が収集した packet tag の `stream_no` と `kind` を、少なくとも 5 箇所で downstream 意味へ写像している。`packet_ticks` は `streams.get(p.stream_no)` で実在 stream を引き、exact `vids` / exact `auds` だけに timeline slot を与える。first `auds` / first `vids` は `streams` の hdrl 順で最初に一致した index である。audio PCM concatenation は selected first `auds` の exact `wb` packet だけを対象にする。video frame count と frame extraction は selected first `vids` の exact `db`/`dc` packet だけを対象にする。unknown kind suffix は reject せず opaque kind として保持し、`pc` 等の non-frame `vids` packet は frame ではないが timeline slot を消費する。nonexistent stream packet と other stream type packet は timeline slot と audio/video payload の対象外であるが、存在だけで container を malformed にしない。

これらは同じ stream existence/kind meaning classification の規則であり、現状は hand-written な `matches!` と equality が分散している。本 slice は、その規則を production pure helper/predicate 群へ集約し、固定小数 stream type table と 1 packet tag について bounded exact proof で閉じる。対象は scan collection 後の packet-to-stream classification であり、scan collection そのもの、`strh`/`strf` byte layout、strl の並び、欠けた `strh` を含む declaration correspondence、format payload validation、timing accumulation、PCM/video decode ではない。

この boundary を選ぶ理由は、AVI-MOVI-003 の残件のうち iterator/body/`Vec` 合成や u128 timing division、PCM conversion matrix、MP4 sample table に依存せず、独立した scalar/fixed-table pure boundary として健全に閉じられる最小単位だからである。unbounded iteration termination、N+1 件以上の entry/stream 列、実 `Vec<MoviPacket>` accumulation、body slice/lifetime composition、nested walker 全体は scan collection の残りだが、iterator/error/slice/allocation semantics を伴う一段重い integration boundary である。AVI-TIME-004 は u128 除算・remainder・multi-stream accumulation、PCM 系は codec/format conversion matrix、MP4-TABLE-005 は multi-table accumulation/cross-table consistency を持つ。signature 再構成は既存 proof が Pass しており marginal gain が小さい。full cost matrix、全体 mutation matrix、legacy retire は新規 proof を追加した後の別 slice に残す。

## 2. exact input domain

proof の入力は次の 3 つだけである。

1. 固定 3 件の declared stream type table `[[u8; 4]; 3]`。各 byte は独立に full 8 bit を取るため、各 stream type は全 2^32 bit pattern を取る。`auds`/`vids`/other、ASCII/non-ASCII、NUL、writer fixture によくある値だけへ限定しない。
2. 1 件の packet stream number `usize`。machine integer domain 全体を symbolic に扱う。production tag proof が `stream_no <= 99` を保証していても、この helper の input を 0..=99 へ narrowing しない。
3. 1 件の packet kind `[u8; 2]`。全 2^16 bit pattern を symbolic に扱う。`wb`/`db`/`dc`/`pc`/unknown、non-ASCII byte を sampling しない。

stream table length N=3 は bounded model の明示的 scope であり、`kani::assume` による input narrowing ではない。N=3 を採る根拠は、first stream selection の leading/middle/trailing position、mixed `auds`/`vids`/other、最初の一致より前の non-match skip、in-range/out-of-range lookup を 1 つの finite composition で同時に覆える最小値だからである。N=2 では middle position が存在せず、other を挟んだ複数 class の ordering と trailing position を同じ表で構成しにくい。N=4 以上はこの sub-boundary の分類種類を増やさず、状態数だけを増やす。より長い stream table、任意長 `Vec<StreamHead>`、strl の並び、欠けた `strh` を含む declaration correspondence は未証明領域に残す。

proof は production `StreamHead` の `scale`/`rate`/`start`/`length`/`sample_size`/`format` を入力に含めない。classification core は `fcc_type` だけを観測する。したがって、post-parse `streams` Vec と同じ hdrl 順を保つ type-only view は exact reparameterization であり、unobserved field を concrete に固定する input narrowing ではない。`format` payload、PCM/video decode、timing arithmetic は別 property の入力である。

## 3. 証明する性質

proof harness 名は `avi_stream_kind_classification_bounded_exact`、runner group 名は `avi-stream-kind` とする。固定 3 件の stream type table、full `usize` stream number、full `[u8; 2]` kind に対し、production pure helper/predicate 群の total correctness を 1 proof で検査する。検査する性質は少なくとも次の通りである。

- stream lookup が fixed table の in-range index `stream_no < 3` でだけ存在し、out-of-range では存在しないこと。
- 存在する lookup が stream type byte を変えずに返すこと。
- exact `auds`、exact `vids`、その他の stream type class が disjoint かつ exhaustive な complement であること。
- first `auds` / first `vids` selection が table 順（production `streams` Vec の hdrl 順と同じ順）の最初の一致 index または absence と正確に一致し、それより前の non-match を飛ばさないこと。
- exact `wb` だけが audio payload kind、exact `db`/`dc` だけが video frame kind、それ以外の全 kind が non-payload kind であること。
- 実在する `auds` stream の exact `wb` packet だけが audio payload、実在する `vids` stream の exact `db`/`dc` packet だけが video frame、それ以外の全 stream/type/kind 組合せが non-payload であること。
- 実在する `auds`/`vids` packet は kind に関係なく timeline slot owner、missing stream packet と other stream type packet は owner でないこと。これにより `pc` は video frame ではないが `vids` timeline slot を消費し、unknown kind は opaque に保持される現行契約を変えない。
- selected first stream への payload 帰属が stream number equality と payload role の conjunction と一致すること。first `auds` 以外の `wb` は audio PCM concatenation 対象外、first `vids` 以外の `db`/`dc` は video frame count/extraction 対象外である。
- これらの計算で overflow、underflow、panic がないこと。

この proof は fixed-length stream table と 1 packet tag について finite machine integer/byte domain 全体の total correctness proof であり、representative sampling ではない。fixed-length model の成功を、任意長 stream table、実 `Vec<StreamHead>`/`Vec<MoviPacket>`、missing `strh` を含む declaration correspondence、scan collection、AVI-MOVI-003 全体、AVI-STREAM-002 全体、AVI-TIME-004 の機械証明と読み替えない。

## 4. production 接続条件

proof model は test-side の分類複製ではなく、production pure helper/predicate 群を呼ぶ。production 側では少なくとも次の use site が同じ helper/predicate 群を使い、kind meaning や stream type equality を重複実装しない。

- `packet_ticks` の stream lookup/type class/timeline ownership 判定。
- `stream_timings` の audio/video clock 分類。
- first `auds` / first `vids` selection。
- audio PCM concatenation の selected-stream + `wb` filter。
- video frame count と video frame extractionの selected-stream + `db`/`dc` filter。

production `StreamHead` は timing/format 責務のため `scale`/`rate`/`start`/`length`/`sample_size`/`format` を持つ。classification proof が `Vec<u8>` payload や allocation を引き込まないよう、helper は最小の lightweight type view に対して書く。具体設計は、`StreamHead` と proof 用の `[u8; 4]` cell の両方が同じ `fcc_type` view interface を提供し、lookup/selection/classification helper がその view だけを観測する形とする。これは製品実装を solver に合わせて複雑化するためではなく、classification core が `fcc_type` 以外を観測しないという既存の責務境界を型で明示するためである。helper は `format`、body slice、payload bytes、allocation、I/O、diagnostic formatting を扱わない。

公開 API、error 種別・文言、error precedence、正常/異常系の振る舞い、f64 精度、対応形式、partial ingest/capability semantics、stream/timing/video/audio 解釈を変えない。unknown kind の opaque 保持、`pc` の timeline slot/non-frame 区別、nonexistent stream packet の非 fatal 扱い、first stream selection 順を変えない。kind validity matrix を新たな拒否規則として製品へ追加しない。

## 5. assume と coverage

input narrowing は禁止である。stream type byte、kind byte、stream number、table index を制限する `kani::assume` は置かない。fixed table length N=3 は bounded model scope であり assume ではない。loop を含む helper では unwind depth が fixed table scan を完全に覆うことを確認し、unwind を下げて通すことは unwinding-check weakening として禁止する。unwinding assertion を無効化しない。

coverage は少なくとも次の complement を落とさない。

- `auds`/`vids`/other。
- in-range/out-of-range lookup。
- `wb`/`db`/`dc`/`pc`/unknown kind。
- audio/video/non-payload kind role。
- timeline owner/non-owner。
- selected/non-selected stream。
- first stream が leading/middle/trailing/absence のいずれかであること。

expected oracle は production helper を呼ばず、raw stream type bytes、stream number、kind bytes から独立に分類する。proof が production helper 同士の一致だけを検査して製品意味とのずれを検出できない状態にしない。

## 6. 検証しない領域

`Chunks` iterator、outer `hdrl`/`movi` scan、nested LIST/OpenDML、scan collection、entry ordering、実 `Vec` allocation/push、body slice/lifetime、packet body payload、`strh`/`strf` byte layout、strl の並びと欠けた `strh` を含む declaration correspondence、WAVEFORMATEX/PCM decode、video `strf`/DIB decode、timing accumulation、remainder、partial ingest、writer/corruption whole-call proof、I/O、allocation、diagnostic formatting、公開 API 全体 no-panic、MP4、signature 再構成はこの local proof の外に置く。これらを未証明領域として ledger に書き、local proof Pass を上位保証へ転用しない。

## 7. decomposition 候補の比較

- **fixed-table stream existence/kind classification proof（採用）**: 固定 3 件の symbolic declared stream type table と 1 件の symbolic packet tag に対し、production pure helper/predicate を合成して lookup existence、type class、first-stream selection、kind role、payload role、timeline ownership、selected-stream attribution を 1 proof で検査する。`Vec` accumulation、byte body、iterator、allocation、I/O を持たない。今回の最小 pure/bounded boundary に一致し、mutation を入れたとき classification oracle が直接失敗する。
- **scalar predicate-only proof（不採用）**: `fcc_type` と kind だけを入力にし、lookup existence と first-stream selection を対象外にする案。最小だが stream existence を閉じず、ユーザー指定の優先候補を満たさない。
- **lookup/selection/payload role の小 proof family（不採用）**: solver cost または oracle 可読性が悪い場合に lookup/selection と packet kind role を 2 proof に分ける案。今回は入力次元が小さく、1 proof で exact complement を保持できる見込みが高い。分割は必要になっても input narrowing に使わない。
- **scan collection + stream classification 合成 proof（不採用）**: PR #288 の scan collection model に stream table を合成する案。収集意味と stream classification は独立 property で、結合すると入力次元と失敗解析だけが増える。
- **direct avi_layout / real Vec composition proof（不採用）**: byte slice、`Chunks` iterator、`Vec<StreamHead>`/`Vec<MoviPacket>`、body slice/lifetime、error precedence を含む integration proof。今回の最小 pure/bounded boundary を超える。
- **AVI-STREAM-002 declaration validation proof（不採用）**: `strh`/`strf` byte layout、scale/rate/start/sample_size/format payload validation を対象にする案。Required だが別 property であり、今回の stream existence/kind classification とは独立に設計する。
- **AVI-TIME-004 accumulation proof（不採用）**: `tick_advance`、remainder、複数 stream/packet clock を含み、u128 除算と multi-stream accumulation を伴う別の heavy Required propertyである。

採用案は KISS/YAGNI/True to the Code で決める。`cfg(kani)` 置換、fmt/alloc stub、solver 総当たり、timeout 延長だけの解決は禁止する。proof が通らなかった後で table length や input domain を縮める場合は weakening として扱い、Pass とせず Coordinator に ask する。

## 8. runner と予算

runner は `AL_KANI_HARNESS_GROUP="avi-stream-kind"` で `avi_stream_kind_classification_bounded_exact` を選択する。registry は 30 harness（WAV 13 + MP4 scan 1 + MP4/AVI boundary 2 + AVI movi tag 1 + AVI movi composition 1 + AVI movi scan 1 + AVI stream kind 1 + legacy containers 10）となる見込みである。既存 verdict フィールド、既定全件挙動、fail-closed、WSL mirror、target 配置、timeout 意味論、group/subset の排他意味論を変えない。全件実行はこの slice の必須検証にしない。

各 proof の設計予算は 10 分以内・4GiB 未満、avi-stream-kind 選択実行の合計実測は 10〜20 分を目標とする。timeout、exit 0、CI green を Proof Pass にしない。Required Pass が得られなければ proof/contract を弱めず、draft PR / Required NotRun として報告する。

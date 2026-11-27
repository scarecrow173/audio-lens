# AVI `strh` declaration field extraction/validation bounded exact proof boundary analysis

対象は Issue #274 の AVI-STREAM-002 残件のうち、`strh` chunk body の byte 列が `StreamHead` の declaration field へどう写像されるか、48 byte 未満をどう拒否するか、little-endian field offset が正確かという small pure boundary である。本書は solver 実験、runner 変更、production 変更に先立つ境界設計であり、`strf` payload validation を含む AVI-STREAM-002 declaration validation 全体、AVI-TIME-004、AVI-PCM、MP4-TABLE-005、MP4-PCM を完了させるものではない。PR #282〜#289 の WAV/MP4/AVI boundary/scan/tag/entry/stream kind proof は再設計せず、runner 変更の regression としてだけ再実行する。

## 1. Property の選定

production parser の `avi_layout` は `hdrl/strl` list の inner chunk を走査し、`strh` ごとに `parse_strh` を呼ぶ。現行 `parse_strh` は `body.len() < 48` を `MediaError::malformed("strh chunk shorter than 48 bytes")` で拒否し、受理時は `fcc_type` に `body[..4]` を byte 順序そのまま写し、`scale`/`rate`/`start`/`length`/`sample_size` を offset 20/24/28/32/44 の little-endian u32 として読む。48 byte 以降の trailing byte と、offset 4..20・36..44 の未観測 field は downstream の結果へ影響しない。

downstream では `video_info` が `scale`/`rate`/`length` から fps/frames を導き、`tick_advance`/`stream_timings` が `scale`/`sample_size`/`start`/`rate` を timing 計算に使う。したがって declaration bytes からこれらの field への写像が壊れると、fps、frame count、stream clock、PCM sample 進行がずれる。ただし本 slice が証明するのは declaration bytes から `StreamHead` field への extraction/validation であり、downstream の timing/codec 意味ではない。

この boundary を選ぶ理由は、AVI-STREAM-002 残件のうち iterator/body/`Vec` 合成、u128 timing division、codec/format conversion matrix、MP4 sample table に依存せず、独立した fixed-layout scalar pure boundary として健全に閉じられる最小単位だからである。`strf` validation は WAVEFORMATEX/BITMAPINFOHEADER の codec 固有 payload matrix を伴う一段重い境界であり、AVI-TIME-004 は u128 除算・remainder・multi-stream accumulation、PCM 系は codec/format conversion matrix、MP4-TABLE-005 は multi-table accumulation/cross-table consistency を持つ。signature 再構成は既存 proof が Pass しており marginal gain が小さい。full matrix、全体 mutation matrix、legacy retire は新規 proof を追加した後の別 slice に残す。

## 2. exact input domain

proof harness 名は `avi_strh_declaration_bounded_exact`、runner group 名は `avi-strh-declaration` とする。proof の入力は次の 2 軸だけである。

1. `body_len: usize`。machine integer domain 全体を symbolic に扱う。`body_len < 48` の reject class と `body_len >= 48` の accept class を exact complement として検査する。length を制限する `kani::assume` は置かない。
2. `prefix: [u8; 48]`。各 byte は独立に full 8 bit を取り、受理された body の先頭 48 byte が取りうる全 2^384 bit pattern を覆う。`fcc_type`、offset 20/24/28/32/44 の field byte、未観測 byte を writer fixture や ASCII/数値の代表値へ限定しない。

この分離は exact reparameterization である。production `parse_strh` は length validation と field extraction をこの順に合成し、field extraction は受理時の先頭 48 byte だけを観測する。実 body の trailing byte、offset 4..20・36..44 の未観測 field、body slice の lifetime、outer `strl` の並びは field extraction core の入力ではない。任意の実 body は、その length と、受理時なら先頭 48 byte によってこの boundary の振る舞いが決まる。逆に任意の `body_len >= 48` と任意の 48 byte prefix の組合せは、その prefix を先頭に持つ実 body として構成できる。したがって length scalar と fixed prefix の Cartesian product は、accept/reject validation と field extraction の入力領域を漏れなく表す。

48 byte 以降の trailing payload をモデル化しない根拠は、production helper が fixed `[u8; 48]` だけを受け取り、型上 trailing byte を観測できないことである。offset 4..20・36..44 の byte は fixed prefix 内に存在するが、proof assertion が出力を観測 offset の byteだけで規定するため、production 実装がこれらの未観測 byteへ依存すれば反例になる。これは trailing/unobserved byte を concrete に固定する input narrowing ではない。

length validation は byte に依存せず、field extractionは length に依存しない。したがって 2 軸を独立に symbolic化しても合成意味の coverage は落ちない。body slice から fixed prefix を作る shell の slice/copy composition はこの proof の対象外であり、任意長 body slice composition の機械証明とは読み替えない。

## 3. 証明する性質

proof は symbolic `body_len` と symbolic fixed prefix に対し、production pure helper の total correctnessを 1 proofで検査する。検査する性質は少なくとも次の通りである。

- `body_len < 48` の全入力が reject class、`body_len >= 48` の全入力が accept classになり、reject/accept が exact complement であること。
- 受理時に `fcc_type` が `prefix[0..4]` の byte 順序を変えず保持されること。
- `scale`/`rate`/`start`/`length`/`sample_size` がそれぞれ offset 20/24/28/32/44 の little-endian u32 decode に正確に一致すること。
- byte-order 反転、offset ずれ、field の drop/mis-map、field 間の入れ替わりがないこと。
- declaration field view が `fcc_type` と 5 つの u32 field を過不足なく対応付けること。
- これらの計算で overflow、underflow、panic がないこと。

この proof は fixed `strh` header model と full machine integer length domainについて finite machine integer/byte domain 全体の total correctness proof であり、representative sampling ではない。fixed-length model の成功を、任意長 body slice、実 `Vec<StreamHead>`/`strl` 走査、`strf` payload、AVI-STREAM-002 全体の機械証明と読み替えない。

## 4. production 接続条件

proof model は test-side の field 抽出複製ではなく production pure helperを呼ぶ。最終設計は次の最小責務へ分離する。

- production length predicate が `body_len >= 48` の accept/reject 規則を返す。
- production fixed-prefix extraction helper が `[u8; 48]` から `fcc_type`、`scale`、`rate`、`start`、`length`、`sample_size` のみを持つ declaration field view を返す。
- production `parse_strh` は `avi_layout` の `strl` loop から呼ばれ、length predicate を通らなかった body を現行どおり同じ error文言で拒否し、受理時は先頭 48 byte を fixed prefix へ写して同じ extraction helperを呼び、返った field viewを `StreamHead` へ写す。

この分離は製品契約を変えない。`StreamHead.format` の `Vec` や `has_strh` は extraction helper に関与させない。`avi_layout` 側は `strf` body の `Vec` 複製と `has_strh` 判定を現行どおり shell に残し、offset 定数、endian、length gate、`StreamHead` への写像を複製しない。field view は solver に合わせた代替実装ではなく、production が読む 6 field だけを宣言 bytes の解釈責務として表す最小の data shape である。

公開 API、error 種別・文言（`"strh chunk shorter than 48 bytes"` を含む）、error precedence、正常/異常系の振る舞い、f64 精度、対応形式、partial ingest/capability semantics、stream/timing/video/audio 解釈を変えない。48 byte 未満拒否、48 byte 以降 trailing byte と未観測 fieldの無視を変えない。48 byte 未満の受理、trailing/unobserved field を理由にした新たな拒否、`fcc_type` kind validity matrix の追加は行わない。

## 5. assume と coverage

input narrowing は禁止である。`body_len`、prefix byte、`fcc_type` byte を制限する `kani::assume` は置かない。fixed prefix length 48 は AVI `strh` declaration の固定レイアウトであり、bounded model の明示 scope であって input narrowing ではない。length class の complement は full `usize` 上で証明する。`fcc_type` の `auds`/`vids`/other への意味分類は PR #289 の stream existence/kind classification boundaryが担う別 propertyであり、この proof では byte 順序保持だけを検査する。未証明の数学的事実を assume しない。loop を含まない設計を優先し、unwinding assertion を無効化しない。

coverage は少なくとも次を落とさない。

- `body_len < 48` / `body_len == 48` / `body_len > 48` を含む full length domain の accept/reject complement。
- `fcc_type` の全 byte pattern。
- offset 20/24/28/32/44 における全 byte pattern と little-endian decode。
- offset 4..20・36..44 の未観測 byte が出力へ寄与しないこと。
- 5 field と `fcc_type` の写像 completeness。

expected oracle は production extraction helperを呼ばず、raw prefix byteから独立に `u32::from_le_bytes` と byte array equalityで期待値を作る。proofが production helper同士の一致だけを検査して製品意味とのずれを検出できない状態にしない。

## 6. 検証しない領域

次の領域はこの local proof の外に置き、ledger の未証明領域に記録する。

- `Chunks` iterator、`hdrl`/`strl` LIST 走査、entry ordering、error precedence、`MediaError` 文言生成 shell。
- `has_strh` による stream 収集判定、missing `strh`、複数 `strh`/`strf` の並びを含む declaration correspondence。
- `strf` parsing、WAVEFORMATEX/BITMAPINFOHEADER validation、`StreamHead.format` の `Vec`、codec/format payload matrix。
- 任意長 body slice/lifetime composition、body から fixed prefix への slice/copy shell、実 `Vec<StreamHead>` accumulation。
- timing accumulation（`tick_advance`/`stream_timings`/`start_tick`）、`packet_ticks`、remainder、u128 除算。
- PCM/video decode、partial ingest、writer/corruption whole-call proof、I/O、allocation、diagnostic formatting、公開 API 全体 no-panic。
- MP4、signature 再構成。

これらを local proof Pass から上位保証へ転用しない。

## 7. decomposition 候補の比較

- **strh declaration field extraction/validation proof（採用）**: full `usize` length scalarと symbolic fixed `[u8; 48]` prefixに対し、production length predicateと production fixed-prefix extraction helperを合成して length gate、`fcc_type` byte順序保持、5つの LE u32 fieldの exact offset decode、field view写像、算術安全性を 1 proofで検査する。`Vec` accumulation、`strl` 走査、body slice/lifetime、I/O を持たない。今回の最小 pure/bounded boundaryに一致し、offset/endianness/length mutation を oracle が直接検出できる。harness 名は `avi_strh_declaration_bounded_exact`、group 名は `avi-strh-declaration` とする。
- **length-gate-only proof（不採用）**: `body.len() < 48` の拒否/受理だけを検査する案。最小だが field offset、endianness、写像を閉じず、declaration semantics の中核を残す。後続 boundary が必要になり、この slice の目的を満たさない。
- **field-decode-only proof（不採用）**: 十分長 body の offset/endianness/写像だけを検査する案。short rejectionを閉じず、length gateとの exact complementを別 proofへ委ねる。2 proof familyでも total correctnessを保てるが、入力次元が小さく 1 proofで実用予算に入る見込みが高いため分割しない。
- **valid/invalid length family（不採用）**: reject classと accept classを 2 proofに分ける案。total correctnessと exact complementを失わない範囲では可能だが、単一 proofが軽いため registryと evidenceを増やす利益がない。proof分割を input narrowingに使わない。
- **combined strh+strf declaration proof（不採用）**: `strh` と `strf` を 1つの declaration proofに合成する案。`strf` は WAVEFORMATEX/BITMAPINFOHEADER の codec固有 payload validationを伴い、AVI-STREAM-002 全体に近づく。入力次元と失敗解析だけが増える。
- **direct avi_layout / strl composition proof（不採用）**: byte slice、`Chunks` iterator、`Vec<StreamHead>`、`has_strh`、missing `strh`、`strf` `Vec` 複製、error precedenceを含む integration proof。今回の最小 pure/bounded boundaryを超える。
- **full AVI-STREAM-002 declaration validation proof（不採用）**: `strf` payload validation、codec/format matrix、declaration correspondenceを対象にする案。Requiredだが別 propertyであり、今回の strh field extraction/validationとは独立に設計する。
- **AVI-TIME-004 accumulation proof（不採用）**: `tick_advance`、remainder、複数 stream/packet clockを含み、u128 除算と multi-stream accumulationを伴う別の heavy Required propertyである。

採用案は KISS/YAGNI/True to the Codeで決める。`cfg(kani)` 置換、fmt/alloc stub、solver 総当たり、timeout 延長だけの解決は禁止する。proofが通らなかった後で length domainや field coverageを縮める場合は weakeningとして扱い、Passとせず Coordinatorに askする。

## 8. runner と予算

runner は `AL_KANI_HARNESS_GROUP="avi-strh-declaration"` で `avi_strh_declaration_bounded_exact` を選択する。registry は 31 harness（WAV 13 + MP4 scan 1 + MP4/AVI boundary 2 + AVI movi tag 1 + AVI movi composition 1 + AVI movi scan 1 + AVI stream kind 1 + AVI strh declaration 1 + legacy containers 10）となる見込みである。既存 verdict フィールド、既定全件挙動、fail-closed、WSL mirror、target 配置、timeout 意味論、group/subset の排他意味論を変えない。全件実行はこの slice の必須検証にしない。

単一 proof timeout は canonical laneで `AL_KANI_TIMEOUT_MS=600000` とする。各 proofの設計予算は 10 分以内・4GiB 未満、avi-strh-declaration 選択実行の合計実測は 10〜20 分を目標とする。timeout、exit 0、CI greenを Proof Passにしない。Required Passが得られなければ proof/contractを弱めず、draft PR / Required NotRunとして報告する。

## 9. mutation detection plan

Able to Fail 証拠として、strh declaration 意味を壊す一時 mutationを入れる。第一候補は production extraction helperの `scale` offsetを 20 から 21 へずらす変更である。期待は avi-strh-declaration proofに `VERIFICATION:- FAILED` が出ることである。generic boundary/tag/entry/scan/stream kind proofだけが Failする mutationを strh declaration oracleの証拠とみなさない。diagnostic payloadだけの mutationは使わない。確認後に完全 revertし、最終状態で canonical laneを再実行する。検出されなければ mutationが不適切か oracleが弱いかを区別し、weakened Passにしない。

## 10. proptest との関係

既存の mutation/truncation/no-panic proptestは弱めない。formal proofは full symbolic length scalarと fixed prefixの pure extraction/validationを exactに検査し、proptestは public integrationを統計的に検査する。short `strh` 拒否、exact 48 byte 受理、offset/endianness、`fcc_type` byte順序、trailing/unobserved byte無視のうち既存 unit/integration testで不足する境界だけを `avi.rs` の deterministic unit testへ追加する。`formal_containers.rs` は変更しない。

## 11. RF 還元判定

RIFF/AVI `strh` declaration の fixed 48 byte layoutと LE u32 offset は形式仕様として汎用化できる可能性がある。ただし現時点の接続条件は AudioLens の `StreamHead` 責務、error 文言、partial ingest、timing/downstream解釈に結び付いており、実証済みの再利用契約として切り出せる段階ではない。本 slice では Ruviform Issueを起票せず、候補と理由を PR本文に記録する。

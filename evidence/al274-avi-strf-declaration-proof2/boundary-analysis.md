# AVI video `strf` BITMAPINFOHEADER declaration bounded exact proof boundary analysis

対象は Issue #274 の AVI-STREAM-002 残件のうち、first `vids` metadata path が video `strf` chunk payload を BITMAPINFOHEADER としてどう size gate し、declaration field へどう写像するかという small pure boundary である。本書は solver 実験、runner 変更、production 変更に先立つ境界設計であり、`strf` presence/copy/ordering、audio `strf` WAVEFORMATEX validation、BITMAPINFOHEADER decode suitability matrix、declaration correspondence を含む AVI-STREAM-002 declaration validation 全体、AVI-TIME-004、AVI-PCM、MP4-TABLE-005、MP4-PCM を完了させるものではない。PR #282〜#290 の WAV/MP4/AVI boundary/scan/tag/entry/stream kind/strh declaration proof は再設計せず、runner 変更の regression としてだけ再実行する。

## 1. 実コード確認と property の選定

現行 production parser の `avi_layout` は `hdrl/strl` list の inner chunk を走査し、`strf` を見ると `head.format = cbody.to_vec()` で opaque byte payload を保持する。現行構造は `strf` presence を表す型を持たず、missing `strf` と explicit empty `strf` を区別しない。複数 `strf` がある場合は loop 構造上 later assignment が先の payload を上書きする。`avi_layout` は `strf` の length や内容で container を reject しない。

first `vids` metadata path の `video_info` は `head.format` が 40 byte 未満なら `MediaError::malformed("video strf shorter than BITMAPINFOHEADER")` を返し、caller はそれを `.ok()` で落として video metadata なし/後段 unavailable に degrade させる。40 byte 以上では width を offset 4..8 の little-endian u32、height bit pattern を offset 8..12 の little-endian u32、bits per pixel を offset 14..16 の little-endian u16、compression を offset 16..20 の little-endian u32 として読む。metadata height は u32 bit pattern を i32 へ再解釈して `unsigned_abs` を取る。width/height/compression/bits per pixel の値 validity はこの metadata extraction path で新たに拒否しない。feature extraction 側の `dib_format` は decode suitability の別責務であり、本 boundary に含めない。

この boundary を選ぶ理由は、AVI-STREAM-002 残件のうち iterator/body/`Vec` 合成、stream ordering/declaration correspondence、codec/layout matrix、u128 timing division、PCM conversion matrix、MP4 sample table に依存せず、fixed 40 byte layout、length scalar、少数 scalar field の pure boundary として independent に閉じられる最小単位だからである。audio `strf` WAVEFORMATEX は AVI audio path にとって重要だが、現行 parsing helper は `crates/al-media/src/wav.rs` にあり、helper 抽出や mutation が lease 外になる可能性が高い。`strf` presence/capture shell は actual body slice、`Vec` copy、strl iteration、ordering/last-wins を含む integration boundary になる。video metadata extraction は lease 内の `avi.rs` だけで True to the Code に接続できる。

## 2. exact input domain

proof harness 名は `avi_video_strf_declaration_bounded_exact`、runner group 名は `avi-strf-declaration` とする。proof の入力は次の 2 軸だけである。

1. `body_len: usize`。machine integer domain 全体を symbolic に扱う。`body_len < 40` の local rejection class と `body_len >= 40` の extraction class を exact complement として検査する。length を制限する `kani::assume` は置かない。
2. `prefix: [u8; 40]`。各 byte は独立に full 8 bit を取り、受理された video `strf` body の先頭 40 byte が取りうる全 2^320 bit pattern を覆う。width/height/bits-per-pixel/compression field byte、`biSize`、planes、その他の未観測 byte を writer fixture や代表値へ限定しない。

この分離は exact reparameterization である。production の first `vids` metadata path は length validation と fixed-prefix extraction をこの順に合成し、field extraction は受理時の先頭 40 byte だけを観測する。実 body の trailing byte、BITMAPINFOHEADER 内の未観測 field、body slice の lifetime、outer `strl` の並び、`StreamHead.format` の `Vec` copy は extraction core の入力ではない。任意の実 body は、その length と、受理時なら先頭 40 byte によってこの boundary の振る舞いが決まる。逆に任意の `body_len >= 40` と任意の 40 byte prefix の組合せは、その prefix を先頭に持つ実 body として構成できる。したがって length scalar と fixed prefix の Cartesian product は、local rejection/extraction validation と declaration field extraction の入力領域を漏れなく表す。

40 byte 以降の trailing payload をモデル化しない根拠は、production helper が fixed `[u8; 40]` だけを受け取り、型上 trailing byte を観測できないことである。offset 0..4、12..14、20..40 など production metadata extraction が読まない byte は fixed prefix 内に存在するが、proof assertion が出力を観測 offset の byteだけで規定するため、production 実装がこれらの未観測 byteへ依存すれば反例になる。これは trailing/unobserved byte を concrete に固定する input narrowing ではない。

length validation は byte に依存せず、field extraction は length に依存しない。したがって 2 軸を独立に symbolic化しても合成意味の coverage は落ちない。body slice から fixed prefix を作る shell の slice/copy composition はこの proof の対象外であり、任意長 body slice composition の機械証明とは読み替えない。

## 3. 証明する性質

proof は symbolic `body_len` と symbolic fixed prefix に対し、production pure helper の total correctnessを 1 proofで検査する。検査する性質は少なくとも次の通りである。

- `body_len < 40` の全入力が first `vids` metadata path の local rejection class、`body_len >= 40` の全入力が extraction classになり、両者が exact complement であること。
- 受理時に width が offset 4..8 の little-endian u32、height bit pattern が offset 8..12 の little-endian u32、bits per pixel が offset 14..16 の little-endian u16、compression が offset 16..20 の little-endian u32 に正確に一致すること。
- metadata height が production semantics（u32 bit pattern を i32 へ再解釈し `unsigned_abs` を取る。`i32::MIN` bit pattern を含む）に正確に一致すること。
- byte-order 反転、offset ずれ、field の drop/mis-map、field 間の入れ替わりがないこと。
- declaration field view が width、height、bits per pixel、compression の production metadata extraction field を過不足なく対応付けること。
- これらの計算で overflow、underflow、panic がないこと。
- width/height/compression/bits per pixel の値 validity をこの boundary が新たに拒否しないこと。

この proof は fixed BITMAPINFOHEADER prefix model と full machine integer length domainについて finite machine integer/byte domain 全体の total correctness proof であり、representative sampling ではない。fixed-length model の成功を、任意長 body slice、実 `Vec<StreamHead>`/`strl` 走査、`strf` presence/copy semantics、audio `strf`、AVI-STREAM-002 全体の機械証明と読み替えない。

## 4. production 接続条件

proof model は test-side の field 抽出複製ではなく production pure helperを呼ぶ。最終設計は次の最小責務へ分離する。

- production length predicate が `body_len >= 40` の extraction/rejection 規則を返す。
- production fixed-prefix extraction helper が `[u8; 40]` から width、height、bits per pixel、compression のみを持つ declaration field view を返す。height は raw bit pattern を production metadata semantics どおり i32 へ再解釈した `unsigned_abs` とする。
- production `video_info` は first `vids` metadata use site として、length predicate を通らなかった body を現行どおり同じ error文言で local error にし、受理時は先頭 40 byte を fixed prefix へ写して同じ extraction helperを呼び、返った compression/width/height/bits-per-pixel を `VideoInfo` へ写す。

この分離は製品契約を変えない。`scale`/`rate`/`length` から導く fps/frames/duration、codec string rendering、feature extraction、timing、PCM decode は extraction helper に関与させない。`avi_layout` 側は `strf` body の `Vec` 複製を現行どおり shell に残し、offset 定数、endian、length gate、`VideoInfo` への写像を複製しない。field view は solver に合わせた代替実装ではなく、production metadata path が読む 4 field だけを BITMAPINFOHEADER declaration bytes の解釈責務として表す最小の data shape である。

公開 API、error 種別・文言（`"video strf shorter than BITMAPINFOHEADER"` を含む）、error precedence、正常/異常系の振る舞い、f64 精度、対応形式、partial ingest/capability semantics、stream/timing/video/audio 解釈を変えない。missing/short/semantically invalid `strf` の degrade/reject 意味、40 byte 以降 trailing byte と未観測 fieldの無視、missing/empty `strf` 非区別、複数 `strf` last-wins を変えない。40 byte 未満の受理、trailing/unobserved field を理由にした新たな拒否、width/height/compression/bits per pixel の validity matrix 追加は行わない。

## 5. assume と coverage

input narrowing は禁止である。`body_len`、prefix byte、width/height/compression/bits-per-pixel byte を制限する `kani::assume` は置かない。fixed BITMAPINFOHEADER prefix length 40 は AVI video `strf` の固定レイアウトであり、bounded model の明示 scope であって input narrowing ではない。length class の complement は full `usize` 上で証明する。`vids`/`auds` selection や `fcc_type` classification は PR #289/#290 の別境界であり、この proof では first `vids` metadata path に適用される payload layout だけを扱う。未証明の数学的事実を assume しない。loop を含まない設計を優先し、unwinding assertion を無効化しない。

coverage は少なくとも次を落とさない。

- `body_len < 40` / `body_len == 40` / `body_len > 40` を含む full length domain の rejection/extraction complement。
- width、height bit pattern、bits per pixel、compression の全 byte patternと little-endian decode。
- `i32::MIN` を含む height bit pattern に対する signed/`unsigned_abs` 解釈。
- offset 0..4、12..14、20..40 の未観測 byte が出力へ寄与しないこと。
- 4 field の写像 completeness。

expected oracle は production extraction helperを呼ばず、raw prefix byteから独立に `u16::from_le_bytes` / `u32::from_le_bytes` / `as i32` / `unsigned_abs` で期待値を作る。proofが production helper同士の一致だけを検査して製品意味とのずれを検出できない状態にしない。

## 6. 検証しない領域

次の領域はこの local proof の外に置き、ledger の未証明領域に記録する。

- `Chunks` iterator、`hdrl`/`strl` LIST 走査、entry ordering、error precedence、`MediaError` 文言生成 shell。
- `strf` presence/existence、missing `strf` と empty `strf` の区別、複数 `strf` の ordering/last-wins、`has_strh` による stream 収集、missing `strh`、declaration correspondence。
- `StreamHead.format` の `Vec` copy allocation、任意長 body slice/lifetime composition、body から fixed prefix への slice/copy shell、実 `Vec<StreamHead>` accumulation。
- audio WAVEFORMATEX parsing/validation、WAVEFORMATEX codec/layout matrix、PCM decode。
- BITMAPINFOHEADER decode suitability validation（dimension/compression/bit depth）、DIB/pixel decode、codec string rendering。
- timing accumulation（`tick_advance`/`stream_timings`/`start_tick`）、`packet_ticks`、remainder、u128 除算。
- partial ingest/capability observation の upper integration、writer/corruption whole-call proof、I/O、allocation、diagnostic formatting、公開 API 全体 no-panic。
- MP4、signature 再構成。

これらを local proof Pass から上位保証へ転用しない。

## 7. decomposition 候補の比較

- **video `strf` BITMAPINFOHEADER size/field extraction proof（採用）**: full `usize` length scalarと symbolic fixed `[u8; 40]` prefixに対し、production length predicateと production fixed-prefix extraction helperを合成して length gate、width/height/bits-per-pixel/compression の exact offset decode、production metadata field viewへの写像、算術安全性を 1 proofで検査する。`Vec` accumulation、`strl` 走査、body slice/lifetime、I/O、pixel decode、codec string renderingを持たない。今回の最小 pure/bounded boundaryに一致し、offset/endianness/length/height-sign mutationを oracleが直接検出できる。harness 名は `avi_video_strf_declaration_bounded_exact`、group 名は `avi-strf-declaration` とする。
- **audio `strf` WAVEFORMATEX size/field extraction proof（不採用）**: 16 byte 未満の local rejectionと format tag/channels/sample rate/byte rate/block align/bits per sampleの fixed-prefix decodeを対象にする案。重要だが現行 helperは `crates/al-media/src/wav.rs` にあり、helper 抽出や mutation が lease 外になる可能性が高い。lease 拡張なしに実装せず、AVI-PCM/WAV-PCM codec matrixへ拡大しない。
- **`strf` shell presence/capture proof（不採用）**: missing/present/empty/multiple `strf` と `head.format` copyの semanticsを対象にする案。現行 productionは presenceを区別する型を持たず、actual body slice、`Vec` copy、strl iteration、ordering/last-winsを含む integration boundaryになる。今回の最小 pure/bounded boundaryを超える。
- **length-gate-only proof（不採用）**: video `strf` の 40 byte 未満 local rejection/受理だけを検査する案。最小だが field offset、endianness、写像を閉じず、declaration semanticsの中核を残す。
- **field-decode-only proof（不採用）**: 十分長 video `strf` の offset/endianness/写像だけを検査する案。short local rejectionを閉じず、length gateとの exact complementを別 proofへ委ねる。2 proof familyでも total correctnessを保てるが、入力次元が小さく 1 proofで実用予算に入る見込みが高いため分割しない。
- **valid/invalid length family（不採用）**: reject classと extraction classを 2 proofに分ける案。total correctnessと exact complementを失わない範囲では可能だが、単一 proofが軽いため registryと evidenceを増やす利益がない。proof分割を input narrowingに使わない。
- **combined audio+video `strf` declaration proof（不採用）**: WAVEFORMATEXと BITMAPINFOHEADERを 1つの declaration proofに合成する案。codec/layout matrixと stream-type dependent interpretationを伴い、AVI-STREAM-002 declaration validation全体に近づく。入力次元と失敗解析だけが増える。
- **direct avi_layout / strl composition proof（不採用）**: byte slice、`Chunks` iterator、`Vec<StreamHead>`、`strf` `Vec` copy、missing/duplicate `strf`、error precedenceを含む integration proof。今回の最小 pure/bounded boundaryを超える。
- **full AVI-STREAM-002 declaration validation proof（不採用）**: `strf` payload validation、codec/format matrix、declaration correspondenceを対象にする案。Requiredだが別 propertyであり、今回の narrow field extraction/validationとは独立に設計する。
- **AVI-TIME-004 / AVI-PCM / MP4-TABLE-005 proof（不採用）**: u128 division/remainder/multi-stream accumulation、codec conversion matrix、multi-table accumulation/cross-table consistencyを伴う別の heavy Required propertyである。

採用案は KISS/YAGNI/True to the Codeで決める。`cfg(kani)` 置換、fmt/alloc stub、solver 総当たり、timeout 延長だけの解決は禁止する。proofが通らなかった後で length domainや field coverageを縮める場合は weakeningとして扱い、Passとせず Coordinatorに askする。

## 8. runner と予算

runner は `AL_KANI_HARNESS_GROUP="avi-strf-declaration"` で `avi_video_strf_declaration_bounded_exact` を選択する。registry は 32 harness（WAV 13 + MP4 scan 1 + MP4/AVI boundary 2 + AVI movi tag 1 + AVI movi composition 1 + AVI movi scan 1 + AVI stream kind 1 + AVI strh declaration 1 + AVI strf declaration 1 + legacy containers 10）となる見込みである。既存 verdict フィールド、既定全件挙動、fail-closed、WSL mirror、target 配置、timeout 意味論、group/subset の排他意味論を変えない。全件実行はこの slice の必須検証にしない。

単一 proof timeout は canonical laneで `AL_KANI_TIMEOUT_MS=600000` とする。各 proofの設計予算は 10 分以内・4GiB 未満、avi-strf-declaration 選択実行の合計実測は 10〜20 分を目標とする。timeout、exit 0、CI greenを Proof Passにしない。Required Passが得られなければ proof/contractを弱めず、draft PR / Required NotRunとして報告する。

## 9. mutation detection plan

Able to Fail 証拠として、strf declaration 意味を壊す一時 mutationを入れる。第一候補は production extraction helperの width little-endian input bytesを正しい offset 4..8 から offset 5..9 へ 1 byte ずらす変更である。期待は avi-strf-declaration proofに `VERIFICATION:- FAILED` が出ることである。generic boundary/tag/entry/scan/stream kind/strh proofだけが Failする mutationを strf declaration oracleの証拠とみなさない。diagnostic payloadだけの mutationは使わない。確認後に完全 revertし、最終状態で canonical laneを再実行する。検出されなければ mutationが不適切か oracleが弱いかを区別し、weakened Passにしない。

## 10. proptest との関係

既存の mutation/truncation/no-panic proptestは弱めない。formal proofは full symbolic length scalarと fixed prefixの pure extraction/validationを exactに検査し、proptestは public integrationを統計的に検査する。short video `strf` local rejection、exact 40 byte受理、offset/endianness、height sign/`unsigned_abs`、trailing/unobserved byte無視のうち既存 unit/integration testで不足する境界だけを `avi.rs` の deterministic unit testへ追加する。`formal_containers.rs` は変更しない。

## 11. RF 還元判定

RIFF/AVI video `strf` BITMAPINFOHEADER の fixed 40 byte layoutと LE field offset は形式仕様として汎用化できる可能性がある。ただし現時点の接続条件は AudioLens の metadata extraction、error 文言、partial ingest、decode suitability 分離に結び付いており、実証済みの再利用契約として切り出せる段階ではない。本 slice では Ruviform Issueを起票せず、候補と理由を PR本文に記録する。

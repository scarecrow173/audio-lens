# AVI audio `strf` WAVEFORMATEX declaration bounded exact proof boundary analysis

対象は Issue #274 の AVI-STREAM-002 残件のうち、first `auds` decode pathと `decoder.rs` probeが audio `strf` chunk payloadを WAVEFORMATEX としてどう size gate し、fixed declaration fieldへどう写像するかという small pure boundary である。本書は solver 実験、runner 変更、production 変更に先立つ境界設計であり、`strf` presence/copy/ordering、`strh`/`strf` declaration correspondence、WAVEFORMATEX codec/layout semantic validation、PCM decode、AVI-STREAM-002 declaration validation 全体、AVI-TIME-004、AVI-PCM、MP4-TABLE-005、MP4-PCM を完了させるものではない。PR #282〜#291 の WAV/MP4/AVI boundary/scan/tag/entry/stream kind/strh/video `strf` declaration proof は再設計せず、runner 変更の regression としてだけ再実行する。

## 1. 実コード確認と property の選定

現行 production parser の `avi_layout` は `hdrl/strl` list の inner chunkを走査し、`strf` を見ると `head.format = cbody.to_vec()` で opaque byte payloadを保持する。現行構造は `strf` presenceを表す型を持たず、missing `strf` と explicit empty `strf` を区別しない。複数 `strf` がある場合は loop 構造上 later assignment が先の payloadを上書きする。`avi_layout` は `strf` の length や内容で container を reject しない。

first `auds` decode path は選択 streamの `NNwb` packet bodyを mux 順に連結し、`decode_waveformatex(&streams[audio_idx].format, &audio_pcm)` を呼ぶ。decode-level failure は container fatal ではなく、`audio` を `None`、理由を `audio_unavailable`、型付き区分を `audio_failure` へ落とす partial ingest である。`decoder.rs` probe も `crate::wav::parse_fmt(&head.format)` を `auds` stream facts に使う。

`crates/al-media/src/wav.rs` の現行 `parse_fmt` は 16 byte 未満を `MediaError::malformed("fmt chunk shorter than 16 bytes")` で拒否し、16 byte 以上なら format tag を offset 0..2 の little-endian u16、channels を offset 2..4 の little-endian u16、sample rate を offset 4..8 の little-endian u32、byte rate を offset 8..12 の little-endian u32、block align を offset 12..14 の little-endian u16、bits per sample を offset 14..16 の little-endian u16 として読む。16 byte 以降の trailing byte（18 byte WAVEFORMATEX の `cbSize` や extensionを含む）は現行 extraction が観測しない。unsupported format tag、zero channels/sample rate/block align、inconsistent block align/byte rate は structural extraction 段階で拒否せず、`sample_layout` 以降の codec/layout semantic validationが扱う。

この boundaryを選ぶ理由は、AVI-STREAM-002 残件のうち iterator/body/`Vec` 合成、stream ordering/declaration correspondence、`strf` presence/copy/ordering、`wb` scan/concatenation、codec/layout matrix、u128 timing division、PCM conversion matrix、MP4 sample tableに依存せず、fixed 16 byte layout、length scalar、6 scalar fieldの pure boundaryとして independent に閉じられる最小の意味ある単位だからである。length-gate-only は field mappingを残し、field-decode-only は rejection complementを残すため完了境界にしない。presence/copy/ordering は shell integration boundaryで一段重く、semantic validation は codec/layout matrixを伴う別境界である。

## 2. exact input domain

proof harness 名は `avi_audio_strf_declaration_bounded_exact`、runner group 名は `avi-strf-audio-declaration` とする。proofの入力は次の 2 軸だけである。

1. `body_len: usize`。machine integer domain全体を symbolicに扱う。`body_len < 16` の structural rejection classと `body_len >= 16` の extraction classを exact complement として検査する。lengthを制限する `kani::assume` は置かない。
2. `prefix: [u8; 16]`。各 byteは独立に full 8 bitを取り、受理された WAVEFORMATEX bodyの先頭 16 byteが取りうる全 2^128 bit patternを覆う。format tag/channels/sample rate/byte rate/block align/bits per sample field byteを writer fixture や代表値へ限定しない。fixed prefix内に production extractionが読まない byteはない。

この分離は exact reparameterization である。production `parse_fmt` は length validationと fixed-prefix extractionをこの順に合成し、field extractionは受理時の先頭 16 byteだけを観測する。実 bodyの trailing byte、body sliceの lifetime、outer `strl` の並び、`StreamHead.format` の `Vec` copy、first `auds` selection、`wb` payload bytesは extraction coreの入力ではない。任意の実 bodyは、その lengthと、受理時なら先頭 16 byteによってこの boundaryの振る舞いが決まる。逆に任意の `body_len >= 16` と任意の 16 byte prefixの組合せは、その prefixを先頭に持つ実 bodyとして構成できる。したがって length scalarと fixed prefixの Cartesian productは、structural rejection/extraction validationと declaration field extractionの入力領域を漏れなく表す。

16 byte以降の trailing payloadをモデル化しない根拠は、production helperが fixed `[u8; 16]` だけを受け取り、型上 trailing byteを観測できないことである。これは trailing byteを concreteに固定する input narrowing ではない。fixed prefix内の byteはすべて 6 fieldのいずれかに属し、未観測 byteはない。length validationは byteに依存せず、field extractionは lengthに依存しない。したがって 2 軸を独立に symbolic化しても合成意味の coverageは落ちない。body sliceから fixed prefixを作る shellの slice/copy compositionはこの proofの対象外であり、任意長 body slice compositionの機械証明とは読み替えない。

## 3. 証明する性質

proofは symbolic `body_len` と symbolic fixed prefixに対し、production pure helperの total correctnessを 1 proofで検査する。検査する性質は少なくとも次の通りである。

- `body_len < 16` の全入力が structural rejection class、`body_len >= 16` の全入力が extraction classになり、両者が exact complementであること。
- 受理時に format tagが offset 0..2 の little-endian u16、channelsが offset 2..4 の little-endian u16、sample rateが offset 4..8 の little-endian u32、byte rateが offset 8..12 の little-endian u32、block alignが offset 12..14 の little-endian u16、bits per sampleが offset 14..16 の little-endian u16に正確に一致すること。
- byte-order反転、offsetずれ、fieldの drop/mis-map、field間の入れ替わりがないこと。
- declaration field viewが productionが読む 6 fieldを過不足なく対応付けること。既存 `FmtChunk` がその最小 viewであり、新しい重複 structは作らない。
- これらの計算で overflow、underflow、panicがないこと。
- format tagの supported/unsupported、channels/sample rate/block alignの zero、block align/byte rate consistencyをこの structural boundaryが新たに拒否しないこと。

この proofは fixed WAVEFORMATEX prefix modelと full machine integer length domainについて finite machine integer/byte domain全体の total correctness proofであり、representative samplingではない。fixed-length modelの成功を、任意長 body slice、実 `Vec<StreamHead>`/`strl` 走査、`strf` presence/copy semantics、PCM decode、AVI-STREAM-002 全体の機械証明と読み替えない。

## 4. production 接続条件

proof modelは test-sideの field抽出複製ではなく production pure helperを呼ぶ。最終設計は次の最小責務へ分離する。

- production length predicate `waveformatex_accepts_body_len` が `body_len >= 16` の extraction/rejection規則を返す。
- production fixed-prefix extraction helper `waveformatex_declaration_fields` が `[u8; 16]` から既存 `FmtChunk` を返す。
- production `parse_fmt` は length predicateを通らなかった bodyを現行どおり同じ error文言 `fmt chunk shorter than 16 bytes` で拒否し、受理時は先頭 16 byteを fixed prefixへ写して同じ extraction helperを呼ぶ。

この分離は製品契約を変えない。`decode_waveformatex` は `parse_fmt` を呼ぶ現行構造を保ち、AVI first `auds` decode pathは同じ production chain（`avi_layout` `strf` copy → `StreamHead.format` → first `auds` `decode_waveformatex` → `parse_fmt` → length predicate/fixed-prefix helper）を使う。`decoder.rs` probeの `parse_fmt` useも同じ `parse_fmt` に残る。`sample_layout` の codec/layout matrix、PCM sample conversion、partial ingest/capability observationは extraction helperに関与させない。`avi_layout` 側は `strf` bodyの `Vec` 複製を現行どおり shellに残し、WAVEFORMATEX offset、endianness、length gateを複製しない。

公開 API、`parse_fmt`/`decode_waveformatex` signature、error種別・文言（`"fmt chunk shorter than 16 bytes"` を含む）、error precedence、正常/異常系の振る舞い、f64精度、対応形式、partial ingest/capability semantics、stream/timing/audio解釈を変えない。missing/short/semantically invalid `strf` の degrade/reject意味、16 byte以降 trailing/extension byteの無視、missing/empty `strf` 非区別、複数 `strf` last-winsを変えない。16 byte未満の受理、trailing/extension/cbSize byteを理由にした新たな拒否、unsupported format tag/zero channels/zero sample rate/inconsistent block align/byte rateに対する structural extraction段階での新たな拒否、codec validity matrix追加は行わない。helper抽出は `decoder.rs` や他 lease外 use siteの変更を必要としない。

## 5. assume と coverage

input narrowingは禁止である。`body_len`、prefix byte、format tag/channels/sample rate/byte rate/block align/bits per sample byteを制限する `kani::assume` は置かない。fixed WAVEFORMATEX prefix length 16は現行 production layoutの固定観測領域であり、bounded modelの明示 scopeであって input narrowingではない。length classの complementは full `usize` 上で証明する。`auds` selection、`wb` filtering、codec validityは別境界であり、この proofでは declaration bytesの structural extractionだけを扱う。未証明の数学的事実を assumeしない。loopを含まない設計を優先し、unwinding assertionを無効化しない。

coverageは少なくとも次を落とさない。

- `body_len < 16` / `body_len == 16` / `body_len > 16` を含む full length domainの rejection/extraction complement。
- format tag、channels、sample rate、byte rate、block align、bits per sampleの全 byte patternと little-endian decode。
- 6 fieldの写像 completeness。
- trailing byte非依存が型上分離で表されていること。

expected oracleは production extraction helperを呼ばず、raw prefix byteから独立に `u16::from_le_bytes` / `u32::from_le_bytes` で期待値を作る。proofが production helper同士の一致だけを検査して製品意味とのずれを検出できない状態にしない。

## 6. 検証しない領域

次の領域はこの local proofの外に置き、ledgerの未証明領域に記録する。

- `Chunks` iterator、`hdrl`/`strl` LIST走査、entry ordering、error precedence、`MediaError`文言生成 shell。
- `strf` presence/existence、missing `strf`と empty `strf`の区別、複数 `strf`の ordering/last-wins、`has_strh`による stream収集、missing `strh`、declaration correspondence。
- `StreamHead.format`の `Vec` copy allocation、任意長 body slice/lifetime composition、bodyから fixed prefixへの slice/copy shell、実 `Vec<StreamHead>` accumulation。
- first `auds` selection、`wb` packet scan/concatenation、audio data mid-frame semantics。
- WAVEFORMATEX codec/layout matrix（`sample_layout`の supported encoding/zero/consistency validation）、PCM sample conversion。
- partial ingest/capability observationの upper integration、probe stream factsの全件 integration、writer/corruption whole-call proof、I/O、allocation、diagnostic formatting、公開 API全体 no-panic。
- MP4、signature再構成。

これらを local proof Passから上位保証へ転用しない。

## 7. decomposition候補の比較

- **audio `strf` WAVEFORMATEX fixed-prefix size/field extraction proof（採用）**: full `usize` length scalarと symbolic fixed `[u8; 16]` prefixに対し、production length predicateと production fixed-prefix extraction helperを合成して length gate、6 fieldの exact offset/endianness/decode、declaration field viewへの写像、算術安全性を 1 proofで検査する。`Vec` accumulation、`strl`走査、body slice/lifetime、`wb` payload、PCM decode、codec matrix、I/Oを持たない。今回の最小 pure/bounded boundaryに一致し、offset/endianness/length/field mutationを oracleが直接検出できる。harness名は `avi_audio_strf_declaration_bounded_exact`、group名は `avi-strf-audio-declaration` とする。harnessは helperが `wav.rs` にあるため `wav.rs` の `#[cfg(kani)]` proof moduleに置く。
- **direct `parse_fmt` dynamic slice proof（不採用）**: symbolic lengthから rejection/extraction representative sliceを構成して現行 `parse_fmt`を直接呼ぶ案。production変更を避けられるが、`len >= 16`の trailing byte non-dependenceを型で閉じられず、短い slice構成・error allocation・動的 slice shellを proofに持ち込む。fixed-prefix helperへ分離できるため採用しない。
- **length-gate-only proof（不採用）**: 16 byte未満の structural rejection/受理だけを検査する案。最小だが field offset、endianness、写像を閉じず、declaration semanticsの中核を残す。
- **field-decode-only proof（不採用）**: 十分長 audio `strf`の offset/endianness/写像だけを検査する案。short structural rejectionを閉じず、length gateとの exact complementを別 proofへ委ねる。2 proof familyでも total correctnessを保てるが、入力次元が小さく 1 proofで実用予算に入る見込みが高いため分割しない。
- **valid/invalid length family（不採用）**: reject classと extraction classを 2 proofに分ける案。total correctnessと exact complementを失わない範囲では可能だが、単一 proofが軽いため registryと evidenceを増やす利益がない。proof分割を input narrowingに使わない。
- **WAVEFORMATEX semantic validation / codec layout proof（不採用）**: `sample_layout`の supported format matrix、zero channel/rate/block align、block align/byte rate consistency、mid-frame data semanticsを対象にする案。AVI-PCM/WAV-PCMの codec/layout matrixであり、今回の structural declaration extractionとは別の重い boundaryである。
- **`strf` shell presence/capture proof（不採用）**: missing/present/empty/multiple `strf`と `head.format` copyの semanticsを対象にする案。actual body slice、`Vec` copy、strl iteration、ordering/last-winsを含む integration boundaryになる。今回の最小 pure/bounded boundaryを超える。
- **combined audio+video `strf` declaration proof（不採用）**: WAVEFORMATEXと BITMAPINFOHEADERを 1つの declaration proofに合成する案。stream-type dependent interpretationを伴い、入力次元と失敗解析だけが増える。
- **direct avi_layout / strl composition proof（不採用）**: byte slice、`Chunks` iterator、`Vec<StreamHead>`、`strf` `Vec` copy、missing/duplicate `strf`、error precedenceを含む integration proof。今回の最小 pure/bounded boundaryを超える。
- **full AVI-STREAM-002 declaration validation proof / AVI-TIME-004 / AVI-PCM / MP4-TABLE-005 proof（不採用）**: declaration correspondence、codec/format matrix、u128 division/remainder/multi-stream accumulation、multi-table accumulation/cross-table consistencyを伴う別の heavy Required propertyである。

採用案は KISS/YAGNI/True to the Codeで決める。`cfg(kani)`置換、fmt/alloc stub、solver総当たり、timeout延長だけの解決は禁止する。proofが通らなかった後で length domainや field coverageを縮める場合は weakeningとして扱い、Passとせず Coordinatorに askする。

## 8. runnerと予算

runnerは `AL_KANI_HARNESS_GROUP="avi-strf-audio-declaration"` で `avi_audio_strf_declaration_bounded_exact`を選択する。registryは 33 harness（WAV 13 + MP4 scan 1 + MP4/AVI boundary 2 + AVI movi tag 1 + AVI movi composition 1 + AVI movi scan 1 + AVI stream kind 1 + AVI strh declaration 1 + AVI video strf declaration 1 + AVI audio strf declaration 1 + legacy containers 10）となる。既存 verdictフィールド、既定全件挙動、fail-closed、WSL mirror、target配置、timeout意味論、group/subsetの排他意味論を変えない。全件実行はこの sliceの必須検証にしない。

単一 proof timeoutは canonical laneで `AL_KANI_TIMEOUT_MS=600000` とする。各 proofの設計予算は 10分以内・4GiB未満、avi-strf-audio-declaration選択実行の合計実測は 10〜20分を目標とする。timeout、exit 0、CI greenを Proof Passにしない。Required Passが得られなければ proof/contractを弱めず、draft PR / Required NotRunとして報告する。

## 9. mutation detection plan

Able to Fail証拠として、audio strf declaration意味を壊す一時 mutationを入れる。第一候補は production extraction helperの sample rate little-endian input bytesを正しい offset 4..8から offset 5..9へ 1 byteずらす変更である。期待は avi-strf-audio-declaration proofに `VERIFICATION:- FAILED`が出ることである。generic boundary/tag/entry/scan/stream kind/strh/video strf proofだけが Failする mutationを audio strf declaration oracleの証拠とみなさない。diagnostic payloadだけの mutationは使わない。確認後に完全 revertし、最終状態で canonical laneを再実行する。検出されなければ mutationが不適切か oracleが弱いかを区別し、weakened Passにしない。

## 10. proptestとの関係

既存の mutation/truncation/no-panic proptestは弱めない。formal proofは full symbolic length scalarと fixed prefixの pure extraction/validationを exactに検査し、proptestは public integrationを統計的に検査する。short audio `strf` structural rejection、exact 16 byte受理、16 byte超 trailing/extension byte無視、offset/endianness、unsupported/zero/inconsistent valuesを structural extractionで拒否しない契約、AVI partial ingest degradeのうち既存 unit/integration testで不足する境界だけを `wav.rs` または `avi.rs`の deterministic unit testへ追加する。`formal_containers.rs`は変更しない。

## 11. RF還元判定

WAVEFORMATEX fixed 16 byte layoutと LE field offsetは形式仕様として汎用化できる可能性がある。ただし現時点の接続条件は AudioLensの AVI/WAV extraction、error文言、semantic validation分離、partial ingestに結び付いており、実証済みの再利用契約として切り出せる段階ではない。本 sliceでは Ruviform Issueを起票せず、候補と理由を PR本文に記録する。

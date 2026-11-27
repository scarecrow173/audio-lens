# container format signature detection bounded exact proof boundary analysis

対象は Issue #274 の canonical 21 proof disposition で再構成が求められている `detect_format` の signature decision boundary である。本書は solver 実験、runner 変更、production 変更に先立つ境界設計であり、MP4/AVI/WAV decoder correctness、box/chunk walker、stream declaration、sample table、PCM、timing、writer round-trip、corruption no-panic、AVI declaration correspondence、AVI-TIME-004、MP4-TABLE-005、legacy proof retire を完了させるものではない。PR #282〜#292 の WAV/MP4/AVI proof は再設計せず、runner 変更の regression としてだけ再実行する。

## 1. 実コード確認と property の選定

現行 production `crates/al-media/src/format.rs` の `detect_format` は pure `&[u8] -> Option<MediaFormat>` であり、I/O、allocation、diagnostic formatting、decode を持たない。現行契約は次の通りである。

1. `data.len() >= 12` かつ offset 0..4 が exact `RIFF` のときだけ RIFF branch に入る。offset 8..12 が exact `WAVE` なら `Some(MediaFormat::Wav)`、exact `AVI `（ trailing space 付き）なら `Some(MediaFormat::Avi)`、それ以外なら `None` を返す。この branch は MP4 fallback へ進まない。したがって len 12 以上で offset 4..8 が `ftyp` でも RIFF precedence が効き、unknown RIFF form type は MP4 へ fallback しない。
2. RIFF branch に入らず、`data.len() >= 8` かつ offset 4..8 が exact `ftyp` なら `Some(MediaFormat::Mp4)` を返す。len 8..11 で `RIFF` prefix と `ftyp` が競合しても、form type を検査できる長さがないため現行契約上は MP4 になる。
3. 上記以外、len 8 未満、unknown/short input、leading `free`/`wide`/`styp`/`mdat`/`moov` は `None` を返す。file extension は見ない。MP4 positive identification は leading `ftyp` box の type field（offset 4..8）だけである。先頭 12 byte 以降の trailing byteは結果に影響しない。

この boundaryを選ぶ理由は、fixed 12 byte prefix、2 つの length threshold、4 byte tag comparison、enum-sized outputだけの pure decision coreとして、AVI declaration correspondence、`strf` presence/copy/ordering、MP4-TABLE-005、AVI-TIME-004、PCM系が必要とする iterator、`Vec` copy、u128 division/remainder、multi-stream accumulation、codec/layout matrixに依存せず閉じられるからである。設計書が要求する boundary matrix と prefix-only 接続条件を、legacy proofの len 7/8/12 representative coverageより強く閉じられる。length-gate-only は tag byteと precedenceを残し、tag-only は short length complementを残すため完了境界にしない。

## 2. exact input domain

proof harness名は `format_signature_detection_bounded_exact`、runner group名は `format-signature` とする。proofの入力は次の 2 軸である。

1. `len: usize`。machine integer domain全体を symbolicに扱う。`len < 8`、`8 <= len < 12`、`len >= 12` の 3 length classを exact complement として検査する。lengthを制限する `kani::assume` は置かない。
2. `prefix: [u8; 12]`。各 byteは独立に full 8 bitを取り、signature decisionが観測しうる先頭 12 byteの全 2^96 bit patternを覆う。`RIFF`/`ftyp`/`WAVE`/`AVI ` tag byte、競合 bit、case、trailing spaceを代表値へ限定しない。

加えて同じ harness内で public shell matrixを検査する。symbolicな先頭 byte列に対し len 0,7,8,11,12,13 の public input sliceを `detect_format` へ渡し、独立した contract oracleと比較する。matrixは full length scalar proofの代替ではなく、public shellが input lengthと先頭 byte captureを正しく供給することの接続証拠である。

この分離は exact reparameterization である。productionの signature decisionが観測する最大領域は fixed 12 byteであり、index 12以降の trailing byteは結果に影響しない。任意の実 inputは、その lengthと、lenに応じて存在する先頭最大 12 byteによってこの boundaryの振る舞いが決まる。逆に任意の `len` と任意の 12 byte prefixの組合せは、その prefixを先頭に持つ inputとして構成できる。したがって length scalarと fixed prefixの Cartesian productは、signature decision coreの入力領域を漏れなく表す。

short inputでは fixed prefixの `len` 以降に実 input byteが存在しない。production shellは存在する byteだけを copyし、残りを fillerで埋める。proofでは filler位置も symbolicに扱うため、helperが fillerを誤って観測すれば独立 oracleとの差として検出される。`len >= 12` では 12 byteすべてが実 input由来であり fillerはない。`8 <= len < 12` では offset 4..8 は実 input由来だが offset 8..12 は fillerであり、length gateがその fillerを結果へ観測可能にしないことを証明する。`len < 8` では offset 4..8 も一部または全部が fillerであり、常に `None` であることを証明する。したがって fixed prefix modelは input narrowingではない。

trailing byteをモデル化しない根拠も同じである。production helperは fixed `[u8; 12]` だけを受け取り、型上 trailing byteを観測できない。public shellは最大 12 byteだけを captureする。これは trailing byteを concreteに固定する input narrowingではない。shell matrixの len 12/13 比較は少なくとも 1 byteの trailing non-dependenceを機械的に接続するが、任意長 shell copy composition全体の証明とは読み替えない。

## 3. 証明する性質

proofは symbolic `len` と symbolic fixed prefixに対し、production decision helperの total correctnessを検査する。検査する性質は少なくとも次の通りである。

- `len < 8` なら常に `None`。
- `8 <= len < 12` なら offset 4..8 が exact `ftyp` のときだけ `Some(MediaFormat::Mp4)`、それ以外は `None`。
- `len >= 12` かつ offset 0..4 が exact `RIFF` なら、offset 8..12 が exact `WAVE` のとき `Some(MediaFormat::Wav)`、exact `AVI ` のとき `Some(MediaFormat::Avi)`、それ以外は `None`。
- `len >= 12` かつ `RIFF` の場合は offset 4..8 が `ftyp` でも MP4 fallback しない。
- `len >= 12` かつ non-RIFF の場合だけ offset 4..8 `ftyp` を `Some(MediaFormat::Mp4)` に写す。
- `Wav`/`Avi`/`Mp4`/`None` の exact complement、tag byte order/case/trailing spaceの厳密性、byte-order/offsetずれや tagの drop/mis-mapがないこと。
- trailing byteと short input filler byteが結果に混入しないこと。
- 演算・indexingに overflow/underflow/panic がないこと。

public shell matrixでは len 0,7,8,11,12,13 の outputが contract oracleと一致することを検査する。len 8..11の `RIFF`+`ftyp` competition、len 12以上の RIFF precedence、len 12/13の trailing non-dependenceを接続証拠に含める。

この proofは fixed signature prefix modelと full machine integer length domainについて finite machine integer/byte domain全体の total correctness proofであり、representative samplingではない。fixed-prefix/scalar modelの成功を、任意長 body slice copy、全 caller integration、decode correctness、公開 API全体 no-panicの機械証明と読み替えない。

## 4. production 接続条件

proof modelは test-sideの signature判定複製だけではなく production pure helperを呼ぶ。最終設計は次の最小責務へ分離する。

- production fixed-prefix decision helper `signature_format_from_prefix(len, prefix)` が length class、RIFF precedence、form type mapping、ftyp recognition、None complementを決める。
- public `detect_format` は input lengthと先頭最大 12 byte captureから同じ helperを呼ぶ。
- helperは `len` と fixed `[u8; 12]` だけを見て決定し、trailing byte、allocation、decode、diagnosticを持たない。

この分離は製品契約を変えない。公開 API、`MediaFormat` enum、signature recognition semantics、caller-visible `None` 解釈、正常/異常系の振る舞い、対応形式を変えない。free/mdat/styp/moov 非認識、RIFF precedence、len 8/12 threshold、unknown RIFF form non-fallback、`WAVE`/`AVI `/`ftyp` byte exactness、trailing byte無視を変えない。signature recognitionを広げる、unknown RIFF formを MP4 fallbackへ変える、len thresholdを緩める、extensionを見る、decode dispatch behaviorを変える変更は行わない。

proof内 oracleは production helperを呼ばず、length class/RIFF precedence/ftyp mappingを独立に検査する。test-side implementationを proof対象にすり替えない。public shell captureは boundary matrix proof/assertionと deterministic regressionで接続し、shell gapを隠さない。任意長 inputから fixed prefixを作る shell copyの完全合成は local proofの外に残す。

## 5. assume と coverage

input narrowingは禁止である。`len`、prefix byte、`RIFF`/`WAVE`/`AVI `/`ftyp` tag byte、競合 bitを制限する `kani::assume` は置かない。fixed signature prefix length 12は現行 production layoutの最大観測領域であり、bounded modelの明示 scopeであって input narrowingではない。length classの complementは full `usize` 上で証明する。未証明の数学的事実を assumeしない。loopを含まない設計を優先し、unwinding assertionを無効化しない。

coverageは少なくとも次を落とさない。

- `len < 8` / `8 <= len < 12` / `len >= 12` の exact complement。
- len 0,7,8,11,12,13 の public shell boundary matrix。
- `RIFF` exact match/mismatch、`WAVE` exact match/mismatch、`AVI ` exact match/mismatch（ trailing spaceを含む）、`ftyp` exact match/mismatch。
- len 12以上の RIFF+unknown form+ftyp競合 non-fallback、len 8..11の RIFF+ftyp competition。
- tag byte order、case、trailing space、offsetずれ、drop/mis-map。
- trailing byteと filler byteの non-dependence。

## 6. 検証しない領域

次の領域はこの local proofの外に置き、ledgerの未証明領域に記録する。

- `decode_mp4`/`decode_avi`/`decode_wav`、MP4 box scan/sample table/PCM declaration/extraction、AVI chunk/movi/stream/timing/PCM、WAV chunk/layout/PCM/Range/Writer。
- partial ingest/capability observation、corruption/mutation/truncation whole-parser safety、I/O、allocation、diagnostic formatting、公開 API全体 no-panic。
- caller dispatch integration、probe/decode integration、任意長 public input shell copyの完全合成、file extension、container semantic validation。
- leading `free`/`wide`/`styp`/`mdat`/`moov` を認識する MP4 detection拡張。現行契約ではこれらは `None` のままである。

これらを local proof Passから上位保証へ転用しない。

## 7. decomposition候補の比較

- **fixed-prefix decision core + public shell boundary matrix proof（採用）**: full `usize` length scalarと symbolic fixed `[u8; 12]` prefixに対し、production decision helperの exact length class、tag byte mapping、RIFF precedence、None complement、算術/indexing安全性を検査する。同じ harness内で len 0,7,8,11,12,13の symbolic public inputに対する `detect_format` shell matrixを検査する。`Vec` accumulation、decode、body slice composition、I/O、allocation、fmt/alloc stubを持たない。signature decisionの独立した小さい境界に一致し、length/tag/precedence/trailing mutationを oracleが直接検出できる。harness名は `format_signature_detection_bounded_exact`、group名は `format-signature` とする。harnessは helperが `format.rs` にあるため `format.rs` の `#[cfg(kani)]` proof moduleに置く。
- **decision proofと shell matrix proofの 2 proof family（不採用）**: scalar decision coreと public shell matrixを分ける案。total correctnessと接続条件を失わない範囲では許容できるが、proofが小さく 1 harnessで実用予算に入る見込みが高い。registryと evidenceを増やす利益がないため分割しない。分割が必要になった場合も input narrowingには使わず、runner/docs/evidenceを同じ final proof setに同期する。
- **direct public API fixed-length matrixのみ（不採用）**: 既存 legacy proofを len 0,7,8,11,12,13へ拡張する案。production変更を避けられるが、full `usize` length scalarと prefix captureの exact reparameterizationを機械的に閉じず、matrix外長さの一般化をコード読解に残す。設計が求める boundary matrixには近いが、今回の pure decision exact proofとしては採用しない。
- **length-gate-only proof（不採用）**: `<8`/`8..11`/`>=12` の分岐だけを検査する案。最小だが tag byte、RIFF precedence、form type mappingを残す。
- **tag-only proof（不採用）**: 十分長 inputの `RIFF`/`WAVE`/`AVI `/`ftyp` mappingだけを検査する案。short length complementと precedenceを閉じない。
- **valid/invalid familyまたは全 length 0..=13 enumeration（不採用）**: exact complementを保てるなら可能だが、proof分割を入力narrowingに使わない。0..=13 enumerationは matrixより費用だけ増え、14以上の trailing non-dependenceを別途閉じない限り本質改善がない。
- **legacy `detect_format_signature_rules` harnessの改修（不採用）**: `formal_containers.rs` 内の legacy harnessを直接書き換える案。in-src production helper接続、selective assurance group、ledger上の replacement証跡を分かりにくくし、lease外ファイルを必要とする。legacy harnessはこの sliceでは retireも削除もしない。
- **decode dispatch / whole-container signature proof（不採用）**: `decode_mp4`/`decode_avi`/`decode_wav` や caller integrationを含む案。signature decisionの独立した小さい境界を超える。
- **MP4 recognition semantics拡張 proof（不採用）**: `free`/`wide`/`styp`/`mdat`先頭MP4の認識や複数box先読みを対象にする案。現行製品契約の変更を伴い、この sliceの対象外である。
- **MP4-TABLE-005/AVI-TIME-004/PCM系 proof（不採用）**: dynamic table、u128 timing、codec/layout matrix、multi-stream accumulationを伴う別の heavy Required propertyである。

採用案は KISS/YAGNI/True to the Codeで決める。`cfg(kani)`置換、fmt/alloc stub、solver総当たり、timeout延長だけの解決は禁止する。proofが通らなかった後で length domain、tag coverage、precedence coverageを縮める場合は weakeningとして扱い、Passとせず Coordinatorに askする。

## 8. runnerと予算

runnerは `AL_KANI_HARNESS_GROUP="format-signature"` で `format_signature_detection_bounded_exact`を選択する。registryは 34 harness（WAV 13 + MP4 scan 1 + MP4/AVI boundary 2 + AVI movi tag 1 + AVI movi composition 1 + AVI movi scan 1 + AVI stream kind 1 + AVI strh declaration 1 + AVI video strf declaration 1 + AVI audio strf declaration 1 + format signature 1 + legacy containers 10）となる。既存 verdictフィールド、既定全件挙動、fail-closed、WSL mirror、target配置、timeout意味論、group/subsetの排他意味論を変えない。全件実行はこの sliceの必須検証にしない。

単一 proof timeoutは canonical laneで `AL_KANI_TIMEOUT_MS=600000` とする。proofの設計予算は2分以内・4GiB未満、format-signature選択実行の合計実測は10〜20分を大幅に下回ることを目標とする。timeout、exit 0、CI greenを Proof Passにしない。Required Passが得られなければ proof/contractを弱めず、draft PR / Required NotRunとして報告する。

## 9. mutation detection plan

Able to Fail証拠として、signature semanticsを壊す一時 mutationを入れる。第一候補は RIFF precedenceを壊し、len 12以上で `RIFF` unknown formでも offset 4..8 `ftyp` へ MP4 fallbackさせる変更である。期待は format-signature proofに `VERIFICATION:- FAILED`が出ることである。legacy signature proofや unrelated boundary proofだけが Failする mutationを signature oracleの証拠とみなさない。diagnostic-only mutationは存在しないため証拠にしない。確認後に完全 revertし、最終状態で canonical laneを再実行する。検出されなければ mutationが不適切か oracleが弱いかを区別し、weakened Passにしない。

## 10. proptestとの関係

既存の mutation/truncation/no-panic proptestは弱めない。formal proofは full symbolic length scalarと fixed prefixの pure decision coreを exactに検査し、proptestは public integrationを統計的に検査する。len 0/7/8/11/12/13、RIFF+unknown form+ftyp競合の non-fallback、len 8..11の ftyp recognition、trailing byte無視、free/mdat/styp/moov非認識のうち既存 unit testで不足する境界だけを `format.rs` の deterministic unit testへ追加する。`formal_containers.rs`は変更しない。

## 11. RF還元判定

generic magic-byte decisionは形式として再利用可能に見える。ただし現時点の接続条件は AudioLensの `MediaFormat`、RIFF precedence、unknown RIFF non-fallback、leading `ftyp`限定、`None`解釈に結び付いており、実証済みの再利用契約として切り出せる段階ではない。本 sliceでは Ruviform Issueを起票せず、候補と理由を PR本文に記録する。

# AVI `strf` capture presence/copy/ordering bounded exact proof boundary analysis

対象は Issue #274 の AVI-STREAM-002 残件のうち、`strl` inner entry 列が `strf` chunk payload を `StreamHead.format` へどう capture するかという narrow boundary である。本書は solver 実験、runner 変更、production 変更に先立つ境界設計であり、full AVI-STREAM-002 declaration correspondence、missing-vs-empty product semantics、first `auds`/`vids` dynamic selection、`wb` scan/concatenation、任意長 body slice composition、nested walker composition、AVI-TIME-004、AVI-PCM、MP4-TABLE-005、MP4-PCM、WAV-PCM/Writer、unbounded scan、whole-parser corruption proof、legacy proof retire、完全 cost matrix を完了させるものではない。PR #282〜#293 の WAV/MP4/AVI/format signature proof は再設計せず、runner 変更の regression としてだけ再実行する。

## 1. 実コード確認と property の選定

現行 production `crates/al-media/src/avi.rs` の `StreamHead` は scalar declaration field に加えて `format: Vec<u8>` を持つ。`avi_layout` は `hdrl/strl` list の inner chunk を scan order で走査し、exact `strh` なら `parse_strh(cbody, &mut head)?` を実行して `has_strh = true` にし、exact `strf` なら `head.format = cbody.to_vec()` を実行し、それ以外の chunk は無視する。loop 終了後に `has_strh` が真のときだけ `streams.push(head)` する。複数 `strf` がある場合は後の exact `strf` body copy が先の copy を上書きする last-write-wins であり、`strh` と `strf` の相対順は最終 field に直接影響しない。`strf` の length や内容で container を reject せず、missing `strf` の初期 empty `Vec` と empty `strf` body copy は最終的にどちらも empty `Vec` になる。

capture された `format` は first `auds` decode path で `decode_waveformatex(&streams[audio_idx].format, ...)` へ渡され、first `vids` metadata/feature path でも `video_info` / `video::extract_features` へ渡る。したがって capture ordering/copy の誤りは audio/video 両方へ届く。一方で PR #290/#291/#292 は fixed-prefix declaration extraction/validation を証明したが、`strf` body がどの presence/copy/ordering 規則で `StreamHead.format` に入るかは証明していない。PR #289 の type-only table proof は `StreamHead.format`、payload bytes、`Vec` accumulation、iterator を入力に持たない。

この boundary を選ぶ理由は、full AVI-STREAM-002 declaration correspondence に必要な nested `Chunks` iterator、entry/error ordering、missing `strh` filter、複数 `strl` ordering、実 `Vec<StreamHead>` accumulation、任意長 body slice、missing-vs-empty product semantics を同時に引き込まずに、declaration chain に意味ある前進を作れる最小の未証明 shell だからである。capture core は exact 4 byte chunk id、presence、last-write-wins、non-`strf` non-interference、bounded body copy だけに依存し、codec/layout matrix、u128 timing、packet accumulation、multi-table consistency に依存しない。

## 2. exact input domain

runner group 名は `avi-strf-capture` とする。最終 proof family は `avi_strl_entry_classification_exact`、`avi_strf_capture_ordering_bounded_exact`、`avi_strf_materialize_bounded_exact` の 3 harness である。classification proof の入力は full `[u8;4]` id、ordering proof の入力は固定 3 件の symbolic class selector付き entry 列、materialize proof の入力は presence bitと bounded payload である。class selectorは full `u8` の剰余写像で 3 variantすべてを覆う冗長 encodingであり、input narrowing ではない。payload表現は次の 2 要素を持つ。

1. full `[u8;4]` id coverageは classification proofが担う。各 byte は独立に full 8 bitを取り、全 2^32 bit patternを覆う。exact `strf`、exact `strh`、それ以外の complementを代表値へ限定しない。ordering proofの class selectorも full `u8` を取り、`Strh`/`Strf`/`Ignore` の全 variantを覆う。
2. `len_selector: u8`。full 8 bit を取り、剰余写像で payload length `{0,1,2}` を作る。`kani::assume` で length を制限しない。selector encoding は冗長だが、宣告した length 0/1/2 の各値が representable であり、selector そのものは production input ではない。
3. `payload_storage: [u8; 2]`。各 byte は独立に full 8 bit を取る。実 payload length が 1 なら先頭 1 byte、2 なら先頭 2 byteだけが実在 byte であり、残りは filler である。filler 位置も symbolic に保ち、実長さを超える byte を誤って観測する実装を拒否する。

固定 3 件は theorem statement の明示的 bounded scope であり、証明が通らなかった後で選ぶ input narrowing ではない。3 件は no `strf`、single `strf`、複数 `strf` の overwrite、overwrite 後の non-`strf` non-interference を同一 finite composition で覆う最小実用件数である。payload length `{0,1,2}` も明示的 bounded scope であり、missing/empty capture 区別、1 byte copy、2 byte order/content preservation を exact に閉じる最小実用域である。len 3 以上の body copy、任意長 body slice、実 `Vec` growth、4 件以上の entry 列はこの theorem statement の外にある未証明領域であり、input narrowing で縮めた領域ではない。

この model は exact reparameterization である。production capture coreが観測するのは entry id と `strf` body sliceだけであり、capture reducerは payload byteを検査して分岐しない。non-`strf` entryでも同じ body shapeを渡すが、production capture helperは exact `strf` のときだけ bodyを観測するため、これは追加の仮定ではない。任意の bounded entry列は id列と各 payload length/contentで表され、逆に任意の symbolic id列と payloadは実 inner `strl` entry列として構成できる。len 3 以上や 4 件以上を含めない点だけが bounded scope である。

## 3. 証明する性質

proof family は固定 3 entry列に対し production entry classification/capture helperを合成した classification proof、ordering/presence proof、bounded bodyに対する production materialize helperの copy proofに分解する。分割は input narrowing ではなく、capture ordering stateと `Vec` allocation/copy の solver 責務を分ける exact decompositionである。検査する性質は少なくとも次の通りである。

- exact `strh`、exact `strf`、other が disjoint exhaustive であること。
- exact `strf` entryだけが capture を更新すること。
- `strf` が 1 度も無ければ capture presence が absent のままであること。
- 1 件以上の exact `strf` があれば、empty body を含めて capture presence が present であること。
- 複数 `strf` では scan order で最後の exact `strf` payloadが残ること。
- non-`strf` entry が capture を clear/overwrite/appendしないこと。
- materialized bytes は absent なら empty `Vec`、present なら captured bodyの length/content/byte orderを保存すること。
- overflow、underflow、out-of-bounds、panic がないこと。

empty `strf` presence と missing `strf` は capture seam内部では `Some(empty slice)` と `None` として区別できるが、現行 `StreamHead.format` へ materialize した後はどちらも empty `Vec` である。proof が検査するのは capture seamの presence/copy/orderingであって、missing-vs-empty を製品 observable semanticsとして区別する設計ではない。この最終非区別を missing-vs-empty product semanticsの機械証明と読み替えない。

## 4. production 接続条件

proof modelは test-side reducer複製だけではなく production helper/predicateを呼ぶ。最終設計は `avi.rs` に次の最小責務を作る。

- inner entry idを exact `Strh`/`Strf`/`Ignore` に分類する pure helper。
- exact `Strf` のときだけ capture tokenを last-write-winsで更新する generic capture helper。productionでは tokenは borrowed `&[u8]` であり、ordering proofでは bounded payload tokenに instantiateする。reducerは tokenを inspectしないため ordering logicを共有し、bounded byte copyは materialize proofと production-shell regressionが接続する。
- captureを `Vec<u8>` へ materializeする helper。

production `avi_layout` の inner loopは全 entryを scan orderで同じ capture helperへ通し、loop外で `head.format` を一度だけ materializeする。`head.format` への writeはこの materialize seamに限定する。copyを loop外へ移すことは overwritten `strf` bodyの allocation timingを変えるが、 immutable input bytesから最終的に得られる externally visible behavior、error precedence、partial ingest、degradation semanticsは変えない。allocation timingや overwritten body copy timingを製品契約とみなさない。

proof内 oracleは production helperを呼ばず、raw symbolic entriesから独立に exact `strf` occurrence、last index、presence、expected bytesを計算する。test-side implementationを proof対象にすり替えない。production shellが capture helperを scan orderで呼び、`head.format` への他の writeを持たないことは、unit/deterministic regressionとコード構造で接続条件として明示し、shell gapを隠さない。fixed composition内の helper呼出し順は production loopの scan orderと同じ 0,1,2 であり、逆順や代表性抽出ではない。

この分離は製品契約を変えない。公開 API、AVI acceptance、partial ingest、error文言/error precedence、missing `strf`と empty `strf` の最終非区別、対応形式、first stream selection、audio/video degradation、timing、PCM、feature extractionを変えない。`strf` を必須化する、missing/emptyで異なる error/degradationにする、first/last-winsを変える、unknown chunkを rejectする、`strh` なし streamの扱いを変える変更は行わない。

## 5. assume と coverage

input narrowingは禁止である。entry id、entry ordering、payload length class 0/1/2、payload byte、`strh`/unknown idを制限する `kani::assume` は置かない。fixed entry count 3と payload max len 2は theorem statementの明示的 bounded scopeであり、証明が通らなかった後で小さく選んだ input narrowingではない。未証明の数学的事実を assumeしない。loopは fixed 3 stepへ静的に展開し、unwinding assertionを無効化しない。loop coverage不足を warningのまま Passにしない。

coverageは少なくとも次を落とさない。

- full `[u8;4]` id bit patternによる exact `strh`/exact `strf`/other complement。
- no `strf`、single `strf`、duplicate `strf`、overwrite後の non-`strf` を含む ordering pattern。
- payload length 0/1/2 と存在する payload byteの全 bit pattern。
- absent presence、present empty capture、present non-empty capture。
- last-write-wins、non-`strf` non-interference、copy byte order/content。

expected oracleは production helperを呼ばず、raw idと payloadから独立に期待 captureを作る。proofが production helper同士の一致だけを検査して製品意味とのずれを検出できない状態にしない。

## 6. 検証しない領域

次の領域はこの local proofの外に置き、ledgerの未証明領域に記録する。

- `Chunks` iterator、`hdrl`/`strl` nested LIST walk、chunk boundary/padding、error precedence、`MediaError`文言、`strh` parse error、missing `strh` filter、複数 `strl` ordering、実 `Vec<StreamHead>` accumulation。
- `strh` scalar declaration correspondence、audio/video `strf` semantic validation、BITMAPINFOHEADER/WAVEFORMATEX decode suitability、first `auds`/`vids` selection、`wb` scan/concatenation。
- 任意長 body slice、len 3 以上の copy、実 `Vec` growth、4 件以上 entry列、nested walker composition、dynamic `Vec`/longer stream table。
- PCM decode、timing accumulation、video feature extraction、partial ingest、corruption/mutation/truncation whole-parser safety、I/O、allocation一般、diagnostic formatting、公開 API全体 no-panic、MP4、format signature semantics。

これらを local proof Passから上位保証へ転用しない。

## 7. decomposition候補の比較

- **fixed-entry capture ordering proof + bounded materialize copy proof（採用）**: 固定3件の symbolic entry列（full `[u8;4]` id + len 0..=2 symbolic payload）に対し、full id classification proofで exact tag complementを検査し、symbolic class列に対する production capture helperで presence/last-write-wins/non-`strf`非干渉を ordering proofで検査し、production materialize helperの bounded copy length/content/byte orderを別 proofで検査する。class selectorは full `u8` の剰余写像による冗長 encodingで、3 variantすべてを覆い、input narrowingではない。generic capture reducerは productionでは `&[u8]` tokenに monomorphizeされ、proofでは bounded payload tokenに instantiateする；reducerは tokenを inspectしないため ordering logicを exactに共有し、byte copy自体は materialize proofが閉じる。`Chunks` iterator、実 `Vec<StreamHead>`、body slice composition、I/O、fmt/alloc stubを持たない。今回の最小 pure/bounded boundaryに一致し、tag recognition、presence、ordering、copy mutationを oracleが直接検出できる。harness名は `avi_strf_capture_ordering_bounded_exact` と `avi_strf_materialize_bounded_exact`、group名は `avi-strf-capture` とする。harnessは helperが `avi.rs` にあるため `avi.rs` の `#[cfg(kani)]` proof moduleに置く。初回の single-proof 実験は `Vec` materializeを含む合成で 2分の設計予算を超える solver状態となったため verdict前に中止し、coverageを縮めずに責務分割した。
- **entry classification proofと copy/ordering proofの完全分離（不採用）**: exact tag classificationと capture ordering/copyを分ける案。total correctnessと接続条件を失わない範囲では許容できるが、proofが小さく 1 harnessで実用予算に入る見込みが高い。registryと evidenceを増やす利益がないため分割しない。分割が必要になった場合も input narrowingには使わず、runner/docs/evidenceを同じ final proof setに同期する。
- **tag-onlyまたは one-step capture proof（不採用）**: exact `strf` recognitionまたは1回の copyだけを検査する案。最小だが ordering/presence/last-write-winsを閉じず、今回の目的を満たさない。
- **concrete deterministic regressionのみ（不採用）**: unit/integration testで duplicate/missing/empty/non-`strf` を確認する案。低コストだが symbolic exact coverageがなく、この sliceの Required local proofにならない。補助接続証拠としては使う。
- **strh+strf declaration reducer proof（不採用）**: `strh` scalar field overwrite、has_strh、strf captureを同一 reducerで証明する案。full correspondenceに近いが、`strh` parse result/error precedence/field payloadを引き込み、入力次元と shell gapが増える。今回は `strf` captureに限定し、strh extractionは PR #290 の既証明 interfaceとして扱う。
- **direct `avi_layout` / real slice composition proof（不採用）**: byte slice、nested `Chunks` iterator、body slice/lifetime、error precedence、実 `Vec` を含む integration proof。True to the Codeには近いが、nested walkerと allocationを含む重い合成で、今回の narrow pure/bounded boundaryを超える。
- **arbitrary-length body/dynamic Vec copy proof（不採用）**: full usize lengthや `Vec` growthを含む案。copy bytesが全て出力に現れるため fixed-prefix exact reparameterizationが成立せず、任意長 body slice compositionの heavy残件に入る。今回は採用しない。
- **full AVI-STREAM-002 declaration correspondence proof（不採用）**: missing `strh` filter、entry/error ordering、複数 `strl`、実 stream table、`strh`+`strf` correspondenceを一度に証明する案。重要だが現時点では iterator/Vec/error precedence/missing-vs-emptyを抱える重い合成であり、この sliceでは採用しない。
- **missing-vs-empty semantic distinction proof（不採用）**: missing `strf`と empty `strf`を製品 observable semanticsとして区別する案。現行 `StreamHead.format: Vec<u8>` と downstream behaviorが区別を保持しないため、検証だけでなく製品設計判断が必要である。この sliceでは内部 capture seamの presenceだけを扱い、最終 observable semanticsを変えない。
- **first `auds`/`vids` selection、`wb` concatenation、AVI-TIME-004、PCM系 proof（不採用）**: dynamic stream/packet accumulation、u128 timing、codec/layout matrixを伴う別の heavy Required propertyなので採用しない。

採用案は KISS/YAGNI/True to the Codeで決める。製品実装を solverに合わせて不自然に複雑化しない。`cfg(kani)` 置換、fmt/alloc stub、solver総当たり、timeout延長だけの解決は禁止する。proofが通らなかった後で entry count、payload length domain、id coverage、presence/copy/ordering coverageを縮める場合は weakeningとして扱い、Passとせず Coordinatorに askする。

## 8. runner と予算

runner は `AL_KANI_HARNESS_GROUP="avi-strf-capture"` で `avi_strl_entry_classification_exact`、`avi_strf_capture_ordering_bounded_exact`、`avi_strf_materialize_bounded_exact`を選択する。registryは 37 harness（WAV 13 + MP4 scan 1 + MP4/AVI boundary 2 + AVI movi tag 1 + AVI movi composition 1 + AVI movi scan 1 + AVI stream kind 1 + AVI strh declaration 1 + AVI video strf declaration 1 + AVI audio strf declaration 1 + format signature 1 + AVI strf capture 3 + legacy containers 10）となる。既存 verdictフィールド、既定全件挙動、fail-closed、WSL mirror、target配置、timeout意味論、group/subsetの排他意味論を変えない。全件実行はこの sliceの必須検証にしない。

単一 proof timeoutは canonical laneで `AL_KANI_TIMEOUT_MS=600000` とする。proofの設計予算は2分以内・4GiB未満、avi-strf-capture選択実行の合計実測は10〜20分を大幅に下回ることを目標とする。timeout、exit 0、CI greenを Proof Passにしない。Required Passが得られなければ proof/contractを弱めず、draft PR / Required NotRunとして報告する。

## 9. mutation detection plan

Able to Fail証拠として、`strf` capture semanticsを壊す一時 mutationを入れる。第一候補は capture helperの last-write-winsを first-write-winsへ変え、既に capture presentのとき exact `strf` bodyで更新しない変更である。期待は avi-strf-capture proofに `VERIFICATION:- FAILED` が出ることである。boundary/movi/stream-kind/strh/strf declaration/format signature proofだけが Failする mutationを capture oracleの証拠とみなさない。diagnostic-only mutationは証拠にしない。確認後に完全 revertし、最終状態で canonical laneを再実行する。検出されなければ mutationが不適切か oracleが弱いかを区別し、weakened Passにしない。

## 10. proptest との関係

既存の mutation/truncation/no-panic proptestは弱めない。formal proofは固定3 entry列と payload length 0..=2 の capture coreを exactに検査し、proptestは public integrationを統計的に検査する。missing `strf`、empty `strf`、single capture、duplicate last-write-wins、unknown/non-`strf` non-interference、copy byte order/content、`strh` なし streamが pushされない現行 behaviorのうち既存 unit/integration testで不足する境界だけを `avi.rs` の deterministic unit testへ追加する。必要なら direct helper testだけでなく最小 AVI fixtureを通る production shell testを優先する。`formal_containers.rs` と `al-testkit` は変更しない。

## 11. RF 還元判定

chunk capture orderingや last-write-wins reducerは形式として再利用可能に見える。ただし現時点の接続条件は AudioLensの AVI `strl` entry classification、`strf` capture、empty/missing最終非区別、`StreamHead.format`、partial ingestに結び付いており、実証済みの再利用契約として切り出せる段階ではない。本 sliceでは Ruviform Issueを起票せず、候補と理由を PR本文に記録する。

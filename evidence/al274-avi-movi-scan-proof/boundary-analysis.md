# AVI-MOVI-003 movi scan collection bounded exact proof boundary analysis

対象は Issue #274 の AVI-MOVI-003 に残る finite iteration、`Vec` accumulation、body composition のうち、`movi` list 直下の固定 3 件の entry 列に対する scan collection 意味である。本書は solver 実験や runner 変更に先立つ境界設計であり、AVI-MOVI-003 全体、AVI-TIME-004、MP4-TABLE-005、MP4/AVI PCM を完了させるものではない。PR #284 の `avi_chunk_step_exact`、PR #286 の `avi_movi_packet_tag_exact`、PR #287 の `avi_movi_entry_decision_exact`、PR #282/#283/#285 の WAV/MP4 proof は再設計せず、runner 変更の regression としてだけ再実行する。

## 1. Property の選定

production parser は `movi` list 直下の entry を `Chunks::next` の yield 順に処理する。各 entry では `avi_movi_entry_decision` が packet、ignore、short-list reject、rec-list reject を返し、loop shell は reject で即座に対応する `MediaError` を返し、ignore で何も収集せず、packet で `MoviPacket` を mux 順に `Vec` へ追加する。first reject より後ろの entry は観測しない。複数 reject がある列でも、結果を決めるのは最も早い位置の reject である。

PR #287 は 1 entry の semantic decision を証明したが、複数 entry を通した収集順、first-reject 停止、ignore の非収集、reject precedence はまだ証明していない。これらは entry decision proof と full nested walker proof の間にある合成責務である。full finite iteration を直接対象にすると、`Chunks` iterator の状態、`Result`/`MediaError` precedence、body slice/lifetime、実 `Vec` allocation、entry 数の非有界性、outer RIFF scan を同時に扱う必要がある。この slice はその integration を避け、iterator/error/`Vec`/slice lifetime semantics を持ち込まずに閉じられる最小の scan collection composition を bounded exact proof で閉じる。

MP4-TABLE-005 は sample table accumulation と cross-table consistency、AVI-TIME-004 は u128 除算・remainder・複数 stream/packet accumulation、PCM 系は conversion matrix を持ち、この slice の最小単位ではない。stream existence/kind validity は pure な boundary を構成できるが、scan collection とは独立した property であり、ledger 上の composition boundary としては finite iteration 系残件が先である。signature 再構成は既存 proof が Pass しており marginal gain が小さい。完全 cost matrix、全体 mutation matrix、legacy retire はこの proof slice の後に整理する。

## 2. exact input domain

proof の入力は固定 3 件の direct child entry 列とする。各 entry は次の independent な入力を持つ。

1. `id: [u8; 4]` — full 2^32 bit pattern。digit tag、non-digit prefix、exact `LIST`、unknown chunk id、non-ASCII byte を sampling しない。
2. `list_type_view: Option<[u8; 4]>` — `None` と full `[u8; 4]` の全 bit pattern。`None` は direct child LIST body length が 4 未満で list type を読めない場合、`Some(bytes)` は body length 4 以上の先頭 4 byte を表す exact reparameterization である。

entry 数 N は 3 とする。N=2 でも first-reject 停止と最小限の ordering を検査できるが、reject 後の packet、連続 ignore の前後にある packet 順、複数 packet の compaction 順、2 件目以降の reject precedence が同時に現れる列を覆えない。N=3 はこれらを 1 つの finite composition で検査できる最小値である。N=4 以上は状態数を増やすだけで、この sub-boundary が対象にする ordering/stop/precedence の種類は増えない。固定 N は input narrowing ではなく bounded model の明示的 scope である。N+1 件以上の列、任意長の列、unbounded iteration termination は未証明領域に残す。

body tail、padding、chunk size、offset は scan collection core の入力ではない。production decision は direct child `LIST` の length-under-4 と先頭 4 byte だけを観測し、collection core は packet tag の収集・非収集と停止だけを扱う。packet body boundary、padding、offset 進行は PR #284 の `avi_chunk_step_exact`、tag digit/kind arithmetic は PR #286 の proof、1 entry の decision は PR #287 の proofが担う。`id != LIST` に symbolic list-type view を渡すのは production 到達外または unobserved の total extension であり、decision が list-type view に依存しないことを coverage を落とさず検査する。digit-only、valid LIST-only、writer fixture-derived entry、representative fourcc matrix への限定は禁止である。

## 3. 証明する性質

proof は固定 3 件の entry 列について、production `avi_movi_entry_decision` と production scan-step helper を合成した finite collection model の total correctness を対象にする。検査する性質は次の通りである。

- 収集される packet tag 列が、first reject より前の packet-decision entry の tag を入力順に並べたものと正確に一致する。
- first reject が結果の reject 種別（short list / rec list）を決定し、それ以降の entry が収集にも結果にも寄与しない。
- 複数 reject があるとき、最も早い index の reject kind が precedence を持つ。
- `Ignore` entry が収集へ何も追加しない。
- reject が無い列では全 packet entry が順序を保って収集される。
- packet/ignore/reject の per-entry 分類が exact complement であるという PR #287 の保証を、合成レベルで重複・欠落なく保つ。
- scan collection model と helper の計算で overflow、underflow、panic がない。

この proof は finite machine integer domain 全体に対する bounded exact proof であり、representative sampling ではない。scan collection proof Pass を unbounded finite iteration、任意長の entry 列、実 `Vec<MoviPacket>` accumulation 全体、body slice/lifetime composition、nested walker 全体、AVI-MOVI-003 全体、AVI-TIME-004 の機械証明と読み替えない。

## 4. 検証しない領域

次の領域はこの local proof の外に置き、ledger の未証明領域に記録する。

- `Chunks` iterator の yield 順、termination、error precedence、`MediaError` 文言生成。
- outer `hdrl`/`movi` scan、missing `movi`、RIFF size と outer LIST の関係。
- entry 数が 3 を超える列、任意長の列、unbounded iteration termination。
- 実際の `Vec<MoviPacket>` allocation/push、body slice/lifetime、packet body payload。
- nested non-rec LIST の内容、OpenDML `rec ` の構造的解釈。
- stream declaration（`strh`/`strf`）、stream number と実在 stream の対応、stream existence、kind validity/meaning matrix。
- timing accumulation、remainder、PCM/video decode、partial ingest。
- writer/corruption whole-call proof、I/O、allocation、diagnostic formatting、公開 API 全体 no-panic。
- MP4、signature 再構成。

これらを local proof Pass から上位保証へ転用しない。

## 5. production 接続条件

scan model は test-side の decision 複製ではなく、entry ごとに production `avi_movi_entry_decision` を呼ぶ。さらに、decision を loop effect へ写像する責務を最小の production scan-step helper `avi_movi_scan_step` に抽出し、production loop と scan model の両方が同じ helperを使う設計を採用する。helper は packet を collect、ignore を skip、reject を stop へ写像する。error 文言や `MediaError` 構築、body slice 作成、実 `Vec` への push は shell に残す。

この helper は `MoviEntryDecision` の自明な再命名ではない。entry decision は direct child entry の semantic 分類を担い、scan-step helper はその分類を loop の収集・継続・停止へ写す別責務を担う。variant が対応していても、分類と loop effect を別名で混線させず、proof が production loop で実際に使われる collect/skip/stop 写像を直接実行できる。公開 API、error 種別・文言、error precedence、正常/異常系の振る舞い、f64 精度、対応形式、partial ingest/capability semantics、stream/timing/video/audio 解釈は変えない。

scan model は 3 回の entry 処理を unroll した finite fold とする。stop 後は後続 entry を観測しない点、collect は入力順にだけ追加される点、skip は収集数を変えない点で、production loop shell の fold 構造に対応する。model は iterator、byte slice、`Result`/`MediaError`、実 `Vec` を持たないため、shell 全体の機械証明ではない。対応が弱い、または iterator 再設計が必要だと判明した場合は、広範な再設計へ進まず Coordinator に確認する。

## 6. assume と coverage

input narrowing は禁止する。各 entry の id/list-type view byte pattern を制限する `kani::assume` は置かない。N=3 固定は bounded model scope であり assume ではない。MP4-SCAN-003 の `valid_steps` のような model-scale assume も、この model では entry 列長が固定 3 で表せるため不要である。`Some`/`None`、rec/other list type、packet/non-packet、reject/ignore の complement を落とさない。未証明の数学的事実を assume しない。loop を使わず 3 件を unroll するため、unwind depth を下げて通す必要はない。

## 7. decomposition 候補の比較

### fixed-count bounded movi scan collection proof（採用）

固定 3 件の symbolic entry 列に対し、production `avi_movi_entry_decision` と production scan-step helper を合成する pure accumulation model で、収集順・first-reject 停止・ignore 非収集・reject precedence を 1 proof で検査する。`Vec`・slice・iterator・allocation・I/O を持たず、entry ごとの input domain は PR #287 と同じ scalar pure domain に留まる。MP4-SCAN-003 の bounded scan pattern と PR #287 の scalar pure boundary pattern の中間に位置し、PR #287 が数秒、MP4-SCAN-003 が約 36 秒・300 MiB 未満だった実測から、10 分・4 GiB の設計予算に入る見込みが高い。harness 名は `avi_movi_scan_collection_bounded_exact` とする。

### two-entry minimal ordering proof（不採用）

N=2 は cost が最小だが、3 件以上でしか現れない収集順と precedence の相互作用をカバーしない。特に、packet-ignore-packet の compaction 順、reject 後の packet 非収集、先頭ではない reject の precedence を同じ列で検査できない。N=3 が予算に入る見込みなので採用しない。

### mp4-scan 型 needs-more bounded model（不採用）

MP4-SCAN-003 は可変長 scan の valid step 数で場合分けし、4 件目の valid box を NeedsMore として排除した。movi scan collection は entry 列長を固定 3 として明示できるため、needs-more 形状や model-scale assume を増やす必要がない。assume の充足可能性と除外列の管理を避けられる固定長 model を採用する。

### combined chunk boundary + scan composition proof（不採用）

`avi_chunk_step` の boundary result と entry decision を合成すれば offset 進行を含む movi walker transition に近づく。しかし、offset/size arithmetic と error path を同じ proof へ戻し、PR #284/#287 で証明済みの層と重複する。この slice が閉じるべき未証明領域は boundary ではなく collection 意味なので採用しない。

### direct movi iterator/full Vec accumulation proof（不採用）

byte slice、`Result`、error 文言、実 `Vec<MoviPacket>`、body slice/lifetime、nested list semantics、outer scan を含み、integration proof へ広がる。最初の iteration 系 composition boundary proof としては大きすぎる。これらは後続の staged composition boundary に残す。

### stream existence/kind validity minimal pure boundary（fallback）

packet の stream number と実在 stream の対応、kind byte 分類を pure predicate として証明する案は独立に成立する。しかし scan collection とは別 property であり、ledger 上の composition boundary 優先順位では後ろである。scan collection proof の exactness/接続条件が維持不能と判明した場合の fallback 候補に留め、その場合は実装前に Coordinator へ確認する。

### AVI-TIME-004 accumulation proof（不採用）

`tick_advance`、remainder、複数 stream/packet clock を含み、u128 除算と multi-stream accumulation を伴う別の heavy Required propertyである。この slice の最小 scan collection boundary では採用しない。

## 8. runner と予算

runner は `AL_KANI_HARNESS_GROUP="avi-movi-scan"` で `avi_movi_scan_collection_bounded_exact` を選択する。registry は 29 件（WAV 13 + MP4 scan 1 + MP4/AVI boundary 2 + AVI movi tag 1 + AVI movi composition 1 + AVI movi scan 1 + legacy containers 10）になる。既存の per-harness elapsed ms、raw log、pass/fail/not_run、`/usr/bin/time -v` max RSS、lane-level `cost_summary`、fail-closed、WSL mirror、target 配置、timeout 意味論、group/subset 排他意味論は変えない。legacy proof を含む全件 lane はこの slice の必須検証にしない。

単一 proof timeout は canonical lane で `AL_KANI_TIMEOUT_MS=600000` とする。設計予算は proof 10 分以内・4 GiB 未満、avi-movi-scan 選択実行の合計実測 10〜20 分以内である。予算を超えた場合は Pass とせず、原因と再実行設計を ledger に残す。timeout、exit 0、CI green を Proof Pass にしない。

## 9. mutation detection plan

Able to Fail 証拠として、scan collection 意味を壊す一時 mutationを入れる。第一候補は production scan-step helper の packet collect を skip へ変える変更である。期待は avi-movi-scan proof に `VERIFICATION:- FAILED` が出ることである。generic boundary proof、tag proof、entry decision proof だけが Fail する mutationを scan collection oracle の証拠とみなさない。diagnostic payload だけの mutationは使わない。確認後に完全 revert し、最終状態で canonical laneを再実行する。検出されなければ mutation が不適切か oracle が弱いかを区別し、weakened Pass にしない。

## 10. proptest との関係

既存の mutation/truncation/no-panic proptest は弱めない。formal proof は固定 3 件の pure collection composition を exact に検査し、proptest は public integration を統計的に検査する。first-reject 後の packet 非収集、ignore 前後の packet 順序保持、複数 reject の先頭 precedence、reject 無し列での全 packet 順序収集のうち既存 unit/integration test で不足する境界だけを `avi.rs` の deterministic unit testへ追加する。`formal_containers.rs` は変更しない。

## 11. RF 還元判定

RIFF/AVI movi scan collection は形式仕様として汎用化できる可能性がある。ただし現時点の接続条件は AudioLens の OpenDML rejection、error 文言、stream 解釈、partial ingest、walker error precedence に結び付いており、実証済みの再利用契約として切り出せる段階ではない。本 slice では Ruviform Issue を起票せず、候補と理由を PR 本文に記録する。

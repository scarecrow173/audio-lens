# MP4 sample table chunk-map bounded proof の検証境界

対象は Issue #274 の MP4-TABLE-005 のうち、production `chunk_samples` が所有する stsc validation/per-chunk expansion と、production `append_chunk_samples` / `build_chunk_map` が所有する stsz 件数整合、stco 起点の sample file-offset 累積、`checked_add` overflow rejection である。本 slice は MP4-TABLE-005 全体の Pass 化ではない。

`.devin/dispatch-specs/al274-next-after-pr302.explore.json` はこの worktree に存在しない。Dispatch Spec に転記された slice design を operative input とし、行番号は dispatch 時点の実コードで再確認した。

## 1. Property と製品リスク

MP4 locating table は `stsc`・`stsz`・`stco`/`co64` から sample byte の file offset を決める。ここを誤ると、存在しない sample を読む、sample を重複/欠落させる、chunk 境界で byte をずらす、offset overflow を wrapped position として扱う、または破損 table を valid として後段 extraction へ渡す。

この slice が所有する Property は次の 2 つである。

1. **MP4-TABLE-005a stsc validation/per-chunk expansion。** bounded table domain で production `chunk_samples` が valid/invalid exact complement を判定し、valid input では各 chunk に正しい `(samples_per_chunk, sample_description_index)` を decode order で割り当てる。
2. **MP4-TABLE-005b chunk-map materialization。** production `append_chunk_samples` が chunk 内で stsz count、decode-order offset accumulation、`idx` 進行、`checked_add` overflow exact complement を持ち、production `build_chunk_map` が validation → per-chunk helper → final count check を正しく接続する。

proptest と unit test は production integration と deterministic regression を担う。proof は stts/ctts、box parser integration、unbounded/任意長 table、sample extraction/conversion、diagnostics、I/O、allocation、公開 API 全体 no-panic を証明済みとみなさない。

## 2. production seam と接続条件

proof harness は production `chunk_samples`、`append_chunk_samples`、`build_chunk_map` を直接呼ぶ。proof-only implementation として stsc expansion や offset accumulation を複製しない。独立 oracle は specification level の valid 条件・期待される per-chunk 割当・u128 widened offset oracle を表す。

当初は production refactor なしで `build_chunk_map` を直接検証する計画だった。しかし direct multi-chunk materialization は CBMC memory explosion で実用 budget に乗らなかったため、behavior-preserving refactor として `build_chunk_map` 内部の chunk 単位処理を `append_chunk_samples` へ抽出した。抽出後も `build_chunk_map` は同じ error 文言・error precedence・final count check を持ち、unit test は変更済み fixture で Pass した。

production 接続条件は次である。

- `parse_stbl` は `stsz`/`stsc`/`stco`/`co64` を parse して `SampleTable` を作り、`build_chunk_map(&table)` を codec 能力と無関係な構造 validation として呼ぶ。
- PCM extraction path は保存した `trak.table` に対して再び `build_chunk_map` を呼び、`extract_sample_bytes` へ渡す。
- `parse_stbl` の file-length gate、stsd index range、sample extraction、conversion は proof の外である。

## 3. exact input domain と proof family

### 3.1 validation/expansion family

validation family は次の bounded scalar domain を対象にする。

- chunk 数: 0..3
- stsc entry 数: 0..2
- `first_chunk`: 0..5
- `samples_per_chunk`: 0..4
- `sample_description_index`: 0..2

modulo selector は finite bounded domain の各値へ到達する exact reparameterizationであり、`kani::assume` による narrowing は置かない。entry length は 0/1/2、len2 は chunk 数 0/1/2/3 に structural split する。split の union は宣言した bounded cross product を覆う。valid 条件は specification level で次である。

- entry 数 0 のとき valid は chunk 数 0 の場合だけであり、expansion は空。
- entry 数 1 のとき、chunk 数は 1 以上、先頭 entry の `first_chunk == 1`、`spc != 0`、`sd != 0`。
- entry 数 2 のとき、上記に加えて第 2 entry の `first_chunk` が第 1 entry より大きく chunk 数以下、`spc != 0`、`sd != 0`。

accept 時は output length と各 chunk の `(spc, sd)` を独立 oracle と比較する。

### 3.2 per-chunk materialization helper family

materialization helper family は production `append_chunk_samples` を fixed shape で検証する。

- proof pair: `(spc, sizes.len) = (1,0), (1,1), (2,1), (2,2)`
- sample size: full `u32`
- chunk offset: full `u64`
- sample-description index: 1..2
- initial output/idx: empty/0

検証するのは、decode order の `(offset,size,sd)`、size exhaustion、overflow 時を含む exact accept/reject complement、partial mutation、`idx` 進行である。overflow 判定は u128 widened oracle を使う。`spc=2,len=2` は full-width size/offset を持つ最高 cost shapeで、単独 trial は Pass した。

### 3.3 end-to-end wrapper composition family

production `build_chunk_map` は次の最小 composition shapeで直接検証する。

- empty table accept
- 1 chunk × 1 sample の accept/overflow exact complement
- short sizes reject
- overlong sizes reject

これにより `chunk_samples` の成功、helper 呼出し、final count check の接続を検証する。multi-chunk end-to-end materialization は direct proof の memory explosion により residual NotRun scope に残す。

## 4. solver/memory explosion の記録と boundary 決定

これらは製品反例ではなく、verdict を返す前に手動停止した resource/cost trial である。raw 観測は `aborted-*.txt` と `actions.jsonl` に残す。

| trial | 観測 |
| --- | --- |
| 初期 unified dynamic validation, unwind8 | 約 4 分で RSS 約 3.2 GiB、中止 |
| 同 proof, unwind6 | 1m42s で RSS 17,764,508 KiB、中止 |
| len2 all-chunks combined | 約 1 分で RSS 14,092,868 KiB、中止 |
| len2/chunks3, unwind6 | 33 秒で RSS 11,881,524 KiB、中止 |
| len2/chunks3, unwind4 | Pass、30.104 s、2,587,916 KiB |
| direct chunks3 materialization | 2 分台で 8.7〜20.4 GiB 級、中止 |
| fixed-shape/full-width/direct-spc/table/count-match/smallnum/static-oracle/exact-length/combined diagnostics | いずれも 2chunk×2sample direct materialization で budget 超過または急増、中止 |
| combined lower-bound diagnostic | 50 秒で RSS 4,626,496 KiB、中止 |
| minimum positive end-to-end single sample | Pass、23.966 s、429,724 KiB |
| production helper spc2/len2 | Pass、61.147 s、372,560 KiB |
| canonical trial 1: len2 chunks0+1 combined, unwind4 | 28 秒で RSS 4,891,052 KiB、中止 |
| canonical trial 2: len2 chunks0, unwind4 | 10 秒で RSS 2,015,880 KiB、中止 |
| canonical trial 3: len2 chunks2, unwind4 | 95 秒で RSS 4,373,808 KiB、中止 |

この記録から、direct multi-chunk `build_chunk_map` proof は入力 bitwidth、oracle、Vec length、spc shapeを個別に絞っても実用 budget に乗らないと判断した。一方、production seam を chunk 単位へ分けると full-width u32/u64 を保持したまま最高 cost helper shapeが Pass した。したがって boundary weakening を隠すのではなく、multi-chunk end-to-end materializationを explicit residual NotRun とし、compositional proof familyを採用する。

また `#[kani::unwind]` を実到達反復より大きくすると、到達不能な追加 iteration の guarded `Vec::push` が分岐を増やす。validation len2 は chunks0/1/2 を unwind3、chunks3 を unwind4 とし、unwinding assertion は有効のままとする。

## 5. 健全性条件

- `kani::assume` は置かない。
- fmt/alloc/IO stub、proof 専用の production 置換、function contract は使わない。
- unwinding assertions は canonical invocation で有効なままとする。
- input narrowing ではなく structural split、production seam extraction、explicit residual NotRun で cost を扱う。
- timeout/resource abort は製品反例と混同しない。raw log、RSS、中止理由を残す。

## 6. runner と予算

runner の `mp4-table-chunk-map` group は validation 6 proof、helper materialization 4 proof、end-to-end composition 4 proofの計 14 proof を選択する。registry 総数は 56 件である。既存 env-detect、WSL mirror、per-harness sequential execution、timeout semantics、elapsed ms、raw log、`/usr/bin/time -v` max RSS、verdict schema、fail-closed、group/subset 排他、既定全件挙動は変えない。

`AL_KANI_TIMEOUT_MS=600000` を canonical budget とし、1 proof 10 分以内・ピーク RSS 4 GiB 未満を設計予算とする。legacy timeout proof を含む全件 lane はこの slice の必須検証にしない。

## 7. mutation plan

Able to Fail 証拠として、production `append_chunk_samples` の `checked_add` overflow rejection を除去し、加算を wrapping 化する semantic mutation を一時的に入れる。期待は `mp4-table-chunk-map` lane が exit 1 / verdict `fail` / 少なくとも 1 件の helper または end-to-end materialization proof で `VERIFICATION:- FAILED` になることである。diagnostic-only mutation、runner error、harness compile error、別 proof だけの失敗は検出証拠にしない。確認後は保存した byte-for-byte copy で復元し、hash と `cmp` が一致することを確認して post-revert canonical lane を再実行する。

## 8. 未証明領域

この slice で証明しない residual NotRun scope は次である。

- multi-chunk end-to-end materialization、spc 3/4、sizes len 3/4、3 件以上 sample
- MP4-TABLE-005 の stts/ctts cumulative DTS/CTS、edit list、stsz/stco box parser completeness、unbounded/任意長 table、4 件以上 chunk、3 件以上 stsc entry、full integer `first_chunk`/`spc`/`sd` table
- MP4-PCM-EXTRACT-007 の payload extraction、sample byte conversion、endianness/sign/float conversion、compressed codec semantics
- parser integration gap、diagnostics、I/O、allocation、dynamic `Vec` の任意長 growth、公開 API 全体 no-panic
- WAV residual、AVI residual、shared PCM conversion、Lean/runner tactical redesign、legacy proof retirement、完全 cost matrix

## 9. RF 還元判定

chunk-map arithmetic は形式的には container 共通の再利用候補だが、現行契約は AudioLens の MP4 error precedence、partial ingest、sample extraction boundary に結び付く。本 slice では Ruviform 側ファイルや Issue を変更せず、候補と理由を PR 本文へ記録する。ruviform#1015 の重複起票はしない。

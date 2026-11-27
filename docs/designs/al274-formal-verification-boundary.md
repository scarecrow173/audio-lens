---
id: "DESIGN-al274FormalVerificationBoundary"
type: "design"
status: "draft"
title: "Issue #274 media parser の Property 起点形式的検証境界設計"
created: "2026-11-26"
updated: "2026-11-27"
owners: []
relations:
  source: []
  changes:
    added: []
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements: []
  implemented-by: []
  depends-on:
    - "docs/specs/formal-verification.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/specs/formal-verification.md"
  refines: []
  refined-by: []
  derives-from:
    - "POLICIES.md"
    - ".agents/skills/verify-audio-lens/SKILL.md"
  derived-by: []
  verifies: []
  verified-by: []
  references:
    - "evidence/al274/verification.md"
    - "evidence/al274/residual/verdict.json"
    - "evidence/al274/stub-spike/README.md"
    - "evidence/al274-boundary/proof-inventory.json"
    - "evidence/al274-boundary/classification.json"
  defers: []
  deferred-by: []
---

# Issue #274 media parser の Property 起点形式的検証境界設計

## Context

Issue #274 の対象は WAV/MP4/AVI parser の形式的検証を、公開 API 名ではなく製品上重要な Property から再設計することである。本書はもともと proof 実装前の verification boundary/ledger 設計 slice の正本であり、後続 implementation slice の境界差分を追記する。WAV-RANGE-008、WAV-CHUNK-003/WAV-SCAN-004、MP4-BOX-002/AVI-CHUNK-001、MP4-SCAN-003、AVI-MOVI-003 tag identity、AVI-MOVI-003 movi entry composition、AVI-MOVI-003 movi scan collection、AVI stream existence/kind classification、AVI-STREAM-002 `strh` declaration field extraction/validation、AVI-STREAM-002 video `strf` BITMAPINFOHEADER declaration field extraction/validation の implementation slice では `crates/`、`tools/`、runner registry を変更済みである。ただし `evidence/al274-boundary/` は baseline のまま読み、ここへ遡って書き換えない。

設計 slice 時点の inventory は `evidence/al274-boundary/proof-inventory.json` / `.md` に台帳化した。その baseline では正本は `tools/run-formal-kani.mjs` の HARNESSES と `crates/al-media/tests/formal_wav.rs` / `formal_containers.rs` であり、canonical 21 proof、一時 spike 12 proof、proptest 13 件を別集合として扱った。baseline canonical のうち `detect_format_signature_rules` だけが Pass で、残り 20 件は 1500 秒/proof lane で timeout である。timeout、環境不足、exit 0、CI green は Proof Pass とみなさない。implementation slice 後の registry 正本は引き続き runner の HARNESSES であり、AVI strf declaration slice 時点では 32 件である。

分類と target structure の baseline 正本は `evidence/al274-boundary/classification.json` / `.md` である。本書はその設計判断、比較、推奨案、ユーザーへ返す open decision、および後続 implementation slice で確定した境界差分を記述する。WAV-RANGE-008 については 2026-11-26 の再設計 slice で local proof・runner group・実走証跡を `evidence/al274-proof-redesign/` に追加し、WAV-CHUNK-003/WAV-SCAN-004 については `evidence/al274-chunk-scan-proof/` に追加した。MP4-BOX-002/AVI-CHUNK-001 の境界と実走証跡は `evidence/al274-mp4-avi-proof/` に、MP4-SCAN-003 の境界と実走証跡は `evidence/al274-mp4-scan-proof/` に、AVI-MOVI-003 tag identity の境界と実走証跡は `evidence/al274-avi-movi-tag-proof/` に、AVI-MOVI-003 movi entry composition の境界と実走証跡は `evidence/al274-avi-movi-composition-proof/` に、AVI-MOVI-003 movi scan collection の境界と実走証跡は `evidence/al274-avi-movi-scan-proof/` に、AVI stream existence/kind classification の境界と実走証跡は `evidence/al274-avi-stream-kind-proof/` に、AVI-STREAM-002 `strh` declaration field extraction/validation の境界と実走証跡は `evidence/al274-avi-strh-declaration-proof/` に、AVI-STREAM-002 video `strf` BITMAPINFOHEADER declaration field extraction/validation の境界と実走証跡は `evidence/al274-avi-strf-declaration-proof2/` に追加する。lazy diagnostic 実装、残り Required proof、完全な cost matrix は後続 slice に残す。

## 設計原則

1. **Property と製品リスクを先に定義する。** 「API X が panic しない」ではなく、chunk 範囲外を読まない、走査が進行して終了する、Range 計算が overflow しない、PCM conversion が音を変えない、writer/reader の解釈が一致する、という検証単位に分解する。
2. **要求状態と verdict を分離する。** Required/Deferred/NotApplicable と Pass/Fail/NotRun/Unsupported を独立に記録する。現行 timeout は NotRun であり、未証明領域として管理する。
3. **保証種別を混ぜない。** proptest の Property Tested、Kani bounded exhaustive、固定長 integration proof、mutation detection evidence を同じ保証として扱わない。bounded proof を無制限入力や公開 API 全体 no-panic の証明と読み替えない。
4. **True to the Code に接続する。** pure helper/model の proof は、実装がその helper を呼ぶこと、呼出側事前条件を満たすこと、事後条件から上位契約を導くことを接続条件として記録する。
5. **CI は Daily Full CI 集約・非ブロッキングを前提にする。** PR 必須の blocking Kani lane は新設しない。ただし Required proof の NotRun は、対応する機能 slice の最終受入を止める。

## 現行 inventory の要点

| 集合 | 件数 | 現行の意味 | 設計上の扱い |
| --- | ---: | --- | --- |
| canonical Kani proof | 21 | runner に登録された正式 proof 集合。1 Pass、20 timeout | target structure と照合して分類。現行形の whole-call symbolic proof は Required proof としない |
| spike Kani harness | 12 | stub ablation の再現資産。runner 未登録 | canonical・集計・証跡から分離して維持。新規 assurance pack に混ぜない |
| proptest | 13 | 常時実行できる統計的 property test。既存証跡では 13/13 Pass | whole-parser integration・round-trip・corruption coverage の primary assurance として維持する |

現行 proof の主な問題は、parser、allocation、診断 formatting、testkit writer、container decode を一つの symbolic end-to-end proof に入れていることである。残存支配因は pointer/slice validity instrumentation、Vec/RawVec、slice bounds panic machinery、parser branch、fmt machinery であり、メモリ封じ込めよりも symex 時間と固定 proof floor が問題である。

## 推奨 target verification structure

詳細な Property 一覧、必要保証、粒度、境界、手法、実行予算、接続条件、未証明領域は `evidence/al274-boundary/classification.json` の `target_verification_structure` を正本とする。ここでは構造だけを示す。表中の要求状態は Required/Deferred/NotApplicable の厳密な語彙とし、proptest primary で Kani 側が Deferred/NotApplicable な配分は手法列と `assurance_allocation` に記録する。

### WAV

WAV は次の層に分ける。

| 層 | 代表 Property | 推奨手法 | 要求状態 |
| --- | --- | --- | --- |
| 形式判定・RIFF header | signature、RIFF size/length 整合 | Kani local proof + proptest | Required |
| chunk walker | chunk boundary、padding、scan progress/termination | Kani local proof + structural mutation proptest | Required |
| layout/PCM | format tag、bits、channels、rate、block_align、byte_rate、PCM conversion、writer encode | Kani local proof + round-trip proptest | Required |
| Range 計算 | frame count、valid/invalid range、byte offset、sample slice exactness | Kani local proof + integration proptest | Required |
| writer/header | parameter gate、block_align/byte_rate overflow、header field、padding/backpatch | proptest primary + selective local/concrete proof | Required |
| end-to-end | writer/reader round-trip、corruption no-panic | proptest primary。whole-call Kani は Deferred | Required |

チャンク境界、走査進行性、Range 計算は独立した局所 proof として設計する。これらは公開 API 全体 no-panic ではない。たとえば Range proof の成功は、`read_wav_range` が range helper の事前条件を満たし、slice index を正しく計算することを示すだけであり、layout walk、I/O、allocation、PCM decode、diagnostic の安全性を示さない。

### WAV-RANGE-008 implementation boundary(2026-11-26 再設計)

PR #281 は exact valid-range proof が 300012ms timeout で Required NotRun だった。再設計 slice では、range arithmetic の exact input domain を constructor-admitted layout 全体に置き、decode 可能な encoding へ絞らない。layout invariant は、非 zero `block_align`、u32 data length、frame-aligned data、`total_frames = data_len / block_align`、data extent が RIFF file length `u32::MAX + 8` に収まることである。public frame index は u64 全域とし、valid predicate `start < end && end <= total_frames` の下で u32 start/end へ zero-extension するのは exact reparameterization として扱う。

proof 対象は valid range の受理、frame count、byte offset、byte length、exclusive byte end、data extent 内 containment、overflow/underflow/panic の不存在、および invalid exact complement の拒否である。I/O short read、allocation、PCM decode、diagnostic formatting、公開 API 全体 no-panic は対象外に残す。compact error payload の `total_frames` は proof に含めるが、人間向け formatting は含めない。

PR #281 の timeout は wide bitvector の nonlinear multiplication monotonicity/containment に集中した。最終案では endpoint product form を捨て、production helper を constructor-faithful count product form にする。valid input は `start32`、正の `frame_count32`、`tail_frames32` で構成し、`end = start + frame_count`、`total = end + tail` とする。data length は 3 つの byte segment `start * block_align`、`frame_count * block_align`、`tail * block_align` の和として表す。これは constructor invariant と整数の distributivity による exact reparameterization であり、代表値 sampling ではない。containment は nonlinear monotonicity ではなく linear segment equality `byte_end + tail_bytes = data_offset + data_len` で検査する。詳細な候補比較と coverage 証明は `evidence/al274-proof-redesign/boundary-analysis.md` を正本とする。

runner は `AL_KANI_HARNESS_GROUP="wav-range"` で `wav_range_plan_valid_ranges_exact` と `wav_range_plan_invalid_ranges_error` を選択する。各 harness の elapsed ms、raw log、pass/fail/not_run、`/usr/bin/time -v` が利用できる環境での max RSS を verdict に残す。初期に検討した `block_align` bit width exact finite partition は、constructive decomposition が exact coverage を保ったまま単一 proof で実用予算に入ったため採用しない。

### WAV-CHUNK-003/WAV-SCAN-004 implementation boundary(2026-11-26 chunk/scan slice)

chunk boundary helper の入力は `(offset: u64, file_len: u64, size: u32)` の全数値領域とする。production scan は RIFF size field の制約下で動くが、constructor-admitted RIFF file length domain へ proof を縮めない。full u64 file/offset domain は、boundary arithmetic が total に定義でき、入力を広げる方向で保証を弱めないため採用する。`offset > file_len` は production 到達外だが、total extension として terminal trailing 側へ分類する。

証明対象は 1 回の chunk header interpretation の total correctness である。valid なら `body_start = offset + 8`、`body_end = body_start + size`、odd-sized chunk の pad、`next_offset`、file length 内 containment、少なくとも 8 byte の進行、残余量の厳密な減少、overflow/underflow/panic の不存在を検査する。invalid exact complement は size overflow、chunk extends past end、missing pad、terminal exact complement は header 不足に対応し、1..7 byte の端数を trailing byte 判定へ分類する。I/O、allocation、PCM decode、diagnostic formatting、fmt/data semantic、duplicate 判定、公開 API 全体 no-panic は proof の外に置く。

scan proof の初期 offset は production と同じ 12、`file_len >= 12` は RIFF/WAVE header 確認後に scan へ入る exact constructor precondition とする。small scan model は terminal または invalid に達するまでに高々 3 valid chunk を受理する入力列を対象にし、full u64 file length と full u32 size を保つ。4 valid chunk 以上は未証明領域に残し、bounded model の成功を unbounded scan termination と読み替えない。one-step measure decrease は exact proof の一部とするが、任意回数の帰納的合成を Kani が機械的に証明したとは扱わない。

採用する分解は single-step total correctness proof と finite unrolled small scan model の組合せである。valid/invalid proof family への分割、function contract、loop invariant、verified stub、fmt/alloc stub、`cfg(kani)` による製品実装置換は採用しない。詳細な候補比較、assume 分類、接続条件、coverage は `evidence/al274-chunk-scan-proof/boundary-analysis.md` を正本とする。

production `wav_layout` は scan position helper で header availability/terminal を判定し、header が使える場合だけ 8 byte header を読んで同じ boundary helper へ `(offset, file_len, size)` を渡す。`Advance` の `body_start` を fmt body read、`next_offset` を scan 更新に使い、invalid 分類は呼出側で既存 `MediaError::malformed` 文言へ写像する。chunk id 解釈と duplicate 判定は production loop に残すが、boundary/offset 計算を複製しない。

### MP4

MP4 は次の層に分ける。

| 層 | 代表 Property | 推奨手法 | 要求状態 |
| --- | --- | --- | --- |
| signature | detect_format の MP4 判定 | Kani local proof + proptest | Required |
| box walker | size32/size64/size0、overflow、limit、child scan progress | Kani local proof | Required |
| sample table | stts/ctts/stsc/stsz/stco の offset/count/time arithmetic | Kani local proof + round-trip proptest | Required |
| PCM declaration/extraction | codec classification、sample width/endian/sign conversion | proptest + Kani local/concrete matrix | Required |
| structural facts | required box、skipped boxes、declared facts | proptest primary | Required |
| partial ingest | compressed/missing audio を error にしない | proptest primary | Required |
| end-to-end | writer/reader round-trip、corruption no-panic | proptest primary。whole-call Kani は Deferred | Required |

### AVI

AVI は次の層に分ける。

| 層 | 代表 Property | 推奨手法 | 要求状態 |
| --- | --- | --- | --- |
| RIFF/LIST walker | chunk/LIST boundary、padding、region limit | Kani local proof | Required |
| stream declaration | strh/strf の kind/scale/rate/start/format | proptest primary + selective local matrix | Required |
| movi packet | NNdc/NNwb id、stream number、kind、body boundary | Kani local proof | Required |
| timing | movi_order の累積、remainder、stream 独立 clock | Kani local proof + integration proptest | Required |
| PCM | WAVEFORMATEX validation/conversion | Kani local proof + proptest。WAV と共有可能なら接続条件を記録 | Required |
| video/partial ingest | video metadata、missing/undecodable auds | proptest primary | Required or NotApplicable |
| end-to-end | writer/reader round-trip、corruption no-panic | proptest primary。whole-call Kani は Deferred | Required |

### MP4-BOX-002/AVI-CHUNK-001 implementation boundary(2026-11-26 mp4/avi boundary slice)

MP4 boundary helper の入力は `(at: u64, limit: u64, size32: u32, largesize: u64)` の全数値領域とする。production index は u64 scalar に写し、slice 長や enclosing box による事後 narrowing はしない。`at > limit` は production 到達外の total extension として truncated base header 側へ分類する。`largesize` は size32 == 1 の branch でだけ意味を持ち、他 branch で symbolic に渡すのは coverage を保存する exact reparameterization である。

MP4 proof は 1 回の box header interpretation の total correctness を対象にする。valid なら size32 == 0 が enclosing region end まで、size32 == 1 が 16 byte largesize header、size32 >= 8 が 8 byte base header を持ち、body_start/body_end/end-of-box が limit 内、checked arithmetic で overflow/underflow/panic がなく、少なくとも header 分進行して残余量を厳密に減らす。invalid exact complement は truncated base/largesize header、size smaller than header、largesize smaller than 16、size overflow、region end overrun である。

AVI boundary helper の入力は `(offset: u64, region_len: u64, size: u32)` の全数値領域とする。offset は enclosing region 先頭からの相対位置であり、最初の valid chunk で `body_start = 8` となる同値式を使う。`offset > region_len` は production 到達外の total extension として trailing bytes 側へ分類する。proof は empty/exact end、1..7 byte trailing、valid chunk、invalid chunk の exact complement、body end、odd-sized chunk の pad、pad を含む next offset、region limit、少なくとも 8 byte の進行、残余量減少を対象にする。

採用する分解は両者とも single-step total correctness proof である。valid/invalid proof family、finite unrolled scan model、measure-decrease 単独 proof、function contract、loop invariant、verified stub、fmt/alloc stub、`cfg(kani)` による製品実装置換は採用しない。one-step measure decrease は proof に含めるが、無制限 scan termination の機械証明とは読み替えない。MP4-SCAN-003、MP4-TABLE-005、MP4-PCM 系、AVI-MOVI-003、AVI-TIME-004、AVI-PCM 系、signature 再構成、writer/corruption whole-call proof は未証明領域または後続 slice に残す。詳細な候補比較、assume 分類、接続条件、coverage は `evidence/al274-mp4-avi-proof/boundary-analysis.md` を正本とする。

production `box_header` は size32/largesize/tag の byte 読取と error 文言生成だけを shell に残し、boundary arithmetic を `mp4_box_boundary_step` へ委譲する。top-level file scan と全 nested `child_boxes` は既存どおり `box_header` を通る。production `Chunks::next` は region-relative offset/region length/size を `avi_chunk_step` へ渡し、valid 結果から body slice・pad を含む remainder・次 offset を導く。RIFF body、`hdrl`、`strl`、`movi` の nested region は同じ iterator を使う。tag/id 解釈や `MediaError` 文言生成は shell に残し、boundary/offset 計算を複製しない。

runner は `AL_KANI_HARNESS_GROUP="mp4-avi-boundary"` で `mp4_box_header_step_exact` と `avi_chunk_step_exact` を選択する。registry は 25 件となり、per-harness elapsed ms/raw log/pass/fail/not_run/max RSS に加えて、選択 proof 数、verified proof 数、total elapsed ms、max observed RSS KiB、計測可否を持つ additive `cost_summary` を verdict に残す。既定全件挙動、fail-closed、WSL mirror、target 配置、timeout 意味論、group/subset 排他意味論は変えない。legacy proof を含む全件 lane はこの slice の必須検証にしない。

### MP4-SCAN-003 implementation boundary(2026-11-27 mp4 scan slice)

MP4 scan proof の input domain は `start<=end` の constructor/caller precondition を持つ full u64 region と、高々 4 回分の full u32 size32/full u64 largesize 列である。`start<=end` は private scan caller の invariant であり、top-level scan、length check 済みの sample entry/stsd、valid box body range 以外からは scan へ入らない。`start>end` は production 到達外の total extension として scan position helper では trailing 側へ分類するが、proof domain には含めない。`largesize` は size32==1 の branch でだけ観測される exact reparameterization である。

small scan model は terminal または invalid に達するまでに高々 3 valid box を受理する入力列を exact に覆う。4 件目の symbolic header は 3 valid box 後の invalid を含めるために使い、4 valid box 以上を受理する列は未証明領域に残す。proof は empty region の即時 complete、exact end と 1..7 byte trailing の区別、valid step の `next_offset=body_end`、少なくとも header 分の strict progress、残余量減少、first invalid 終端、boundary containment、checked arithmetic を検査する。bounded model の成功を無制限 scan termination の機械証明と読み替えない。

採用する分解は finite unrolled scan model に、最小の production scan position/next-offset transition helper を接続する構成である。production top-level scan と nested `child_boxes` は `mp4_scan_position` で terminal を判定し、`box_header` が返す valid box の `next_offset` は `mp4_box_scan_next_offset` から導く。tag 解釈、error 文言生成、`Vec<BoxHeader>` 収集は shell に残し、boundary/offset 計算を複製しない。single-step measure のみ、direct `child_boxes` proof、function contract/loop invariant は採用しない。詳細な候補比較、assume 分類、接続条件、coverage は `evidence/al274-mp4-scan-proof/boundary-analysis.md` を正本とする。

runner は `AL_KANI_HARNESS_GROUP="mp4-scan"` で `mp4_box_scan_progress_bounded_exact` を選択する。registry は 26 件となる。MP4/AVI boundary、WAV chunk/scan、WAV range の各 group は regression として再実行し、それらの proof 設計は変更しない。

### AVI-MOVI-003 tag identity implementation boundary(2026-11-27 avi movi tag slice)

movi packet tag proof の input domain は full `[u8; 4]` chunk id とする。digit prefix、kind byte、unknown/opaque suffix、non-digit prefix を sampling せず全 2^32 bit pattern で扱い、input narrowing assume は置かない。`id[2]`/`id[3]` は意味分類せず opaque kind byte として保持する。現行製品契約が unknown suffix を拒否せず保持するため、proof 側に kind validity matrix を追加しない。

proof は 1 回の tag interpretation の total correctness を対象にする。先頭 2 byte が両方 ASCII digit のときだけ受理し、`stream_no=(id[0]-b'0')*10+(id[1]-b'0')`、stream number 0..=99、kind byte の順序保持、受理/非受理の exact complement、overflow/underflow/panic の不存在を検査する。non-digit prefix は kind suffix によらず非 packet とする。

採用する分解は single total correctness tag proof である。valid/invalid proof family、combined chunk boundary + movi tag composition proof、direct movi iterator proof、AVI-TIME-004 accumulation proof は採用しない。`Vec` 展開、u128 timing 除算、remainder、複数 stream/packet 累積、codec matrix を必要としない scalar pure contract なので、小さい input domain を分割で増やさず 1 proof で exact complement を検査する。詳細な候補比較、assume 分類、接続条件、coverage は `evidence/al274-avi-movi-tag-proof/boundary-analysis.md` を正本とする。

production `movi` loop は `Chunks::next` が返した同じ inner chunk id を `avi_movi_packet_tag` へ渡し、digit 判定、stream number 計算、kind 抽出を複製しない。`Chunks::next` の boundary/offset 計算は既存どおり `avi_chunk_step` を使い、body slice、padding、error precedence、`MediaError` 文言生成、OpenDML `rec ` rejection、`Vec<MoviPacket>` 収集は shell に残す。tag proof Pass を nested movi walker 全体、AVI-MOVI-003 全体、AVI-TIME-004 の機械証明と読み替えない。

runner は `AL_KANI_HARNESS_GROUP="avi-movi-tag"` で `avi_movi_packet_tag_exact` を選択する。registry は 27 件となる。MP4/AVI boundary、MP4 scan、WAV chunk/scan、WAV range の各 group は regression として再実行し、それらの proof 設計は変更しない。legacy proof を含む全件 lane はこの slice の必須検証にしない。

### AVI-MOVI-003 movi entry composition implementation boundary(2026-11-27 avi movi composition slice)

movi entry composition proof の input domain は full `[u8; 4]` chunk id と exact list-type view `Option<[u8; 4]>` とする。`None` は direct child LIST body length 4 未満、`Some(bytes)` は body length 4 以上の先頭 4 byte を表す exact reparameterization である。body tail、padding、chunk size、offset はこの decision core が観測しない入力なのでモデル化しない。`id != LIST` に symbolic list-type view を渡すのは production 到達外または unobserved の total extension であり、decision が list-type view に依存しないことを coverage を落とさず検査する。input narrowing assume は置かない。

proof は 1 entry の semantic decision の total correctness を対象にする。exact `LIST` かつ list type 不在なら short-list reject、exact `rec ` なら rec-list reject、`rec ` の byte complement なら ignore、`LIST` 以外なら list-type view に依存せず digit tag 受理時だけ packet、それ以外は ignore とする。packet/ignore/short-list reject/rec-list reject の exact complement、stream number 0..=99 と kind byte preservation、overflow/underflow/panic の不存在を検査する。packet body boundaryは PR #284 の `avi_chunk_step_exact`、tag digit/kind arithmetic は PR #286 の `avi_movi_packet_tag_exact` が担う別層である。

採用する分解は single total correctness entry decision proof である。LIST/non-LIST exact complement family、bounded finite unrolled movi scan proof、combined chunk boundary + entry composition proof、direct movi iterator/full nested composition proof、AVI-TIME-004 accumulation proof は採用しない。finite iteration、entry ordering、`Vec<MoviPacket>` accumulation、body slice/lifetime、nested walker 全体をこの proof の保証と読み替えない。詳細な候補比較、assume 分類、接続条件、coverage は `evidence/al274-avi-movi-composition-proof/boundary-analysis.md` を正本とする。

production `movi` loop は `Chunks::next` が返した同じ inner chunk id と、`LIST` のときだけ body から作った list-type viewを `avi_movi_entry_decision` へ渡す。loop shell は short list type と exact `rec ` を現行どおり `MediaError` へ写像し、boundary/offset 計算、body slice、padding、iterator error precedence、`Vec<MoviPacket>` 収集を残す。decision helper は error 文言や `MediaError` を構築せず decision/tag だけを返す。公開 API、error precedence、正常/異常系の振る舞い、stream/timing/video/audio 解釈を変えない。

runner は `AL_KANI_HARNESS_GROUP="avi-movi-composition"` で `avi_movi_entry_decision_exact` を選択する。registry は 28 件となる。AVI movi tag、MP4/AVI boundary、MP4 scan、WAV chunk/scan、WAV range の各 group は regression として再実行し、それらの proof 設計は変更しない。legacy proof を含む全件 lane はこの slice の必須検証にしない。

### AVI-MOVI-003 movi scan collection implementation boundary(2026-11-27 avi movi scan slice)

movi scan collection proof の input domain は固定 3 件の direct child entry 列とする。各 entry は full `[u8; 4]` chunk id と exact list-type view `Option<[u8; 4]>` を独立に取り、entry composition slice と同じ exact reparameterization を使う。input narrowing assume は置かない。N=3 は packet-ignore-packet の compaction 順、reject 後の packet 非収集、先頭以外の reject precedence を同じ列で検査できる最小値である。N 固定は bounded model の明示的 scope であり、N+1 件以上の列、任意長の列、unbounded iteration termination は未証明領域に残す。

proof は production `avi_movi_entry_decision` と production scan-step helper `avi_movi_scan_step` を 3 件の symbolic entry 列へ合成した pure accumulation model の total correctness を対象にする。first reject より前の packet tag が入力順に収集されること、first reject が結果の reject 種別と停止位置を決めること、複数 reject で最も早い index が precedence を持つこと、ignore が収集へ寄与しないこと、reject 無し列で全 packet が順序を保つこと、per-entry exact complement、overflow/underflow/panic の不存在を検査する。expected oracle は production decision/helper を呼ばず raw id/list-type view から独立に分類する。

production `movi` loop は `avi_movi_scan_step` を通して decision を collect/skip/stop へ写像する。scan-step helper は semantic 分類の再命名ではなく、entry 分類を loop effect へ写す責務を持ち、proof model と production loop が同じ写像を使う。`MediaError` 文言、body slice、padding、iterator error precedence、実 `Vec<MoviPacket>` 収集は shell に残す。公開 API、error precedence、正常/異常系の振る舞い、stream/timing/video/audio 解釈を変えない。

採用する分解は fixed-count bounded movi scan collection proof である。two-entry minimal ordering proof、mp4-scan 型 needs-more bounded model、combined chunk boundary + scan composition proof、direct movi iterator/full Vec accumulation proof、AVI-TIME-004 accumulation proof は採用しない。stream existence/kind validity は scan collection と独立した fallback 候補に留める。詳細な候補比較、assume 分類、接続条件、coverage は `evidence/al274-avi-movi-scan-proof/boundary-analysis.md` を正本とする。

runner は `AL_KANI_HARNESS_GROUP="avi-movi-scan"` で `avi_movi_scan_collection_bounded_exact` を選択する。registry は 29 件となる。AVI movi composition、AVI movi tag、MP4/AVI boundary、MP4 scan、WAV chunk/scan、WAV range の各 group は regression として再実行し、それらの proof 設計は変更しない。legacy proof を含む全件 lane はこの slice の必須検証にしない。

### AVI stream existence/kind classification implementation boundary(2026-11-27 avi stream kind slice)

stream existence/kind classification proof の input domain は固定 3 件の declared stream type table `[[u8; 4]; 3]`、full `usize` stream number、full `[u8; 2]` kind とする。各 stream type は全 2^32 bit pattern、kind は全 2^16 bit pattern、stream number は machine integer domain 全体を symbolic に扱い、input narrowing assume は置かない。production tag proof が `stream_no <= 99` を保証していても 0..=99 へ縮めない。N=3 固定は input narrowing ではなく bounded model の明示的 scope であり、leading/middle/trailing position、mixed `auds`/`vids`/other、in-range/out-of-range lookup を 1 つの finite composition で覆う最小値である。より長い stream table、任意長 `Vec<StreamHead>`、strl ordering、missing `strh` を含む declaration correspondence は未証明領域に残す。

proof は 1 packet の post-scan stream/kind classification の total correctness を対象にする。lookup existence が `stream_no < 3` と一致し、lookup が type byte を保存し、exact `auds`/exact `vids`/other が complement であり、first `auds`/first `vids` が table 順の最初の一致または absence と一致し、exact `wb` だけが audio payload kind、exact `db`/`dc` だけが video frame kind、実在 `auds`/`vids` packet が kind に関係なく timeline slot owner、missing/other stream packet が owner でないこと、selected first stream への payload 帰属が stream number equality と payload role の conjunction と一致すること、overflow/underflow/panic がないことを検査する。unknown kind は reject せず opaque に保持し、`pc` は video frame ではないが `vids` timeline slot を消費する現行契約を変えない。

採用する分解は fixed-table stream existence/kind classification proof である。production 側では lightweight `fcc_type` view を介する pure helper/predicate 群に分類規則を集約し、`packet_ticks`、`stream_timings`、first `auds`/`vids` selection、audio `wb` filter、video `db`/`dc` count/extraction filter が同じ規則を使う。proof は `StreamHead.format`、payload bytes、body slice、`Vec` accumulation、iterator、allocation、I/O を入力に持たず、type-only table は production `streams` と同じ hdrl 順の exact reparameterization である。`scale`/`rate`/`start`/`length`/`sample_size`/`strf` payload は classification core が観測しない入力として除外する。scalar predicate-only proof、小 proof family、scan collection 合成、direct layout/real Vec composition、AVI-STREAM-002 declaration validation、AVI-TIME-004 accumulation は採用しない。詳細な候補比較、assume 分類、接続条件、coverage は `evidence/al274-avi-stream-kind-proof/boundary-analysis.md` を正本とする。

runner は `AL_KANI_HARNESS_GROUP="avi-stream-kind"` で `avi_stream_kind_classification_bounded_exact` を選択する。registry は 30 件となる。AVI movi scan/composition/tag、MP4/AVI boundary、MP4 scan、WAV chunk/scan、WAV range の各 group は regression として再実行し、それらの proof 設計は変更しない。legacy proof を含む全件 lane はこの slice の必須検証にしない。


### AVI-STREAM-002 strh declaration implementation boundary(2026-11-27 avi strh declaration slice)

strh declaration proof の input domain は full `usize` body length scalar と fixed `[u8; 48]` symbolic prefix の組合せとする。length scalar は machine integer domain 全体を取り、`body_len < 48` の reject classと `body_len >= 48` の accept classの exact complementを検査する。fixed prefixは受理 body の先頭 48 byteが取りうる全 bit patternを覆う。48 byte以降の trailing payloadは production fixed-prefix extraction helperが型上観測しないためモデル化せず、offset 4..20・36..44 の未観測 byteは proof assertionが出力を観測 offsetだけで規定することで non-dependenceを検査する。input narrowing assumeは置かない。任意長 body slice、実 `Vec<StreamHead>`、`strl` ordering/missing `strh` を含む declaration correspondenceは未証明領域に残す。

proof は 1 回の `strh` declaration field extraction/validationの total correctnessを対象にする。length gate、`fcc_type` byte順序保持、offset 20/24/28/32/44 の little-endian u32 decode、`StreamHead` への field写像に対応する declaration field view、overflow/underflow/panic の不存在を検査する。`fcc_type` の `auds`/`vids`/other 分類は PR #289 の stream existence/kind classificationが担う別 propertyであり、この proofでは byte順序保持だけを扱う。

採用する分解は production length predicateと fixed-prefix extraction helperを接続した single bounded exact proofである。production `parse_strh` は `avi_layout` の `strl` loopから呼ばれ、length predicateで現行どおり 48 byte未満を同じ error文言で拒否し、受理時は先頭 48 byteを fixed prefixへ写して extraction helperを呼び、結果を `StreamHead` へ写す。`strf` bodyの `Vec` 複製、`has_strh`、body slice/lifetime、iterator/error precedenceは shellに残す。length-gate-only proof、field-decode-only proof、valid/invalid length family、combined strh+strf proof、direct layout composition proof、full AVI-STREAM-002 declaration validation proof、AVI-TIME-004 accumulation proofは採用しない。詳細な候補比較、exact reparameterization根拠、assume分類、接続条件、coverageは `evidence/al274-avi-strh-declaration-proof/boundary-analysis.md` を正本とする。

runner は `AL_KANI_HARNESS_GROUP="avi-strh-declaration"` で `avi_strh_declaration_bounded_exact` を選択する。registryは 31 件となる。AVI stream kind、AVI movi scan/composition/tag、MP4/AVI boundary、MP4 scan、WAV chunk/scan、WAV rangeの各 groupは regressionとして再実行し、それらの proof設計は変更しない。legacy proofを含む全件 laneはこの sliceの必須検証にしない。

### AVI-STREAM-002 video `strf` declaration implementation boundary(2026-11-27 avi strf declaration slice)

video `strf` declaration proof の input domain は full `usize` body length scalar と fixed `[u8; 40]` symbolic prefix の組合せとする。length scalar は machine integer domain 全体を取り、`body_len < 40` の first `vids` metadata local rejection classと `body_len >= 40` の extraction classの exact complementを検査する。fixed prefixは受理 bodyの先頭 40 byteが取りうる全 bit patternを覆う。40 byte以降の trailing payloadは production fixed-prefix extraction helperが型上観測しないためモデル化せず、offset 0..4・12..14・20..40 の未観測 byteは proof assertionが出力を観測 offsetだけで規定することで non-dependenceを検査する。input narrowing assumeは置かない。任意長 body slice、実 `Vec<StreamHead>`、`strf` presence/copy/ordering、`strl` ordering/missing `strh`を含む declaration correspondenceは未証明領域に残す。

proof は 1 回の video `strf` BITMAPINFOHEADER declaration field extraction/validationの total correctnessを対象にする。length gate、width offset 4..8、height bit pattern offset 8..12、bits per pixel offset 14..16、compression offset 16..20 の little-endian decode、height の signed/`unsigned_abs` metadata semantics、declaration field viewへの写像、overflow/underflow/panic の不存在を検査する。width/height/compression/bits per pixel の値 validity matrixは追加しない。BITMAPINFOHEADER decode suitability validationは `dib_format` の別責務であり、この proofの保証に含めない。

採用する分解は production length predicateと fixed-prefix extraction helperを接続した single bounded exact proofである。production `video_info` は first `vids` metadata use siteとして、length predicateで現行どおり 40 byte未満を同じ error文言で local errorにし、受理時は先頭 40 byteを fixed prefixへ写して extraction helperを呼び、結果を `VideoInfo` へ写す。`avi_layout` の `strf` body `Vec` 複製、fps/frames/duration、codec string rendering、feature extraction、body slice/lifetime、iterator/error precedenceは shellに残す。audio WAVEFORMATEX proof、shell presence/capture proof、length-gate-only proof、field-decode-only proof、valid/invalid length family、combined audio+video proof、direct layout composition proof、full AVI-STREAM-002 declaration validation proof、AVI-TIME-004/AVI-PCM/MP4-TABLE-005 proofは採用しない。詳細な候補比較、exact reparameterization根拠、assume分類、接続条件、coverageは `evidence/al274-avi-strf-declaration-proof2/boundary-analysis.md` を正本とする。

runner は `AL_KANI_HARNESS_GROUP="avi-strf-declaration"` で `avi_video_strf_declaration_bounded_exact` を選択する。registryは 32 件となる。AVI strh declaration、AVI stream kind、AVI movi scan/composition/tag、MP4/AVI boundary、MP4 scan、WAV chunk/scan、WAV rangeの各 groupは regressionとして再実行し、それらの proof設計は変更しない。legacy proofを含む全件 laneはこの sliceの必須検証にしない。

## canonical 21 proof の disposition 要約

- **Kani に残す/再構成:** `detect_format_signature_rules`。small pure property に適合するが、len 0,7,8,11,12,13 と RIFF/ftyp 競合の boundary matrix に再構成する。
- **Required replacement NotRun:** `header_reads_valid_ranges_slice_exact`、`header_reads_invalid_ranges_error`。現行 fixture whole-read proof ではなく、Range arithmetic の局所 proof に置き換える。
- **再構成（Deferred）:** WAV/container whole-call no-panic、MP4/AVI writer symbolic end-to-end proof。対応する local property または proptest primary assurance へ分解する。
- **廃止/統合（Deferred）:** unsigned early reject、mutation-only/truncation-only whole-parser proof。signature/boundary proof と proptest coverage に統合する。

現行 proof をすぐ削除しない。代替となる Required local proof が Pass し、proptest/未証明領域の対応が ledger 化されるまでは legacy inventory として残し、Required proof の計算対象から外す。これは現行 timeout を隠すためではなく、保証の空白を無断で作らないためである。

## fmt Stub と入力領域の監査

既存 stub evidence は read-only 引用とし、新たな Kani 計測は行わない。

- WAV の `wav_reads_never_panics_unsigned` と `read_wav_range_never_panics_signed` には `std::fmt::format` の empty stub がある。memchr/PadAdapter を除去して product 側到達は進んだが、1500 秒/proof でも verdict を返さなかった。
- fmt stub を維持できるのは、Result と error reason 文字列を検査しない proof に限る。formatting 自体の panic/alloc 挙動、message 長依存 allocation、diagnostic 内容は保証範囲から除外される。したがって、成功しても公開 API の無条件 no-panic ではなく、条件付き bounded guarantee である。
- MP4 writer proof では codec assertion が `format!` 生成文字列を使うため、fmt stub は contract-equivalent ではない。spike の fmt/both は cost mechanism の control であり、product 反例や採用根拠として扱わない。
- alloc-error stub は failure endpoint の event を減らしたが wall-clock/RSS/到達深度を有意に改善しなかった。採用しない。
- memory-safety instrumentation、slice bounds、pointer validity は stub しない。これらを外すと検証対象の安全性そのものを弱める。
- WAV Range proof の u8 由来 [0,255] は 8frame fixture の分岐代表には有効だが、公開 u64 API 全域ではない。valid/invalid pair の分割自体は補集合で coverage を落とさない一方、fixture 外への一般化はできない。
- mutation-only/truncation-only 分割は per-proof 分岐を減らすが、mutation+truncation の複合入力は proptest に依存する。proptest を形式的網羅と同値に扱わない。
- MP4 signed と AVI signed は RIFF size field が偶然 `ftyp` になる入力で重複し得る。unsigned との union coverage は失われないが、「厳密に disjoint な 3 分割」という表現は正確ではない。

## Boundary design options

### Option A: assurance pack（推奨）

proptest 13 件を常時実行 baseline に保ち、Kani は signature、PCM conversion、chunk/box boundary、scan progress、Range/timing arithmetic など小さい local proof の assurance pack に再構成する。whole-call symbolic proof は代替が揃うまで legacy inventory に残し、Required proof から外す。

| 評価軸 | 評価 |
| --- | --- |
| 保証範囲 | 高リスクの境界/算術/変換に形式的保証を集中できる。whole API no-panic は proptest primary で明示的に残る |
| 検出能力 | off-by-one、overflow、padding、scan 停滞、PCM sign/width、Range boundary、timing remainder に強い。multi-edit corruption や広い writer matrix は proptest に残る |
| 実行コスト | 目標は各 proof 10 分以内・4GiB 未満。小型化で 1500 秒 timeout の fixed floor を避けやすい |
| 再実行コスト | property 単位選択で 10〜20 分を目標にできる。Daily Full CI の sequential pack は 60 分以内を設計上限にする |
| CI 適合 | Daily Full CI 集約・非ブロッキングに適合する。PR 必須 blocking lane を作らない |
| 実装 prerequisite | parser の純粋算術/ walker / conversion を小さな責務へ切り出すか、True to the Code を保つ proof model が必要。固定長 sink は検証支援として有用 |
| 移行コスト | 中。proof と少量の責務整理が必要。製品契約や入出力形式は変えない |
| 残留リスク | helper/model と実装の接続条件を誤ると形式的保証が製品コードに届かない。whole API no-panic は未証明として残る |

### Option B: 現行 whole-call proof の戦術改良

runner 分割、unwind 調整、bounded input、lazy diagnostic、fmt/alloc stub、並列実行などで現行 21 proof を残しながら verdict を取りに行く。

| 評価軸 | 評価 |
| --- | --- |
| 保証範囲 | 成功すれば広い統合範囲を直接主張できる。ただし bounded かつ stub 条件付きであれば公開 API 全体 no-panic ではない |
| 検出能力 | 統合経路の interaction defect は拾える。原因局所性は低い |
| 実行コスト | 現行 20/21 が 1500 秒 timeout。支配因は instrumentation と allocation/parser branch であり、戦術改良だけで実用予算に入る確証はない |
| 再実行コスト | 高い。小変更でも whole-call proof の再計測が必要になる |
| CI 適合 | Daily 集約でも時間とメモリの変動が大きい。PR blocking には不適 |
| 実装 prerequisite | lazy diagnostic や runner 改善が必要。製品実装の診断設計に踏み込む |
| 移行コスト | 低〜中。既存 harness を残せるが、検証意味論の整理は必要 |
| 残留リスク | timeout が続く限り形式保証は増えない。fmt stub を広げると保証範囲を狭める |

### Option C: proptest primary + mutation/golden evidence 中心

Kani は `detect_format` 級の small pure proof に限定し、parser 安全性と round-trip は proptest、mutation detection、golden/regression で支える。

| 評価軸 | 評価 |
| --- | --- |
| 保証範囲 | 統計的 coverage は広いが、boundary arithmetic の形式的網羅はない |
| 検出能力 | corruption/round-trip/facts には強い。rare boundary の保証は seed/generator 依存 |
| 実行コスト | 最も低い。常時 lane を維持できる |
| 再実行コスト | 低い |
| CI 適合 | 最も適合する |
| 実装 prerequisite | 小さい。既存 test を維持・強化するだけでよい |
| 移行コスト | 低い |
| 残留リスク | Issue #274 が求める chunk boundary、scan progress、Range 計算の形式的保証を満たさない。formal verification assurance pack 優先のユーザー指定に反する |

## 旧新の保証範囲・コスト比較

新構成は未実装であり、以下の新規側数値は実測ではなく設計予算である。旧構成の数値は `evidence/al274/` の read-only 引用であり、canonical 1500 秒/proof と ablation 600 秒 cap を混同しない。

| 比較対象 | 旧/現行構成 | 推奨 target structure | 保証範囲の変化 | 再実行コストの変化 |
| --- | --- | --- | --- | --- |
| signature 判定 | `detect_format_signature_rules` は Pass。symex 約0.69秒。len 7/8/12 の代表境界のみ | len 0,7,8,11,12,13 と RIFF/ftyp 競合を small local proof で維持。proof 2分以内・4GiB未満 | 代表長だけの主張から boundary matrix へ拡張。prefix-only 性質の接続条件を明示 | 現行と同じく低コストを維持する設計 |
| WAV/container whole-call no-panic | 20件の大半が 1500秒 timeout。代表 peak は unsigned 13762MiB、range 10324MiB、MP4 writer 7150MiB。形式保証は未取得 | chunk/box boundary、scan progress、Range/timing arithmetic の local proof。各 proof 10分以内・4GiB未満を予算化。whole-call no-panic は Deferred | bounded whole API の未成証明を、未証明のまま小さい高リスク Property の Required proof に置き換える。局所 proof を全 API no-panic へ転用しない | 対象 Property だけを選択実行でき、10〜20分を目標にする。Daily sequential pack は60分以内を上限にする |
| writer round-trip | WAV/MP4/AVI の symbolic end-to-end は timeout。mp4 writer は trial 1200秒/約4.5GiB、avi writer は 1200秒/約3.9GiB でも未収束 | round-trip は proptest primary。PCM conversion、header/serialization、sample extraction を local proof または concrete matrix に分離 | 任意 symbolic end-to-end の主張は下げるが、数値変換・header・table arithmetic の説明責任を強める。製品契約は変更しない | 変更箇所に応じて小さい proof/proptest を選べる。全 end-to-end symbolic proof の常時再計測を避ける |
| mutation/truncation | mutation-only/truncation-only whole-parser proof は 900〜1200秒 trial でも timeout。複合入力は従来から proptest | walker boundary proof + proptest multi-edit/truncate coverage。whole-parser corruption proof は Deferred/統合 | corruption dimension を形式的に網羅する主張を縮小し、walker 境界を形式化、複合 corruption は Property Tested として明示 | walker/property 単位で再実行可能。proptest は現行 lane 内に留める |

この比較は「新構成の方が保証が常に広い」という意味ではない。旧構成が取得できていなかった保証を成功扱いせず、製品リスクの高い局所 Property に必要な保証を集中させ、残りを Property Tested/Deferred/未証明として明示する点が変更である。

## 推奨案

**Option A を推奨する。** 理由は、現行 timeout の支配因が proof 面積である一方、製品リスクは boundary/scan/range/timing/PCM conversion に集中しているためである。小さい proof は失敗時の原因局所性が高く、固定長 sink や concrete matrix と組み合わせれば再実行コストも制御できる。`ruviform#806` で有効だったのは単なる harness 削減ではなく、local proof、exact gate、固定長 sink、proptest との union coverage を ledger で正直に管理した点である。AudioLens でも同じ原則を採用する。

Option B は完全には捨てない。lazy diagnostic や runner 改善は assurance pack の cost を下げる補助手段になり得る。ただし、現行 whole-call proof を Required proof の中心に据える案は採用しない。Option C は baseline として必要だが、形式的保証を増やす案ではない。

## formal verification assurance pack を優先する理由

assurance pack は「21 proof を全部通す」ことではなく、製品リスクごとに必要な保証種別を選び、Required proof を小さく Pass させる構成である。これにより、次の効果を期待する。

- timeout の whole-call proof を成功扱いしないまま、未証明領域を Property 単位で管理できる。
- boundary/scan/range の代表的 local proof を実用予算内に設計できる。
- proptest との overlap を保証の重複ではなく補完として記録できる。
- Daily Full CI で再実行可能な assurance pack に集約し、PR の lint/test hygiene を長時間 Kani で止めない。
- mutation detection evidence を、mutation を入れる位置と検出すべき oracle に対応付けられる。

## mutation detection evidence の設計

本 slice では新規 mutation を実行しない。後続実装 slice では、変更または新規追加する検証ごとに次を記録する。

| 検証 | 代表的 mutation | 期待する検出 |
| --- | --- | --- |
| WAV chunk boundary proof/test | `body_end` の off-by-one、padding の有無反転、size overflow | Err、拒否、または boundary assertion failure になる |
| WAV scan progress proof/test | offset を進めない、pad を二重に進める | progress/termination assertion failure または structural test failure になる |
| WAV Range proof/test | `end > total` を許す、`start*block_align` で wrap する | invalid range Err または arithmetic assertion failure になる |
| PCM conversion proof/test | sign extension、bit width、endian、clamp/round/cast を壊す | conversion equality/range oracle failure になる |
| MP4 box walker proof/test | size32/size64/size0 の limit を 1 byte ずらす | Err または scan/boundary assertion failure になる |
| AVI movi timing proof/test | remainder 処理、stream 別累積、dwStart offset を壊す | timing oracle failure になる |

mutation が検出されなかった場合は、mutation が不適切か、oracle が弱いか、対象 Property の保証が不足しているかを区別して ledger に記録する。mutation survival を test fail と同義にせず、検出不能理由を残す。

## RF 還元判定

候補はあるが、本 slice では新規 Issue 起票や既存 Issue 更新を行わない。`ruviform#1015` が PCM WAV backend、`cfg(kani)` 診断置換、共通検証支援の再評価を既に担当しており、重複 Issue を作るべきではない。AudioLens 側で共有価値がある候補は次の通りである。

- 固定長 `Write+Seek` sink / `KaniSink` の test-only 検証支援。
- RIFF chunk boundary walker と WAVEFORMATEX/PCM conversion の検証済み共通契約。
- diagnostic error payload を検証対象から分離する設計知見。`cfg(kani)` 置換ではなく lazy diagnostic/contract 型の再設計として評価する。

Cross-repo Issue の起票または更新が必要と判断した場合は、実施前に Coordinator へ確認する。本 slice は不要と判断し、確認も実施していない。

## ユーザーへ返す open decision

1. **pure helper への責務抽出をどこまで行うか。** True to the Code を最も強くするには実装から純粋算術/walker を切り出す。一方、製品コードの refactor を最小化するなら test-side model を作る。後者は接続条件が弱い。推奨は「小さい責務抽出を優先し、製品契約を変えない範囲で行う」である。
2. **lazy diagnostic を独立した製品改善として採用するか。** Kani を通すためだけに診断実装を変えるべきではない。診断コスト自体が製品上も問題なら別 Issue で採用し、proof では diagnostic 契約を明示する。
3. **固定長 sink/concrete matrix をどこまで使うか。** writer round-trip の symbolic end-to-end を維持するより、serialization/pcm conversion を symbolic local proof にし、integration は concrete matrix に寄せる案を推奨する。採用範囲は実装 slice で決める。
4. **実行予算の確定値。** 本書では各 local proof 10 分以内・4GiB 未満、選択実行 10〜20 分、Daily sequential pack 60 分以内を設計予算とする。実測後に厳しすぎる/緩すぎる場合は証跡付きで調整する。
5. **現行 legacy proof の retire 条件。** 代替 Required proof が Pass し、proptest と未証明領域の対応表が更新された時点で廃止/統合する。それまでは registry から外さないか、外す場合も NotRun/Deferred を明示するかを決める。

## 後続 slice に残す作業

- WAV-CHUNK-003/WAV-SCAN-004 は PR #283、MP4-BOX-002/AVI-CHUNK-001 は PR #284、MP4-SCAN-003 bounded scan は PR #285、AVI-MOVI-003 tag identity は PR #286、AVI-MOVI-003 movi entry composition は PR #287、AVI-MOVI-003 movi scan collection は PR #288、AVI stream existence/kind classification は PR #289、AVI-STREAM-002 `strh` declaration field extraction/validation は PR #290、AVI-STREAM-002 video `strf` BITMAPINFOHEADER declaration field extraction/validation は本 sliceで実装・実走済みである。残り Required local proof の実装と実走。
- AVI-MOVI-003 の unbounded iteration termination、N+1 件以上の entry 列、実 `Vec<MoviPacket>` accumulation、body slice/lifetime composition、nested movi/list walker 全体、longer stream table/dynamic `Vec`、declaration correspondence、packet body boundary と tag の完全合成、MP4-TABLE-005（`stts`/`ctts`/`stsc`/`stsz`/`stco` 累積と `Vec` 展開）、MP4-PCM、AVI-TIME-004（u128 除算・remainder・複数 stream/packet 累積）、AVI-PCM。
- runner 登録 schema、assurance pack、選択実行の Daily CI 集約。
- lazy diagnostic の製品設計と実装（採用する場合）。
- 全体 mutation matrix と完全な旧新 cost matrix。MP4/AVI boundary slice の mutation/cost 比較は `evidence/al274-mp4-avi-proof/` に限定証拠として記録済み。
- legacy whole-call proof の残り retire 判定、`evidence/al274-boundary/` への retired/superseded ledger schema 統合。
- Required 検証の Pass 化と未証明領域の最終管理。
- audio-lens#278 での文書責務分離と docs index 整備。

## 完了時の検証境界

元の設計/ledger slice では、ローカル検証を lease 内ファイルの markdown/JSON/照合に比例させ、`mise run fmt` と `mise exec -- prek run --all-files` を canonical hygiene とした。`check`/`lint`/`test`/`test-formal`/`test-formal-kani` はコード変更を伴わないため NotApplicable とした。後続 implementation slice では別途、対象クレートの `check`/`lint`/`test`/`test-formal` と選択 Kani assurance group を必須検証とする。MP4/AVI boundary slice の実行結果は `evidence/al274-mp4-avi-proof/verification.md`、MP4 scan slice の実行結果は `evidence/al274-mp4-scan-proof/verification.md`、AVI movi tag slice の実行結果は `evidence/al274-avi-movi-tag-proof/verification.md`、AVI movi scan collection slice の実行結果は `evidence/al274-avi-movi-scan-proof/verification.md`、AVI stream existence/kind classification slice の実行結果は `evidence/al274-avi-stream-kind-proof/verification.md`、AVI strh declaration slice の実行結果は `evidence/al274-avi-strh-declaration-proof/verification.md`、AVI video strf declaration slice の実行結果は `evidence/al274-avi-strf-declaration-proof2/verification.md` に記録する。統合検証は merge 後の対象 SHA に対する CI 証拠で確認する。

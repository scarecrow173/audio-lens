# MP4-BOX-002 / AVI-CHUNK-001 exact proof boundary analysis

対象は Issue #274 の MP4-BOX-002（ISO-BMFF box header の size32/size64/size0/overflow/limit boundary）と AVI-CHUNK-001（RIFF chunk boundary/padding/region limit）である。本書は solver 実験や runner 変更に先立つ境界設計であり、PR #282/#283 で Required Pass 済みの WAV proof を再設計しない。WAV proof は runner 変更の regression としてだけ再実行する。

## 1. Property の選定

MP4-BOX-002 は top-level scan、`moov`/`trak`/`mdia`/`minf`/`stbl`、`stsd`、sound sample entry のすべての nested box walk が使う共通入口である。size form・header 最小値・limit 包含・checked arithmetic の誤りは、後続の semantic parse と sample table の前提を崩す。AVI-CHUNK-001 は RIFF body、`hdrl`、`strl`、`movi` のすべての nested region が使う共通 iterator である。body end、word-alignment pad、trailing byte、次状態の誤りは stream declaration、movi packet boundary、timing、PCM decode の事前条件を崩す。

両者は allocation、I/O、`Vec` decode、PCM decode、video decode、diagnostic formatting を除いた scalar arithmetic/pure core に自然に分離できる。size form、limit、padding、off-by-one、overflow、進行停滞という検出価値の高い mutation を少数 proof で直接観測できるため、費用対効果と mutation 検出の代表性が高い。

MP4 sample table（`stts`/`ctts`/`stsc`/`stsz`/`stco`）は `Vec` 展開、累積 loop、複数 table の合成を含む。AVI movi timing は u128 除算、remainder、複数 packet/stream 累積を含む。どちらも Required だが、最初の代表 proof としては solver 負荷と接続条件が重いため本 slice では選ばない。MP4-SCAN-003、MP4-TABLE-005、MP4-PCM 系、AVI-MOVI-003、AVI-TIME-004、AVI-PCM 系は後続 slice に残す。

## 2. exact input domain

### MP4-BOX-002

helper の入力は `(at: u64, limit: u64, size32: u32, largesize: u64)` の全数値領域とする。`at` と `limit` は production index を u64 scalar に写した値、`size32` は box header の u32 size field、`largesize` は size32 == 1 のときだけ意味を持つ u64 largesize field である。slice 長、enclosing box length、ISO-BMFF 上の到達可能性による事後 narrowing はしない。

`largesize` を size32 != 1 の入力でも symbolic に渡すのは coverage を保存する exact reparameterization である。helper はその branch で値を読まないため、結果は largesize に依存しない。production では truncated largesize header の場合に largesize を読まないが、helper には未読値の代わりに 0 を渡す。この 0 は観測されず、proof が full largesize domain を覆うため保証差を生じない。

`at > limit` は production scan からは到達しないが、total extension として truncated base header 側へ分類する。`at + 8` や `at + 16` の u64 overflow も、対応する header が利用できないことの exact complement として truncated 側へ分類する。これは production 到達状態の意味論を変えず、数値領域を狭めない。

### AVI-CHUNK-001

helper の入力は `(offset: u64, region_len: u64, size: u32)` の全数値領域とする。`offset` は現在の chunk header の region 先頭からの相対位置、`region_len` はこの iterator が受け取った enclosing region の全長、`size` は chunk header の u32 size field である。relative offset 表現を採るため、valid chunk の `body_start` は `offset + 8`、最初の chunkでは 8 となる。これは task で許容された offset 基準の同値式である。

`offset > region_len` は production iterator からは到達しないが、total extension として trailing bytes 側へ分類する。production では region_len が usize 由来だが、proof は full u64 を対象にする stronger domain であり、constructor-admitted domain への縮小ではない。

## 3. 証明する性質

### MP4-BOX-002

helper は全入力を `Advance` または exact な `Invalid` へ total に写す。valid 側の exact 条件は次の通りである。

- base header が利用できる。すなわち `at + 8` が u64 で overflow せず、`at + 8 <= limit` である。
- size32 == 0 の box は base header 8 byte を持ち、enclosing region end まで伸びる。`body_start = at + 8`、`body_end = limit` である。
- size32 == 1 の largesize box は 16 byte header を持つ。`at + 16` が overflow せず limit 内であり、`largesize >= 16` である。`body_start = at + 16`、`body_end = at + largesize` と同値である。
- size32 >= 8 の box は 8 byte header を持ち、`body_start = at + 8`、`body_end = at + size32` と同値である。
- `body_end` 計算は checked arithmetic で overflow/underflow/panic がなく、`body_start <= body_end <= limit` である。
- valid box は少なくとも header 分だけ進行し、`body_end > at` かつ `limit - body_end < limit - at` という残余量の厳密な減少が成立する。

invalid exact complement は、base header truncated、largesize header truncated、size32 smaller than header、largesize smaller than 16、size calculation overflow、region end overrun である。error precedence は production と同じく base truncated、form-specific truncated/minimum、overflow、limit overrun の順とする。

### AVI-CHUNK-001

helper は全入力を `End`、`TrailingBytes`、`Advance`、または exact な `Invalid` へ total に写す。terminal exact complement は次の通りである。

- `offset == region_len` は `End` であり、空 region または exact end で iteration が終わる。
- `offset != region_len` かつ header が使えない状態は `TrailingBytes` である。production 到達状態では 1..7 byte trailing、`offset > region_len` は total extension である。

valid 側の exact 条件は次の通りである。

- header が使える。すなわち `offset < region_len` かつ `region_len - offset >= 8` である。
- `body_start = offset + 8`。
- `body_end = body_start + size` が u64 で overflow せず、`body_end <= region_len` である。
- odd-sized chunk の word-alignment pad を含む `next_offset = body_end + (size mod 2)` が u64 で overflow せず、`next_offset <= region_len` である。
- `next_offset >= offset + 8` であり、少なくとも header 分だけ進行する。
- `next_offset <= region_len` であり、`region_len - next_offset < region_len - offset` という残余量の厳密な減少が成立する。

invalid exact complement は、header が使える状態での body end overflow、body extends past end、missing pad である。error precedence は production と同じく body end overflow、body overrun、missing pad の順とする。

## 4. scan/termination の扱い

本 slice の Required proof は single-step transition exactness とする。valid step の strict progress/measure decrease を proof に含めるが、これを無制限 scan termination の機械的証明とは読み替えない。finite unrolled scan model は追加しない。理由は、MP4-BOX-002/AVI-CHUNK-001 の対象が共通入口の境界算術であり、scan synthesis を追加すると今回の mutation oracle と solver 負荷が混ざるためである。MP4-SCAN-003 と無制限/多数 chunk scan は未証明領域に残す。

## 5. 検証しない領域

この local proof は次を証明しない。

- MP4 semantic box type、duplicate/required box、skipped box path、sample table、PCM declaration/extraction、writer fixture、writer round-trip。
- AVI LIST form type semantic、stream declaration、movi packet id/kind、timing cumulative clock、PCM/video decode、writer round-trip。
- I/O、allocation、`Vec` decode、diagnostic formatting、公開 API 全体 no-panic。
- signature 判定、format detection、mutation/truncation を含む whole-parser integration。

これらは未証明領域として ledger に記録し、local proof Pass を上位保証へ転用しない。

## 6. production 接続条件

MP4 では production `box_header` が header byte から size32/largesize/tag を読み、boundary arithmetic 自体は production pure helperへ委譲する。top-level scanと全 nested `child_boxes` は既存どおり `box_header` を通る。helper の事前条件は、at/limit が実 slice 由来の usize から widening された値であることだけであり、proof domain はそれを包含する。helper の `body_start`/`body_end` は limit 内に証明されるため、production は usize へ戻して body slice や次状態へ届ける。

AVI では production `Chunks::next` が region-relative offset/region_len/size を pure helper へ渡す。RIFF body、`hdrl`、`strl`、`movi` の nested region は既存どおり同じ iterator を使う。helper の valid 結果から body slice、pad を含む次 remainder、次 offset を導く。tag/id 解釈と error 文言生成は production shell に残し、boundary/offset 計算を複製しない。

両者とも test-side model だけを証明して True to the Code を満たしたとはしない。`cfg(kani)` 置換、fmt/alloc stub、writer fixture、`Cursor`、sample table `Vec` decode、full parser は proof target に入れない。

## 7. assume と coverage

- MP4/AVI とも input narrowing assume は置かない。full numeric domain で検査する。
- largesize を全入力で symbolic にすることは exact reparameterization であり、size32 != 1 では観測されない。
- proof が呼ぶのは production helper 自体であり、coverage 保存のない test-side model ではない。
- assume の充足可能性は input narrowing がないため自明である。主要経路の到達性は deterministic unit test で size32 exact end/overrun、size0、largesize exact/overrun、truncated header、size/largesize minimum、large offset arithmetic、AVI odd/even/zero size、exact end、missing pad、trailing bytes、body overrun、複数 chunk、nested region 相当を確認する。
- 未証明の数学的事実は assume しない。one-step measure decrease と有限 region length から人間が termination を説明してよいが、無制限 scan termination を Kani proof 済みとは記録しない。

## 8. decomposition 候補の比較

### single-step total correctness proof（採用）

1 つの symbolic box/chunk step について valid/invalid/truncated/end を total に検査し、boundary arithmetic と progress measure を exact に証明する。任意回転や全 nested scan へ直接 generalized しない。helper が allocation/I/O を持たない単純算術であり、full scalar domain でも実用予算に入る見込みが高い。exact complement を 1 proof にまとめられるため、proof family 間の union coverage を追加管理しなくてよい。

### valid/invalid exact complement family（不採用）

valid acceptance/layout exactness と invalid rejection を 2 件以上に分ける案である。分岐空間が小さい今回は単一 proof で exact complement を直接検査できる。分割すると harness 数と runner ledger が増えるだけで、solver query の支配因は変わらない。実測で単一 proof が予算超過した場合にだけ再評価する。

### finite unrolled scan model（不採用）
宣言した small step 数まで symbolic size 列を unroll し、strict progress と terminal 判定を合成検査する案である。本 slice の対象は scan Property ではなく boundary helper Property なので、今回は不要である。MP4-SCAN-003 や AVI の scan synthesis を扱う後続 slice で、step 数を結果に合わせて後から下げない条件つきで再評価する。

### measure-decrease proof 単独（不採用）

valid step の残余量減少だけを証明する案である。termination の数学的根拠にはなるが、size0/largesize/pad/limit containment/invalid exact complement を証明しない。単独では不採用とし、single-step proof の assertion として併用する。

### function contract / loop invariant / verified stub decomposition（不採用）

今回の helper は小さい純粋算術であり、loop も外部関数も持たない。unstable flag の適用範囲、proof-for-contract registry、assert 後 assume の管理を増やす理由がない。採用しない。

## 9. runner 戦略と予算

runner に `mp4-avi-boundary` assurance group を追加し、Required proof `mp4_box_header_step_exact` と `avi_chunk_step_exact` を漏れなく選択する。既定全件挙動、fail-closed、WSL mirror、target 配置、timeout 意味論、group/subset 排他意味論は変えない。per-harness elapsed ms、raw log、pass/fail/not_run、`/usr/bin/time -v` が利用できる環境での max RSS を維持し、選択 lane の total/max cost summary を additive に追加する。

単一 proof timeout は canonical lane で `AL_KANI_TIMEOUT_MS=600000` とする。設計予算は各 proof 10 分以内・4 GiB 未満、mp4-avi-boundary 選択実行合計 10〜20 分を目標とする。legacy proof を含む全件 Kani lane はこの slice の必須検証にしない。proof が timeout/Fail なら、solver や timeout だけを変えて再試行せず、分解と boundary を見直す。Required Pass が得られなければ proof/contract を弱めず Required NotRun/Fail として報告する。

## 10. mutation detection plan

Able to Fail 証拠として、次を別々に行う。

- MP4 box arithmetic mutation: size0 end 計算の off-by-one、largesize/header minimum の誤り、body_end limit の 1 byte ずれ、checked 加算の欠落のいずれか。
- AVI chunk arithmetic mutation: pad 加算漏れ、pad の二重計上、body_end off-by-one、次状態を進めない変更のいずれか。

diagnostic payload だけの mutation は使わない。各 mutation は mp4-avi-boundary canonical lane で対応 proof の少なくとも 1 件に `VERIFICATION:- FAILED` が出ることを確認した後、完全 revert し、通常 lane を再実行する。検出されなければ mutation が不適切か oracle が弱いかを区別し、weakened Pass にせず ledger に残す。

## 11. proptest との関係

MP4/AVI boundary の method は `Kani local` である。既存 mutation/truncation/no-panic proptest は弱めず、whole parser integration の統計的 assurance として残す。既存 AVI unit test は overrun/missing pad/trailing bytes を integration 経路で持つが、helper 接続を直接示す小さい deterministic regression を追加する。MP4 には src 内 unit test がないため、size form/limit/error precedence の小さい deterministic regression を追加する。case budget を不必要に増やさず、formal proof の代替にしない。

## 12. RF 還元判定

ISO-BMFF box boundary walker と RIFF chunk boundary walker は汎用化できる可能性がある。ただし現時点の接続条件は AudioLens の error precedence、`MediaError` 写像、MP4/AVI scan semantics に結び付いており、実証済みの再利用契約として切り出せる段階ではない。本 slice では Ruviform Issue を起票せず、候補と理由を PR 本文に記録する。

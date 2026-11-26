# WAV-CHUNK-003 / WAV-SCAN-004 exact proof boundary analysis

対象は Issue #274 の WAV-CHUNK-003（chunk offset/size/padding boundary）と WAV-SCAN-004（scan progress/termination/trailing byte 判定）である。本書は実装と solver 実験に先立つ境界設計であり、PR #282 で Required Pass 済みの WAV-RANGE-008 を再設計しない。Range proof は `crates/al-media/src/wav.rs` を変更する regression としてだけ再実行する。

## 1. exact input domain

### WAV-CHUNK-003

boundary helper の入力は `(offset: u64, file_len: u64, size: u32)` の全数値領域とする。`size` は RIFF chunk header の u32 size field、`offset` と `file_len` は production が使う u64 表現である。full u64 file/offset domain を proof domain に採用し、constructor-admitted RIFF file length domain へ縮めない。

この選択は入力を広げる方向であり、保証を弱める事後縮小ではない。production の scan は RIFF size field の制約から `file_len <= u32::MAX + 8` の状態にだけ到達するが、boundary arithmetic 自体は u64 全域で total に定義できる。full u64 で証明すれば、RIFF constructor が将来境界内で変わっても同じ helper の算術保証を再利用できる。solver が重いことを理由に後から縮める場合は weakening として扱い、Pass としない。

helper は scan invariant の内部状態だけを対象にするのではなく、数値上あり得るすべての `offset`/`file_len` 組合せを valid/invalid/terminal のいずれかへ写す。`offset > file_len` は production loop からは到達しないが、total extension として terminal trailing 側へ分類する。この extension は production 到達状態の意味論を変えない。

### WAV-SCAN-004

scan model の初期状態は production と同じ `offset = 12` とする。`file_len` は u64 全域のうち `file_len >= 12` とする。これは production が 12 byte の RIFF/WAVE header を読み、signature と RIFF size を確認した後にだけ scan へ入る exact constructor precondition であり、入力縮小ではない。RIFF size field による上限は仮定しない。

small scan model は、terminal または invalid に達するまでに受理する valid chunk 数が高々 3 である入力列を対象にする。size は full u32、file length は full u64 >= 12 のままとし、代表値や concrete fixture へ sampling しない。4 つ目の symbolic size までは、3 valid chunk の後に invalid chunk で終わる列を exact に覆うために使う。4 valid chunk 以上を受理する列だけが small model の外側である。

step 数 3 の根拠は、initial offset からの最初の遷移、valid chunk 後の nonzero offset からの遷移、さらにその次の遷移を同じ proof で合成できる最小余裕を持つことである。1 step では scan の合成を検査できず、2 step では trailing/terminal 判定の前に 2 回の valid step を通る列と terminal で終わる列の両方を覆えるが、3 step のほうが回帰としての判読性と検出能力の余裕が大きい。full u64/full u32 arithmetic のまま実用予算に入るかを実測し、入らなければ proof を弱めず原因を ledger に残す。

## 2. 証明する性質

### boundary step

1 回の chunk header interpretation について、helper が全入力を `Advance` / `Invalid` / `Terminal` のいずれか 1 つだけへ total に写すことを証明する。valid 側の exact 条件は次の通りである。

- header が使える。すなわち `offset < file_len` かつ残余が 8 byte 以上である。
- `body_start = offset + 8`。
- `body_end = body_start + size` が u64 で overflow しない。
- `body_end <= file_len`。
- odd-sized chunk の word-alignment pad を含む `next_offset = body_end + (size mod 2)` が u64 で overflow せず、`next_offset <= file_len`。
- `next_offset >= offset + 8` であり、少なくとも header 分だけ進行する。
- `next_offset <= file_len` であり、`file_len - next_offset < file_len - offset` という残余量の厳密な減少が成立する。

invalid exact complement は、header が使える状態で、size 加算 overflow、chunk body が file length を越える、または pad byte が file length 内にない場合である。terminal exact complement は、header が使えない状態であり、`offset == file_len` なら complete、それ以外なら trailing bytes である。1..7 byte の端数は trailing byte 判定であり、valid chunk とはみなさない。

### scan progress

scan proof は次を検査する。

- 初期 offset は 12 である。
- terminal 判定は exact end と trailing bytes を区別する。exact end だけが scan 完了で、1..7 byte の端数は production では malformed trailing bytes へ写像する。
- valid step ごとに offset が単調増加し、少なくとも 8 byte 進み、`file_len` を越えない。
- valid step ごとに残余量 `file_len - offset` が厳密に減る。
- invalid step は scan を継続せず、boundary helper の invalid 分類として終わる。
- 高々 3 valid chunk の small model 内では、symbolic size 列、file length、terminal/invalid 条件の全組合せについて、上の進行性と終端分類が成立する。

one-step measure decrease は exact proof の一部として検査する。ただし、任意回数の合成による帰納的 termination を Kani が機械的に証明したとは読み替えない。small model は scan transition の境界と terminal 判定を exact に覆い、measure decrease は有限 file length に対する数学的 termination 根拠を与える。無制限 chunk 数の termination は機械証明済み領域に入れず、ledger の未証明領域に残す。

## 3. 検証しない領域

この local proof family は次を証明しない。

- seek/read/short read、I/O error、underlying reader の振る舞い。
- `Vec` allocation と allocation failure。
- PCM decode、sample conversion、f64 精度。
- diagnostic formatting と人間向け error message の内容。
- fmt/data chunk semantic、format tag、channels、sample rate、block_align の解釈。
- duplicate fmt/data 判定、unknown chunk semantic、chunk id の意味。
- RIFF signature/size 検証そのもの。
- 公開 API 全体 no-panic。
- 無制限 chunk 数を機械的に網羅した scan termination。

これらは未証明領域として ledger に残す。local proof Pass を上位保証へ転用しない。既存 proptest と unit test は統合 coverage として維持し、formal proof の代替にも重複証拠にもしない。

## 4. production への接続条件

True to the Code の接続条件は次の通りである。

1. production `wav_layout` は scan position helper を使い、exact end、trailing bytes、header availability を判定する。
2. header availability がある場合だけ 8 byte header を読み、同じ production boundary helper へ `(offset, file_len, size)` を渡す。
3. boundary helper の `Advance` が返す `body_start` を fmt body read に使い、`next_offset` を次の scan offset に使う。
4. boundary helper の invalid 分類は、呼出側で既存の `MediaError::malformed` 文言へ写像する。公開 error kind/text は変えない。
5. chunk id 解釈、duplicate fmt/data 判定、fmt size limit、fmt body parse は production loop に残し、boundary/offset 計算を複製しない。
6. helper は I/O、`Vec` allocation、PCM decode、`std::fmt` 診断文字列構築を実行しない。

test-side model だけを証明して完了とはしない。Kani proof は production helper を直接呼ぶ。scan harness 内の small unrolled model は、production helper の composition を検査する検証側モデルであり、製品実装の代替ではない。

## 5. assume と coverage の分類

| assume / bound | 分類 | 根拠 |
| --- | --- | --- |
| boundary proof に input narrowing assume を置かない | total numeric domain | helper は full u64/full u32 入力を total に写す |
| scan proof の `file_len >= 12` | exact constructor precondition | production は RIFF/WAVE header と RIFF size 確認後に scan へ入る |
| scan proof の高々 3 valid chunk | small scan model の明示 bound | full u64 file length と full u32 size は保持し、4 valid chunk 以上だけを未証明領域に残す |
| valid step の残余量減少を使う合成 | exact one-step proof + 人間の数学的合成 | small bounded model の成功を unbounded scan termination と読み替えない |

assume の充足可能性は、exact end、odd/even/zero size、missing pad、trailing bytes、複数 chunk、large offset/file length を通る concrete unit test で示す。主要経路は既存 integration/proptest でも到達する。未証明の数学的事実を assume しない。

## 6. decomposition 候補の比較

### single-step total correctness proof + finite unrolled scan model（採用）

1 つの symbolic step について valid/invalid/terminal の exact complement、boundary arithmetic、progress measure を証明する。scan harness は高々 3 valid chunk の small model を固定回数 unroll し、initial offset、strict progress、terminal/trailing 判定、invalid 終端を検査する。

採用理由は、boundary helper が allocation/I/O を持たない単純算術であり、valid/invalid を別 proof に分けなくても exact complement を 1 proof で検査できる見込みが高いことである。proof を分けないため union coverage の追加台帳が要らず、runner と ledger の同期も小さい。scan model 側で measure decrease assertion も検査し、任意回転への generalized termination を直接主張しないことを明確にする。

### valid/invalid exact complement family

valid acceptance/layout exactness と invalid rejection を 2 件以上に分ける案である。分岐空間が小さい今回は、単一 proof で exact complement を直接検査できる。分割すると harness 数と runner ledger が増えるだけで、solver query の非線形性や I/O 依存は変わらない。実測で単一 proof が予算超過した場合にだけ再評価する。現時点では採用しない。

### finite unrolled scan model 単独

small scan model だけを proof にし、boundary exactness を scan assertions に混ぜる案である。scan model だけでは単一 step の全入力分類が読みにくく、boundary mutation と scan mutation の検出責務が分かれない。採用しない。

### measure-decrease proof 単独

valid step の残余量減少だけを証明する案である。termination の数学的根拠にはなるが、body_end/pad/file_len containment、trailing byte 判定、invalid exact complement を証明しない。単独では不採用とし、scan/boundary proof の assertion として併用する。

### function contract / loop invariant / verified stub decomposition

Kani function contracts や loop invariant は、今回の helper が小さい純粋算術であるため必要ない。unstable flag の適用範囲、proof-for-contract registry、assert 後 assume の管理を増やす理由がない。採用しない。将来採用が必要になった場合は、proof-for-contract を Required proof として registry と canonical lane に含める。

## 7. runner 戦略と予算

runner には `wav-chunk-scan` assurance group を追加し、Required proof `wav_chunk_boundary_step_exact` と `wav_chunk_scan_progress_bounded_exact` を漏れなく選択する。既定全件挙動、fail-closed、WSL mirror、target 配置、timeout 意味論、group/subset 排他意味論は変えない。per-harness elapsed ms、raw log、pass/fail/not_run、`/usr/bin/time -v` が利用できる環境での max RSS を残す。

単一 proof timeout は canonical lane で `AL_KANI_TIMEOUT_MS=600000` とする。設計予算は各 proof 10 分以内・4 GiB 未満、wav-chunk-scan 選択実行合計 10〜20 分を目標とする。legacy proof を含む全件 Kani lane はこの slice の必須検証にしない。proof が timeout/Fail なら、solver や timeout だけを変えて再試行せず、分解と boundary を見直す。Required Pass が得られなければ proof/contract を弱めず Required NotRun/Fail として報告する。

## 8. mutation detection plan

Able to Fail 証拠として、次を別々に行う。

- boundary arithmetic mutation: `body_end` の off-by-one または pad 加算漏れなど、boundary 系 proof が検出すべき一時変更。
- scan progress mutation: `next_offset` を進めない、または pad を二重に進めるなど、scan progress 系 proof が検出すべき一時変更。

diagnostic payload だけの mutation は使わない。各 mutation は wav-chunk-scan canonical lane で少なくとも 1 件の `VERIFICATION:- FAILED` を確認した後、完全 revert し、通常 lane を再実行する。検出されなければ mutation が不適切か oracle が弱いかを区別し、weakened Pass にせず ledger に残す。

## 9. proptest との関係

WAV-CHUNK-003 の method は `Kani local + proptest mutation` である。既存 `mutated_or_truncated_headers_never_panic` は no-panic を広く調べるが、pad/off-by-one/trailing bytes の structural oracle を直接検査するものではない。不足があれば `formal_wav.rs` に小さい structural regression を追加する。case budget を不必要に増やさず、formal proof の代替にしない。

## 10. RF 還元判定

RIFF chunk walker の boundary arithmetic は汎用化できる可能性がある。ただし現時点の接続条件は AudioLens の WAV scan invariant、error precedence、`MediaError` 写像に結び付いており、実証済みの再利用契約として切り出せる段階ではない。本 slice では Ruviform Issue を起票せず、候補と理由を PR 本文に記録する。

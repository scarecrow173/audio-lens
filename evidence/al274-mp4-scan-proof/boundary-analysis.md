# MP4-SCAN-003 exact proof boundary analysis

対象は Issue #274 の MP4-SCAN-003（ISO-BMFF child box scan の進行性・終了性）である。本書は solver 実験や runner 変更に先立つ境界設計であり、PR #284 で Required Pass 済みの MP4-BOX-002、PR #282/#283 で Required Pass 済みの WAV proof を再設計しない。MP4/AVI boundary proof と WAV proof は runner 変更の regression としてだけ再実行する。

## 1. Property の選定

MP4-SCAN-003 は top-level file scan と、`moov`、`trak`、`edts`、`mdia`、`minf`、`stbl`、`stsd`、sound sample entry の nested box walk に共通する scan Property である。production scan は valid box ごとに `body_end` を次 offset とし、exact end でだけ正常終了し、1..7 byte trailing や invalid box で継続しない必要がある。この性質が崩れると、scan が停滞する、trailing bytes を読み飛ばす、first invalid を見落とす、または valid box の並びを誤って解釈する。

この Property を選ぶ理由は、残る Required formal gap のうち最小の責務へ分解できるためである。PR #284 の pure boundary helper `mp4_box_boundary_step` と、PR #283 の finite unrolled scan proof pattern を直接再利用でき、`Vec` 展開、u128 timing 除算、codec matrix、複数 stream 累積を必要としない。MP4-TABLE-005、MP4-PCM、AVI-MOVI-003、AVI-TIME-004、AVI-PCM はこれらの重い依存を持つ。signature 再構成は軽いが、既存 `detect_format_signature_rules` が Pass しており追加保証の差分が小さい。完全な cost matrix と全体 mutation matrix は新規 proof 追加直後に再度 stale になるため、この slice では限定的比較に留める。

## 2. exact input domain

scan model の入力は `(start: u64, end: u64)` と、高々 4 回分の `(size32: u32, largesize: u64)` 列である。`start` と `end` は production index を写す u64 scalar であり、slice 長や enclosing box length で事後 narrowing しない。`size32` は full u32、`largesize` は full u64 とする。`largesize` は size32 == 1 の branch でだけ観測される exact reparameterization であり、size32 != 1 の入力では値が結果へ影響しない。

`start <= end` は production `child_boxes` の constructor/caller precondition として使う。事後 narrowing ではない。根拠は次の通りである。top-level scan は `0 <= data.len()` から始まる。`parse_stsd` は `body.len() >= 8` を確認してから `(8, body.len())` を渡す。sound sample entry は `body.len() > children_at` のときだけ `(children_at, body.len())` を渡す。それ以外の nested call は valid `BoxHeader` が返す `body_start <= body_end` を渡す。したがって、公開入口から `start > end` の scan へは到達しない。

`start > end` は production 到達外の total extension とし、scan position helper では trailing bytes 側へ分類する。proof domain は constructor-admitted な `start <= end` に限定する。これは保証を縮める事後 narrowing ではなく、private scan function の呼出側 invariant である。将来 public constructor や別 caller がこの invariant を変える場合は、proof domain と接続条件を再評価する必要がある。

small scan model は、terminal または invalid に達するまでに受理する valid box 数が高々 3 である入力列を対象にする。4 つ目の symbolic size/largesize までは、3 valid box の後に invalid box で終わる列を exact に覆うために使う。4 valid box 以上を受理する列だけが small model の外側である。step 数 3 の根拠は、scan 初期状態、valid box 後の非初期状態、さらにその次の合成を同じ proof で検査できる最小余裕を持つためである。1 step では scan 合成を検査できず、2 step でも terminal/invalid の合成は検査できるが、3 step のほうが mutation 検出と判読性の余裕が大きい。full u64/full u32 arithmetic のまま実用予算に入るかを実測し、入らなければ proof を弱めず原因を ledger に残す。

## 3. 証明する性質

scan proof は次を検査する。

- 空 region（`start == end`）は即座に complete になる。
- exact end だけを complete として受理する。
- 1..7 byte trailing は trailing success ではなく、production scan では既存どおり truncated box header error へ写像される terminal 分類である。
- valid box ごとに `next_offset = body_end` で進み、少なくとも header 分の strict progress がある。
- valid box ごとに offset が単調増加し、`end` を越えない。
- valid box ごとに残余量 `end - offset` が厳密に減る。
- valid step の `body_start`/`body_end`/end containment は、その step の `mp4_box_boundary_step` 事後条件と一致する。
- first invalid box で scan を止める。proof model は invalid outcome を返し、後続 box を valid として受理しない。
- checked arithmetic によって overflow/underflow/panic がない。
- 高々 3 valid box の small model 内では、symbolic size32/largesize 列、start/end、terminal/invalid 条件の全組合せについて、上の進行性と終端分類が成立する。

one-step measure decrease は exact proof の一部として検査する。ただし、任意回数の合成による帰納的 termination を Kani が機械的に証明したとは読み替えない。small model は scan transition の境界と terminal 判定を exact に覆い、measure decrease は finite region length に対する数学的 termination 根拠を与える。無制限 box 数の termination は機械証明済み領域に入れず、ledger の未証明領域に残す。

## 4. 検証しない領域

この local proof は次を証明しない。

- MP4 semantic box type、duplicate/required box、skipped box path。
- sample table、PCM declaration/extraction、writer fixture、writer round-trip。
- deep nesting performance、多数 box scan、4 valid box 以上を受理する scan。
- I/O、allocation、`Vec` decode、diagnostic formatting。
- signature 判定、format detection、mutation/truncation を含む whole-parser integration。
- 公開 API 全体 no-panic。
- AVI の scan、movi、timing、PCM。

これらは未証明領域として ledger に残す。local proof Pass を上位保証へ転用しない。

## 5. production への接続条件

True to the Code の接続条件は次の通りである。

1. production scan は `mp4_scan_position` で exact end、trailing bytes、header availability を判定する。
2. header availability がある場合だけ production `box_header` が size32/largesize/tag を読み、boundary arithmetic を `mp4_box_boundary_step` へ委譲する。
3. valid box の次 scan offset は、production と proof model の両方が `mp4_box_scan_next_offset` を使って `body_end` から導く。
4. top-level file scan と nested `child_boxes` は同じ scan position/transition helper を使う。tag 解釈、error 文言生成、`Vec<BoxHeader>` 収集は production shell に残し、boundary/offset 計算を複製しない。
5. trailing bytes の公開 error kind/text は既存どおり `{context}: truncated box header` とする。scan position helper を導入しても error precedence、公開 API、対応形式、正常/異常系の振る舞い、f64 精度、partial ingest/capability semantics は変えない。
6. helper は I/O、`Vec` allocation、sample table decode、PCM decode、`std::fmt` 診断文字列構築を実行しない。

scan composition 全体を test-side model に残す案も検討したが、terminal 判定と next offset 計算まで test-side にすると mutation oracle が弱い。そこで、最小の production scan position helper と next-offset transition helper を抽出する。反復制御と error propagation は caller context 依存なので production loop に残す。`cfg(kani)` 置換、fmt/alloc stub、writer fixture、`Cursor`、full parser、sample table `Vec` decode は proof target に入れない。

## 6. assume と coverage の分類

| assume / bound | 分類 | 根拠 |
| --- | --- | --- |
| `start <= end` | exact constructor/caller precondition | private scan caller は empty region または valid box body range だけを渡す |
| `largesize` を全入力で symbolic にする | exact reparameterization | size32 != 1 では観測されず、coverage を制限しない |
| 高々 3 valid box | small scan model の明示 bound | 4 valid box 以上だけを未証明領域に残し、size/end/start の数値領域は縮めない |
| valid step の残余量減少を使う合成 | exact one-step proof + 人間の数学的合成 | bounded model の成功を unbounded termination と読み替えない |

assume の充足可能性は、empty region、exact end、1..7 byte trailing、size8 zero-body、size0 terminal、size32/largesize valid step、body overrun、複数 box、first invalid を通る concrete unit testで示す。主要経路は既存 integration/proptest でも到達する。未証明の数学的事実を assume しない。

## 7. decomposition 候補の比較

### finite unrolled scan model + production scan transition helpers（採用）

宣言した small step 数まで symbolic size32/largesize 列を unroll し、strict progress、exact end、trailing/invalid 判定を検査する。PR #283 の WAV scan proof pattern に倣う。加えて、terminal 判定と next offset 計算を production helper へ抽出し、test-side model と production loop が同じ transition を使うようにする。step 数を結果に合わせて後から下げない。無制限 scan termination に読み替えない。

採用理由は、MP4-SCAN-003 が boundary helper だけでは覆えない composition Property であり、finite model が first invalid、exact end、trailing、複数 box の合成を直接検査できるためである。helper 抽出は小さく、公開挙動を変えずに mutation oracle を強められる。

### single-step measure-decrease のみ（不採用）

PR #284 の `mp4_box_header_step_exact` が持つ one-step progress/measure decrease を再利用する案である。finite scan composition、first invalid 停止、exact end/trailing 判定を検査できない。MP4-SCAN-003 の主 proof としては不十分なので採用しない。one-step measure decrease は scan proof の assertion として併用する。

### production scan transition/cursor helper + finite model（一部採用）

scan composition の True to the Code と mutation oracle を強めるため、terminal 判定と next offset 計算の最小 helper を抽出する案である。完全な cursor object や反復制御まで抽出すると、usize/u64 conversion と error context をまたぐ abstraction が増え、今回必要な保証に対して複雑になる。そこで scan position と next offset の 2 helper に限定する。反復制御と error propagation は既存 loop に残し、unit test と code inspection で接続する。

### function contract / loop invariant / unbounded termination decomposition（不採用）

### function contract / loop invariant / unbounded termination decomposition（不採用）

多数 box や無制限 scan に強い案だが、contract/loop invariant の unstable flag、proof-for-contract registry、runner 意味論の追加管理を伴う。今回の対象は高々 3 valid box の bounded exact proofであり、最初の Required proof としては過剰である。採用しない。将来採用する場合は proof-for-contract も Required proof として registry と canonical lane に含める。

### direct `child_boxes` proof（不採用）

`Vec` 収集、byte slice、tag 読出し、error 文言を含むため、scalar pure boundary proof より重い。検証対象が boundary arithmetic と scan composition から integration へ広がり、PR #282〜#284 で確立した小さい proof の費用対効果を失う。採用しない。

## 8. runner 戦略と予算

runner には `mp4-scan` assurance group を追加し、Required proof `mp4_box_scan_progress_bounded_exact` を漏れなく選択する。既定全件挙動、fail-closed、WSL mirror、target 配置、timeout 意味論、group/subset 排他意味論は変えない。per-harness elapsed ms、raw log、pass/fail/not_run、`/usr/bin/time -v` が利用できる環境での max RSS、lane-level `cost_summary` を維持する。

単一 proof timeout は canonical lane で `AL_KANI_TIMEOUT_MS=600000` とする。設計予算は proof 10 分以内・4 GiB 未満、mp4-scan 選択実行合計 10〜20 分を目標とする。legacy proof を含む全件 Kani lane はこの slice の必須検証にしない。proof が timeout/Fail なら、solver や timeout だけを変えて再試行せず、分解と boundary を見直す。Required Pass が得られなければ proof/contract を弱めず Required NotRun/Fail として報告する。

## 9. mutation detection plan

Able to Fail 証拠として、scan progress/composition を壊す一時 mutation を入れる。第一候補は `mp4_box_scan_next_offset` が `body_end` ではなく現在 offset を返す進行停滞である。期待は mp4-scan proof の少なくとも 1 件に `VERIFICATION:- FAILED` が出ることである。boundary proof だけが Fail する mutation を scan oracle の証拠とみなさない。diagnostic payload だけの mutation は使わない。確認後に完全 revert し、最終状態で canonical lane を再実行する。検出されなければ mutation が不適切か oracle が弱いかを区別し、weakened Pass にしない。

## 10. proptest との関係

既存の mutation/truncation/no-panic proptest は弱めない。formal proof は scalar pure scan/boundary arithmetic を exact に検査し、proptest は public integration を統計的に検査する。不足する scan terminal/trailing/zero-body/multiple-box boundary は `mp4.rs` の deterministic unit testで覆う。`formal_containers.rs` は変更しない。

## 11. RF 還元判定

ISO-BMFF scan walker は汎用化できる可能性がある。ただし現時点の接続条件は AudioLens の error precedence、`MediaError` 写像、box semantic、partial ingest に結び付いており、実証済みの再利用契約として切り出せる段階ではない。本 slice では Ruviform Issue を起票せず、候補と理由を PR 本文に記録する。

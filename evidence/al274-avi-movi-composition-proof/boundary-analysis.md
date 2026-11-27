# AVI-MOVI-003 movi entry composition exact proof boundary analysis

対象は Issue #274 の AVI-MOVI-003 に残る nested movi/list composition のうち、`movi` list 直下 1 件の entry を packet、ignore、short-list reject、rec-list reject へ分類する semantic decision である。本書は solver 実験や runner 変更に先立つ境界設計であり、AVI-MOVI-003 全体、AVI-TIME-004、MP4-TABLE-005、MP4/AVI PCM を完了させるものではない。PR #284 の `avi_chunk_step_exact`、PR #286 の `avi_movi_packet_tag_exact`、PR #282/#283/#285 の WAV/MP4 proof は再設計せず、runner 変更の regression としてだけ再実行する。

## 1. Property の選定

production parser は `movi` list 直下の chunk について、chunk id が exact `LIST` のときだけ list type view を検査する。list type を読むための 4 byte がなければ malformed、先頭 4 byte が byte 順と末尾 space まで exact に `rec ` なら input unavailable、それ以外の direct child LIST は再帰せず無視する。chunk id が `LIST` 以外なら list type view は観測せず、`avi_movi_packet_tag` が受理する digit tag のときだけ packet として収集し、それ以外を無視する。

この decision は boundary walker output と tag identity の間に残る未証明の分岐責務である。`avi_chunk_step_exact` は chunk header/body/pad の境界を証明し、`avi_movi_packet_tag_exact` は digit prefix と kind suffix の意味を証明するが、direct child `LIST`、short list type、exact `rec ` rejection、non-rec LIST ignore、packet/ignore 選択の合成はまだ証明していない。full nested walker や finite iteration を直接対象にすると、iterator state、`Result`/`MediaError` precedence、body slice/lifetime、outer RIFF scan、複数 entry ordering、`Vec<MoviPacket>` accumulation を同時に扱う必要がある。この slice はその integration を避け、1 entry の pure decision core を exact に閉じる。

MP4-TABLE-005 は `stts`/`ctts`/`stsc`/`stsz`/`stco` accumulation と cross-table consistency、AVI-TIME-004 は u128 除算・remainder・複数 stream/packet accumulation、PCM 系は conversion matrix を持つ。signature 再構成は軽いが既存 `detect_format_signature_rules` が Pass しており、追加保証の差分が小さい。完全な cost matrix と全体 mutation matrix は新規 Required proof 追加直後に再度 stale になるため、この slice では限定的比較に留める。legacy retire は代替 coverage と Coordinator 承認が不足するため対象外である。

## 2. exact input domain

proof の入力は次の 2 つだけとする。

1. `id: [u8; 4]` — full 2^32 bit pattern。digit tag、non-digit prefix、exact `LIST`、unknown chunk id、non-ASCII byte を sampling しない。
2. `list_type_view: Option<[u8; 4]>` — `None` と full `[u8; 4]` の全 bit pattern。`None` は direct child LIST body length が 4 未満で list type を読めない場合、`Some(bytes)` は body length 4 以上の先頭 4 byte を表す exact reparameterization である。

body tail、padding、chunk size、offset はこの decision core の入力ではない。production decision は direct child `LIST` の length-under-4 と先頭 4 byte だけを観測し、body tail を読まない。したがって body tail をモデル化しないのは input narrowing ではなく、観測 interface をそのまま写した exact reparameterization である。packet body boundary、padding、offset 進行は PR #284 の `avi_chunk_step_exact` が担う別層であり、tag digit/kind arithmetic は PR #286 の proof が担う。

`id != LIST` では production は list type view を観測しない。proof がそれでも symbolic `list_type_view` を渡すのは、coverage を落とさず decision がこの入力に依存しないことを検査する total extension である。`None` と `Some` のどちらでも、non-LIST entry の packet/ignore 判定が変わらないことを exact complement で検査する。digit-only、valid LIST-only、writer fixture-derived entry、representative fourcc matrix への限定は禁止である。exactness を保てない input domain しか出ない場合は実装へ進まず Coordinator へ確認する。

## 3. 証明する性質

proof は 1 entry の semantic decision の total correctness を対象にする。検査する性質は次の通りである。

- `id == LIST` かつ `list_type_view == None` のときだけ short-list reject を返す。
- `id == LIST` かつ `list_type_view == Some(b"rec ")` のときだけ rec-list reject を返す。比較は byte 順、大文字小文字、末尾 space を含む exact equality であり、`Rec `、`recX`、その他の complement は reject しない。
- `id == LIST` かつ `list_type_view` が `Some` の exact `rec ` complement なら ignore を返し、packet へ誤分類しない。
- `id != LIST` なら `list_type_view` に依存せず、`avi_movi_packet_tag(id)` が `Some` のときだけ packet を返す。packet variant は digit prefix、decimal stream number 0..=99、kind byte の順序保持を失わない。
- `id != LIST` かつ tag helper が `None` なら ignore を返す。unknown chunk id や non-digit prefix を packet へ誤分類しない。
- packet、ignore、short-list reject、rec-list reject は exact complement であり、重複・欠落・中間状態がない。
- short-list reject と rec-list reject を packet tag 判定と混線させず、LIST entry を packet として返さない。
- decision 計算で overflow、underflow、panic がない。

この proof は finite machine integer domain 全体に対する total correctness proof であり、representative sampling ではない。entry composition proof Pass を finite movi iteration、`Vec` accumulation、nested walker 全体、AVI-MOVI-003 全体、AVI-TIME-004 の機械証明と読み替えない。

## 4. 検証しない領域

この local proof は次を証明しない。

- AVI RIFF/LIST walker 全体、outer `hdrl`/`movi` scan、missing `movi`。
- 複数 entry の iteration、termination、ordering、reject 後停止。
- `Chunks` iterator の error precedence、`MediaError` 文言生成、body slice/lifetime。
- `Vec<MoviPacket>` accumulation、packet body payload、nested non-rec LIST の内容、OpenDML `rec ` の構造的解釈。
- stream declaration（`strh`/`strf`）、stream number と実在 stream の対応、stream existence、kind validity/meaning matrix。
- timing accumulation、remainder、PCM/video decode、partial ingest。
- writer round-trip、corruption whole-call proof、I/O、allocation、diagnostic formatting、公開 API 全体 no-panic。
- MP4、signature 再構成。

これらは未証明領域として ledger に残す。local proof Pass を上位保証へ転用しない。

## 5. production への接続条件

True to the Code の接続条件は次の通りである。

1. 最小 production helper `avi_movi_entry_decision` を `crates/al-media/src/avi.rs` に抽出する。helper は full `[u8; 4]` chunk id と exact list-type view `Option<[u8; 4]>` を受け取り、packet、ignore、short-list reject、rec-list reject の decision だけを返す。
2. helper は `MediaError`、error 文言、body slice、allocation、I/O、`Vec` 収集を持たない。packet variant では `avi_movi_packet_tag` が返す stream number と kind をそのまま保持する。
3. production `movi` loop は direct child LIST/short/rec/packet/ignore の分岐を複製せず、`Chunks::next` が返した同じ inner chunk id と、`LIST` のときだけ body から作った list-type view をこの helper へ渡す。
4. loop shell は現行どおり short list type を `MediaError::malformed("movi LIST shorter than list type")`、exact `rec ` を `MediaError::input_unavailable("OpenDML/nested rec LIST is not supported")` へ写像する。decision helperは error の種別や文言を作らない。
5. `Chunks::next` の boundary/offset 計算、body slice、padding、iterator error precedence、`Vec<MoviPacket>` 収集は shell に残す。公開 API シグネチャ、公開 error の種別/文言、error precedence、正常/異常系の振る舞い、f64 精度、対応形式、partial ingest/capability semantics、stream/timing/video/audio 解釈を変えない。
6. boundary walker と semantic decision の接続は、同じ production `Chunks::next` が inner chunk id/body を返し、movi loop がその値を変更せず helper へ渡すことで確認する。finite iteration と nested walker 全体の機械証明はこの slice に含めない。

entry decision-only proof では True to the Code を保てず、combined movi transition helper、bounded scan model、iterator 再設計が必要だと判明した場合は、広範な再設計へ進まず Coordinator へ確認する。

## 6. assume と coverage の分類

| assume / bound | 分類 | 根拠 |
| --- | --- | --- |
| なし | 該当なし | input narrowing を置かない |
| `id` を full `[u8; 4]` とすること | exact input domain | production chunk id と同じ表現であり、digit/list/unknown/non-ASCII を漏れなく表す |
| direct child LIST body を `Option<[u8; 4]>` へ写すこと | exact reparameterization | production decision は length-under-4 と先頭 4 byte だけを観測し、body tail を読まない |
| non-LIST entry に symbolic list-type view を渡すこと | total extension | production 到達外または unobserved の入力だが、decision が影響を受けないことを coverage を落とさず検査する |
| `Some`/`None`、rec/other list type、packet/non-packet を 1 proof で検査すること | exact complement の branch selection | 分類の片側を省かず、complement の重複・欠落を検査する |
| body tail、padding、offset、packet payload を入力に含めないこと | boundary layering | これらは decision core が観測しない。boundary arithmetic は `avi_chunk_step_exact`、packet payload は後続 slice の責務である |

assume の充足可能性と主要経路到達性は、exact `rec `、short movi LIST、`Rec `/`recX` などの non-rec exact complement、ignored LIST 後の packet、non-LIST unknown chunk、kind byte order、non-frame `pc`、stream 00/99 を通る deterministic regression で示す。未証明の数学的事実を assume しない。

## 7. decomposition 候補の比較

### single total correctness entry decision proof（採用）

full chunk id と exact list-type view について packet/ignore/short-list reject/rec-list reject を 1 proof で検査する。loop、allocation、I/O、body slice、`Vec` を持たず、PR #286 の scalar pure boundary proof pattern に倣える。分岐は equality 判定と digit range 判定だけで、input domain を縮めずに exact complement を表せる。registry・runner・evidence を必要以上に増やさないため採用する。harness 名は `avi_movi_entry_decision_exact` とする。

### LIST/non-LIST exact complement family（不採用）

direct child LIST decision と non-LIST packet/ignore decisionを 2 proof に分ける案である。assertion は小さくなるが、入力は byte array と Option byte arrayだけで、単一 proof が exact complement を直接表せる。分割すると registry と再実行単位が増える一方、non-LIST decision が list-type view に依存しない性質は単一 proof の方が自然に表せる。proof が実用予算を超えた場合でも input domain は縮めず、分解の必要性と原因を ledger に残して判断する。

### bounded finite unrolled movi scan proof（不採用）

高々数件の direct child entry 列について collect/ignore/reject の順序を検査する案である。AVI-MOVI-003 全体へ近いが、iterator state、entry 数、reject 後停止、collection ordering、`Vec` accumulation を導入する。今回の未証明分岐は 1 entry decision に閉じており、最初の composition boundary としては重い。entry decision の接続条件が弱いと判明した場合にだけ再検討し、広範な再設計になるなら Coordinator へ確認する。

### combined chunk boundary + entry composition proof（不採用）

`avi_chunk_step` の boundary result と entry decisionを 1 つの movi transition helperへ合成する案である。boundary walker output と semantic decision の接続を強める一方、offset/size arithmetic、body slice、error pathを同じ proofへ戻し、PR #284 で証明済みの層を重複させる。今回の decision は walker が返す id/body を変更せず使う条件で独立に検査でき、boundary proofの保証を増やさない。合成条件を追加する必要が出た場合は、exact coverage が重複証明ではないことを先に示す。

### direct movi iterator/full nested composition proof（不採用）

byte slice、`Result`、error 文言、`Vec<MoviPacket>`、nested list semantics、outer scan を含むため integration へ広がり、この slice の bounded exact proofとしては大きすぎる。最初の composition boundary proofとしては費用対効果が低い。finite iteration/termination/ordering は後続の heavy proof 残件として ledger に残す。

### AVI-TIME-004 accumulation proof（不採用）

`tick_advance`、remainder、複数 stream/packet clock まで進める案である。u128 除算と multi-stream accumulation を含み、別の heavy Required property である。movi entry decision の前件ではなく、この slice の対象外なので採用しない。

## 8. runner 戦略と予算

runner には `avi-movi-composition` assurance group を追加し、Required proof `avi_movi_entry_decision_exact` を漏れなく選択する。既定全件挙動、fail-closed、WSL mirror、target 配置、timeout 意味論、group/subset 排他意味論は変えない。per-harness elapsed ms、raw log、pass/fail/not_run、`/usr/bin/time -v` が利用できる環境での max RSS、lane-level `cost_summary` を維持する。

単一 proof timeout は canonical lane で `AL_KANI_TIMEOUT_MS=600000` とする。設計予算は proof 10 分以内・4 GiB 未満、avi-movi-composition 選択実行合計 10〜20 分を目標とする。legacy proof を含む全件 Kani lane はこの slice の必須検証にしない。proof が timeout/Fail なら、solver や timeout だけを変えて再試行せず、分解と boundary を見直す。Required Pass が得られなければ proof/contract を弱めず Required NotRun/Fail として報告する。

## 9. mutation detection plan

Able to Fail 証拠として、movi entry composition semantic を壊す一時 mutationを入れる。第一候補は short list type reject を無効化し、`list_type_view == None` を ignore へ落とす変更である。期待は avi-movi-composition proof の少なくとも 1 件に `VERIFICATION:- FAILED` が出ることである。generic boundary proof や既存 tag proof だけが Fail する mutationを composition oracle の証拠とみなさない。diagnostic payload だけの mutationは使わない。確認後に完全 revert し、最終状態で canonical laneを再実行する。検出されなければ mutation が不適切か oracle が弱いかを区別し、weakened Pass にしない。

## 10. proptest との関係

既存の mutation/truncation/no-panic proptest は弱めない。formal proof は 1 entry の pure semantic decision を exact に検査し、proptest は public integration を統計的に検査する。exact `rec ` rejection、short movi LIST malformed、`Rec `/`recX` などの exact complement non-rec LIST ignore、ignored LIST 後の packet収集、non-LIST unknown chunk ignore、kind byte order、non-frame `pc`、stream 00/99 のうち既存 testで不足する境界だけを `avi.rs` の deterministic unit testへ追加する。`formal_containers.rs` は変更しない。

## 11. RF 還元判定

RIFF/AVI movi direct child entry decision は形式仕様として汎用化できる可能性がある。ただし現時点の接続条件は AudioLens の OpenDML rejection、error 文言、stream 解釈、partial ingest、walker error precedence に結び付いており、実証済みの再利用契約として切り出せる段階ではない。本 slice では Ruviform Issue を起票せず、候補と理由を PR 本文に記録する。

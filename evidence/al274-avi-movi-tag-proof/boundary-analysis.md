# AVI-MOVI-003 movi packet tag exact proof boundary analysis

対象は Issue #274 の AVI-MOVI-003 に含まれる movi packet tag の semantic identity である。本書は solver 実験や runner 変更に先立つ境界設計であり、AVI-MOVI-003 全体、AVI-TIME-004、MP4-TABLE-005、MP4/AVI PCM を完了させるものではない。PR #284 で Required Pass 済みの AVI-CHUNK-001、PR #285 で Required Pass 済みの MP4-SCAN-003、PR #282/#283 で Required Pass 済みの WAV proof は再設計せず、runner 変更の regression としてだけ再実行する。

## 1. Property の選定

production parser は `movi` list の inner chunk id を先頭から見て、先頭 2 byte が両方 ASCII digit のときだけ packet として収集する。受理した packet では、`NN` を 10 進の stream number、残り 2 byte を kind として保持する。kind は後段で `wb` の audio packet、`db`/`dc` の frame-carrying video packet、`pc` 等の非 frame packet を区別する。digit 判定、桁の解釈、kind byte の順序が壊れると、audio packet 選択、video frame index、timeline slot の意味が変わる。

この sub-boundary を選ぶ理由は、残る Required formal gap のうち最小の責務へ分解できるためである。PR #284 の `avi_chunk_step_exact` は generic chunk boundary walker を証明済みであり、今回の tag identity はその上に乗る scalar pure contract である。`Vec` 展開、u128 timing 除算、remainder、複数 stream/packet 累積、codec matrix を必要としない。MP4-TABLE-005、AVI-TIME-004、AVI-PCM、MP4-PCM はこれらの重い依存を持つ。signature 再構成は軽いが、既存 `detect_format_signature_rules` が Pass しており追加保証の差分が小さい。完全な cost matrix と全体 mutation matrix は新規 proof 追加直後に再度 stale になるため、この slice では限定的比較に留める。

## 2. exact input domain

proof の入力は full `[u8; 4]` とする。4 byte は chunk id をそのまま表し、digit らしい値、ASCII以外、kind byte、unknown suffix、non-digit prefix を sampling せず全 2^32 bit pattern で扱う。digit にだけ絞る assume、representative fourcc matrix、writer fixture 由来の concrete tag への限定は置かない。

`id[0]` と `id[1]` は受理判定だけに使い、`id[2]` と `id[3]` は意味分類せず opaque kind byte として保持する。現行製品契約は unknown suffix を拒否せず kind として保持するため、proof 側に kind validity matrix を追加しない。`wb`、`db`、`dc`、`pc` の意味分類は下流責務であり、この helper の受理条件ではない。

input domain を 4 個の independent u8 scalar へ分解しても `[u8; 4]` と exact に同値だが、実装では array symbolic input を使い、byte 順序の再構成ミスを proof model に持ち込まない。input narrowing が必要になった場合は exactness を維持できないため、実装へ進まず Coordinator へ確認する。

## 3. 証明する性質

proof は 1 回の tag interpretation の total correctness を対象にする。検査する性質は次の通りである。

- 先頭 2 byte が両方 ASCII digit のときだけ packet tag として受理する。
- 受理時の stream number は `stream_no = (id[0] - b'0') * 10 + (id[1] - b'0')` に一致する。
- 受理時の stream number は 0..=99 に収まる。
- 受理時の kind は `id[2]`、`id[3]` をこの順序で保持する。
- 先頭 2 byte の少なくとも一方が ASCII digit でない入力は、kind suffix が `wb`、`db`、`dc`、`pc` 等であっても受理しない。
- 受理と非受理は exact complement であり、中間状態や部分的な受理がない。
- digit 判定と stream number 計算で overflow、underflow、panic がない。

この proof は finite machine integer domain 全体に対する total correctness proof であり、representative sampling ではない。packet body boundary、padding、nested region scan は PR #284 の generic chunk boundary proof が担う別層である。tag proof Pass を nested movi walker 全体や AVI-MOVI-003 全体の機械証明と読み替えない。

## 4. 検証しない領域

この local proof は次を証明しない。

- AVI RIFF/LIST walker 全体、nested `movi` LIST/`rec ` semantics、OpenDML。
- stream declaration（`strh`/`strf`）、stream number と実在 stream の対応。
- kind の妥当性、codec 意味分類、frame-carrying 判定そのもの。
- packet body boundary、padding、chunk/LIST error precedence、`MediaError` 文言。
- timing accumulation、remainder、複数 stream/packet clock。
- PCM/video decode、partial ingest、writer round-trip、corruption whole-call proof。
- I/O、allocation、`Vec<MoviPacket>` 収集、diagnostic formatting、公開 API 全体 no-panic。
- MP4、signature 再構成。

これらは未証明領域として ledger に残す。local proof Pass を上位保証へ転用しない。

## 5. production への接続条件

True to the Code の接続条件は次の通りである。

1. 最小 production helper `avi_movi_packet_tag` を `crates/al-media/src/avi.rs` に抽出する。helper は full `[u8; 4]` chunk id を受け取り、受理時だけ stream number と kind を返す。
2. production `movi` loop は digit 判定、stream number 計算、kind 抽出を複製せず、walker から得た同じ inner chunk id をこの helper へ渡す。
3. `Chunks::next` の boundary/offset 計算は既存どおり `avi_chunk_step` を使う。今回の helper は body slice、padding、error precedence、`MediaError` 文言生成、OpenDML `rec ` rejection、`Vec<MoviPacket>` 収集へ介入しない。
4. tag helper 抽出によって公開 API シグネチャ、公開 error の種別/文言、error precedence、正常/異常系の振る舞い、f64 精度、対応形式、partial ingest/capability semantics、stream/timing/video/audio 解釈を変えない。
5. helper は I/O、allocation、`Vec` decode、`std::fmt` 診断文字列構築を実行しない。
6. generic chunk boundary proof と tag proof の接続は、同じ production `Chunks::next` が inner chunk id と body を返し、movi loop がその id を変更せず tag helper へ渡すことで確認する。nested walker composition 全体の機械証明はこの slice に含めない。

tag-only proof では True to the Code を保てず、combined movi walker helper や iterator 再設計が必要だと判明した場合は、広範な再設計へ進まず Coordinator へ確認する。

## 6. assume と coverage の分類

| assume / bound | 分類 | 根拠 |
| --- | --- | --- |
| なし | 該当なし | input domain は full `[u8; 4]` であり、digits、kind、suffix、stream number を縮めない |
| symbolic byte array を使うこと | exact input domain | `[u8; 4]` は production chunk id と同じ表現であり、2^32 pattern を漏れなく表す |
| digit と non-digit の branch を 1 proof で検査すること | exact complement の branch selection | 受理/非受理の両側を同じ input domain 上で検査し、non-digit complement を落とさない |
| kind byte を semantic classification しないこと | production contract preservation | 現行契約は unknown suffix を opaque kind として保持する |

assume の充足可能性と主要経路到達性は、`00wb`、`99dc`、non-digit first/second prefix、unknown/opaque suffix、kind byte order、non-frame `pc` の保持を通る deterministic regression で示す。既存 integration test は `00db`、`01wb`、`02wb`、`00pc` を通る。未証明の数学的事実を assume しない。

## 7. decomposition 候補の比較

### single total correctness tag proof（採用）

full `[u8; 4]` tag domain について受理/非受理、stream number、kind preservation、算術安全性を 1 proof で検査する。入力は byte array だけで loop、allocation、I/O を持たず、PR #284 の scalar pure boundary proof pattern に倣える。input domain が小さく finite であり、valid/invalid family へ分けなくても exact complement を 1 proof で検査できる。registry、runner、evidence を必要以上に増やさないため採用する。harness 名は `avi_movi_packet_tag_exact` とする。

### valid/invalid exact complement family（不採用）

digit tag 受理と non-digit 非受理を 2 proof に分ける案である。assertion は小さくなるが、full `[u8; 4]` domain は single proof で実用予算に入る見込みが高く、分割によって registry と再実行単位だけが増える。分岐片側の漏れを防ぐ目的は single proof の exact complement assertion で達成できる。proof が実際に budget を超えた場合でも、input domain を縮めず、分解の必要性と原因を ledger に残して判断する。

### combined chunk boundary + movi tag composition proof（不採用）

`avi_chunk_step` の boundary result と tag helper を 1 つの movi packet transition helper へ合成する案である。AVI-MOVI-003 全体へ近いが、production `Chunks` iterator の再構成、body slice、error context、nested LIST semantics の接続を伴う。今回の tag identity は chunk walker が返す id を変更しない条件で独立に検査でき、boundary walker は PR #284 で証明済みである。広範な iterator 再設計はこの slice の最小単位を超えるため採用しない。tag-only proof の接続条件が不十分だと判明した場合は、実装前ではなく判断材料を添えて Coordinator へ確認する。

### direct movi iterator proof（不採用）

byte slice、`Result`、error 文言、`Vec<MoviPacket>`、nested list semantics を含むため integration へ広がり、scalar pure proof より重い。最初の tag boundary proofとしては費用対効果が低い。I/O なしの in-memory iterator でも allocation と error path を含み、今回証明したい digit/kind arithmetic の支配因ではない。採用しない。

### AVI-TIME-004 accumulation proof（不採用）

`tick_advance`、remainder、複数 stream/packet clock まで進める案である。u128 除算と multi-stream accumulation を含み、別の heavy Required property である。movi tag identity の前件として必要ではなく、この slice の対象外なので採用しない。

## 8. runner 戦略と予算

runner には `avi-movi-tag` assurance group を追加し、Required proof `avi_movi_packet_tag_exact` を漏れなく選択する。既定全件挙動、fail-closed、WSL mirror、target 配置、timeout 意味論、group/subset 排他意味論は変えない。per-harness elapsed ms、raw log、pass/fail/not_run、`/usr/bin/time -v` が利用できる環境での max RSS、lane-level `cost_summary` を維持する。

単一 proof timeout は canonical lane で `AL_KANI_TIMEOUT_MS=600000` とする。設計予算は proof 10 分以内・4 GiB 未満、avi-movi-tag 選択実行合計 10〜20 分を目標とする。legacy proof を含む全件 Kani lane はこの slice の必須検証にしない。proof が timeout/Fail なら、solver や timeout だけを変えて再試行せず、分解と boundary を見直す。Required Pass が得られなければ proof/contract を弱めず Required NotRun/Fail として報告する。

## 9. mutation detection plan

Able to Fail 証拠として、movi tag semantic identity を壊す一時 mutation を入れる。第一候補は stream number の tens digit と ones digit を入れ替える変更である。期待は avi-movi-tag proof の少なくとも 1 件に `VERIFICATION:- FAILED` が出ることである。generic boundary proof だけが Fail する mutation を tag oracle の証拠とみなさない。diagnostic payload だけの mutation は使わない。確認後に完全 revert し、最終状態で canonical lane を再実行する。検出されなければ mutation が不適切か oracle が弱いかを区別し、weakened Pass にしない。

## 10. proptest との関係

既存の mutation/truncation/no-panic proptest は弱めない。formal proof は scalar pure tag arithmetic を exact に検査し、proptest は public integration を統計的に検査する。`00wb`、`99dc`、non-digit prefix、unknown/opaque suffix、kind byte order、non-frame `pc` の保持のうち既存 unit/integration testで不足する境界だけを `avi.rs` の deterministic unit testへ追加する。`formal_containers.rs` は変更しない。

## 11. RF 還元判定

RIFF/AVI movi tag の digit prefix と kind suffix の解釈は形式仕様として汎用化できる可能性がある。ただし現時点の接続条件は AudioLens の stream 解釈、unknown suffix の opaque 保持、partial ingest、error precedence に結び付いており、実証済みの再利用契約として切り出せる段階ではない。本 slice では Ruviform Issue を起票せず、候補と理由を PR 本文に記録する。

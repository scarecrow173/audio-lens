---
id: "SPEC-fgr8er1ZTGCLj1pib4Kj8H"
type: "spec"
status: "proposed"
title: "Media Parser Property 起点の形式的検証境界: Kani / Property Test / Lean 検討範囲 (Issue #274)"
created: "2026-11-28"
updated: "2026-11-28"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/audio-lens/issues/274"
  changes:
    added:
      - type: "doc"
        path: "docs/specs/formal-verification-boundary.md"
      - type: "doc"
        path: "docs/knowledge/formal-verification.md"
    modified:
      - type: "dir"
        path: "evidence/al274-boundary/"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements: []
  implemented-by: []
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related: []
  refines: []
  refined-by: []
  derives-from: []
  derived-by: []
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# Media Parser Property 起点の形式的検証境界

本書は、WAV/MP4/AVI Media Parser の Property について、何を Kani で網羅的に証明し、何を proptest/Fuzzing/統合テストに委ね、どこに Lean 等のより強い手法を検討し、どこに形式的検証を適用しないかを定める。判定単位は API やファイルではなく Property と、その保証を構成する局所契約である。

本書は検証境界と粒度の正本である。`tools/run-formal-kani.mjs`、harness 実装、実行時間の履歴は実装・証跡側の記録であり、それらが変わっても本書の保証範囲を自動的に拡張しない。旧 `docs/specs/formal-verification.md` は別 PR が変更中であるため、本書はそれを編集せず自己完結する。

## 用語と判定軸

- **Requirement state**: Required / Deferred / NotApplicable は保証として必要かどうかの軸である。
- **verdict**: Pass / Fail / NotRun / Unsupported は実行済み検証の結果の軸である。Timeout、Kani 環境不足、exit 0、CI green を Proof Pass とみなさない。
- **Bounded Exhaustive local guarantee**: 宣言した入力領域と Bound 内では、production pure helper または production decision step の性質を網羅的に検査した保証である。公開 API 全体の任意長入力に対する no-panic 証明ではない。
- **Property Tested**: proptest による sampled assurance である。Kani の exhaustive assurance と同一視しない。
- **legacy whole-call proof**: `decode_*` や writer output decode を直接 symbolic に呼ぶ proof。実用予算内で verdict を返さない場合は Required assurance として扱わない。

## 境界の判定基準

Property は次の順で分類する。

1. **Kani local proof に置く**: 入力がビットベクトル・固定長列・小さい有限状態列で表現でき、I/O、allocation、diagnostic 文字列、外部ライブラリを含まない。純粋算術、境界計算、exact complement、分類、短い走査の進行性が対象である。
2. **proptest/Fuzzing/統合テストに委ねる**: 入力空間が広く、round-trip、破損、partial ingest、実ファイル構造との相互作用、statistical robustness が主目的である。探索の広さと回帰防止を重視する。
3. **Lean 等のより強い手法を検討する**: 任意長・任意回数の算術的性質で、bounded model checking の展開 bound を取り除いても証明したい場合。現時点では導入可否は open decision とし、実装依存・証明工学コスト・CI コストを後続で評価する。
4. **形式的検証を適用しない**: 製品契約として未定義、他レイヤの責務、観測不能な内部実装詳細、またはコストに対し検出価値が低い範囲。残留リスクとして明示する。

Kani local proof は「pass した local 契約」を証明するだけであり、それを公開 API 全体 no-panic や parse correctness 全体へ転用しない。公開 API の保証は、unit/integration/proptest と明示された未証明領域の管理で構成する。

## 共通の実行予算と支配要因

Required local proof の予算目標は、1 proof あたり 10 分以内・peak memory 4 GiB 未満、WAV/MP4/AVI sequential assurance pack で 60 分以内、関連変更に対する property 単位の選択実行で 10〜20 分以内である。現行 main 統合済みの local proof evidence は多くが数秒〜40 秒、peak RSS は概ね 256〜823 MiBである。

支配要因は次である。

- 非線形乗算、除算、広い u64 状態を組み合わせた bitvector solving。
- `Vec`/slice allocation、panic machinery、diagnostic formatting、`std::fmt::format` 展開。
- API 全体を通す長い制御フローと、symbolic byte 列の全経路探索。
- 無制限または大きな反復。small scan model の 3 要素を超える列、深い nested box/LIST 構造。

Kani 実行は local exhaustive assurance を返せる場合に限定する。timeout した proof は Fail ではないが Pass でもなく、NotRun として扱う。

## Kani で網羅的に証明する範囲

### 対象となる性質

Kani local proof の第一候補は次の性質である。

- offset/size/padding/end の checked arithmetic、region containment、exact complement。
- scan step の strict progress、残余量の単調減少、terminal exact end、trailing byte 判定。
- signature/FOURCC/kind/classification の全ビット列に対する exact complement。
- 固定長 header prefix の byte offset extraction。
- 独立 oracle と比較できる算術。例えば widened integer oracle による overflow 判定。
- 小さい有限列における collect/skip/stop、ordering、first error stop。

### 現行 assurance pack の成立状況

現行 registry には 42 件の canonical harness がある。うち baseline `c91b8ce20c90733fa21098451cb768a74425884d` 以降に追加された local proof は 23 件で、対応 evidence は Pass を記録している。`spike_` 12 件は canonical registry に混ぜない。

| Property / 代表 proof | 証明範囲 | 現行 verdict | 明示された残り |
| --- | --- | --- | --- |
| WAV-SIG-001 / `format_signature_detection_bounded_exact` | len と固定 12 byte prefix、RIFF precedence、non-fallback、public shell matrix | Pass | signature は decode capability を意味しない |
| WAV-CHUNK-003 / `wav_chunk_boundary_step_exact` | chunk size/pad/end、region containment、progress | Pass | 大きな chunk 数、未知 chunk semantic、I/O、allocation |
| WAV-SCAN-004 / `wav_chunk_scan_progress_bounded_exact` | 3 valid chunk までの small scan model、strict progress/terminal/trailing | Pass | 4 valid chunk 以上、無制限 scan termination |
| WAV-RANGE-008 / `wav_range_plan_valid_ranges_exact`, `wav_range_plan_invalid_ranges_error` | constructor-admitted layout の valid/invalid exact complement、frame/byte extent | Pass | I/O、allocation、PCM decode、diagnostic formatting |
| MP4-SIG-001 / `format_signature_detection_bounded_exact` | `ftyp` offset/size/version の exact detection | Pass | free/mdat 先頭 MP4、decode capability |
| MP4-BOX-002 / `mp4_box_header_step_exact` | size32/size64/size0、minimum size、overflow、limit、progress | Pass | box semantic、sample table、PCM |
| MP4-SCAN-003 / `mp4_box_scan_progress_bounded_exact` | 3 valid box までの scan model | Pass | 4 valid box 以上、深い nesting、box semantic |
| MP4-PCM-DECL-006 / `mp4_pcm_declaration_classification_exact`, `mp4_pcm_capability_width_exact` | fixed scalar sound-entry declaration domain での PCM classification、ISO pcmC、lpcm flags/defaults、unknown complement、zero channel/rate capability gate、sample-width matrix | Pass | parser integration、sample table、PCM extraction/conversion、payload semantics、diagnostics、I/O、allocation |
| AVI-CHUNK-001 / `avi_chunk_step_exact` | RIFF chunk boundary/padding/progress | Pass | nested LIST/OpenDML/idx1 semantic |
| AVI-MOVI-003 / tag/entry/scan collection proof | decimal stream prefix、LIST/rec 判定、3 entry collect/skip/stop | local proof は Pass、Property 全体は NotRun | body boundary/lifetime composition、4件以上、実 Vec accumulation、無制限 iteration |
| AVI-STREAM-002 / kind/strh/strf/capture/presence proof | stream kind、strh/strf fixed header extraction、capture ordering、strh presence | local proof は Pass、Property 全体は NotRun | 任意長 nested walker、複数 strl、完全な declaration correspondence |
| AVI-TIME-004 / `avi_stream_start_tick_exact` | 全 u32 start/scale 入力対の start tick と overflow exactness | local proof は Pass、Property 全体は NotRun | remainder、packet increment/accumulation、stream separation |
| AVI-PCM-005 / audio strf declaration、wb concatenation proof | WAVEFORMATEX fixed header extraction、3 packet/0..2 byte modelでの first-audio wb copy ordering | local proof は Pass、Property 全体は NotRun | PCM conversion semantics、任意長 copy、dynamic Vec、missing/empty behavior |

Required local proof が Pass した項目でも、上表の残りは証明済みにしない。AVI-MOVI-003、AVI-STREAM-002、AVI-TIME-004、AVI-PCM-005 のように一部 local proof だけが揃っている Property は、Property 全体 verdict として NotRun を維持する。

### 新たに Required とすべき Kani local proof

現行未実装で、必要保証が Bounded Exhaustive local のものは次である。

- WAV-RIFF-002: RIFF size と入力長の整合。
- WAV-PCM-LAYOUT-006: format tag/bits/channels/rate/block_align/byte_rate の validation。
- WAV-PCM-CONV-007: PCM byte→f64 変換と writer encode の clamp/round/cast。
- MP4-TABLE-005: sample table offset/count/size/time 計算。
- MP4-PCM-EXTRACT-007: PCM extraction offset/width conversion。
- AVI-TIME-004: remainder、packet increment/accumulation、stream separation。
- AVI-PCM-005: PCM decode conversion semantics。

これらは whole-call symbolic proof ではなく、production pure helper 単位へ分割して設計する。実装は後続 sliceであり、本書は範囲だけを定義する。

## proptest/Fuzzing/統合テストに委ねる範囲

次は Property Tested を主保証とする。

- writer/reader round-trip。符号化方式・channels/rate/sample length の広い組合せを探索する。
- 破損・切断入力で panic しないこと。mutation/truncation の組合せは入力空間が大きく、形式的 exhaustive より広域探索が有効である。
- MP4 box 構造、必須 box、skipped boxes、declared facts の統合整合。
- codec declaration/capability、compressed/missing audio の partial ingest。
- AVI video metadata/features、missing/undecodable auds の partial ingest。
- writer の header/backpatch/padding 全体。個別算術だけを Kani に切り出し、writer output decode round-trip は proptest primary にする。

proptest の Pass は exhaustive proof ではない。一方で、Kani proof が Pass しても proptest を不要にしない。local helper の証明と production integration の接続条件は別だからである。正規 property test は 13 件を維持し、変更クレートの回帰 test とともに実施する。

Fuzzing は現行必須 lane ではない。導入する場合も Kani/proptest の代替ではなく、広い構造入力空間に対する継続的探索として位置付ける。

## Lean 等のより強い手法を検討する範囲

Lean の導入候補は、Kani の bounded model checking では任意長・任意回数を直接カバーできない算術的性質である。

- 任意長 chunk/box scan の termination と strict progress。
- AVI timing の remainder を含む累積算術、stream ごとの独立 clock、長い packet 列に対する不変条件。
- RIFF size、frame count、sample table count など、u64 全域で overflow しないことを不変条件として証明する計算。
- WAVEFORMATEX/PCM conversion の bit-level 正しさを、round-trip sampling ではなく仕様レベルで証明する領域。

ただし導入は未決定である。検討事項は、証明対象を production Rust とどう同期するか、Rust セマンティクスのモデル化コスト、CI 実行時間、専門性、証明のメンテナンス負荷である。結論が出るまで、Lean を Required assurance として扱わない。Kani local proof で代替できる部分は Lean 導入を待たずに進める。

## 形式的検証を適用しない範囲

次には形式的検証を適用しない、または直接の対象にしない。

- ユーザー向け diagnostic メッセージ本文、その localization、formatting 出力。
- OS ファイル I/O、file system failure、外部プロセス挙動。
- allocator 自身の正しさ、`std`/`core` の internal invariant。
- 未対応 codec の内部 semantics、製品契約外の codec 仕様。
- GUI/CLI 全体の振る舞い。これらは別の検証層で扱う。
- public API 全体 no-panic の symbolic proof。個別 local proof を転用しない。

これらは「保証不要」ではなく、手法が不適切または責務が別層という意味である。製品挙動は unit/integration/proptest で観測し、panic の可能性や未証明領域は ledger に残す。

## Stub、assume、Bound の監査規則

- `assume` は入力領域を狭める制約であり、証明範囲の一部として明記する。製品入力全域に対する証明へ読み替えない。
- `#[kani::unwind]` は証明対象の反復回数上限である。超過列は未証明として残す。
- stub は置換対象の semantics を変更し得るため、等価性または無関係性を proof oracle の独立性で再評価する。
- `fmt_format_empty` は、Result と error reason 文字列を検査しない 2 件の WAV legacy no-panic harness でのみ採用済みである。これは条件付き保証であり、formatting 自体、message 長依存 allocation、公開 API 全体 no-panic の証明ではない。allocation failure stub と MP4 writer fmt stub は却下された経緯を維持する。
- Mutation/truncation split では、「mutated」「truncated」「両方」の入力領域を混ぜない。proptest 側の property 名と Kani harness の入力領域を ledger で一致させる。

## 実行・集計の分離

- `spike_` harness は実験用であり、canonical proof と別に集計する。runner registryへ混入しない。
- Required local proof、legacy whole-call proof、spike proof、proptest を別系列として記録する。
- Kani 環境がない環境では `Unsupported`/`NotRun` を返す。runner exit 0 や CI job green を proof verdict と混同しない。
- 実行時間と peak memory は verdict とは別に `/usr/bin/time -v` 等で記録する。タイムアウト proof の wall clock は proof の実効コストではなく timeout 到達である。

## Property 対応表

| ID / Property | 主な手法 | 現行 verdict | 未証明・残留リスク |
| --- | --- | --- | --- |
| WAV-SIG-001 RIFF/WAVE signature | Kani local + proptest | Pass | signature は decode capability を意味しない |
| WAV-RIFF-002 RIFF size/input length | Kani local | NotRun | proof 未実装 |
| WAV-CHUNK-003 chunk boundary | Kani local + proptest mutation | Pass | 大きな chunk 数、未知 chunk semantic |
| WAV-SCAN-004 scan progress | Kani local | Pass | 4 valid chunk 以上、無制限 scan termination |
| WAV-FMT-005 fmt/data presence/order | proptest primary + selective Kani | NotRun | selective formal proof 未実装 |
| WAV-PCM-LAYOUT-006 format/layout validation | Kani local + proptest mutation | NotRun | proof 未実装 |
| WAV-PCM-CONV-007 PCM conversion | Kani local + proptest round-trip | NotRun | symbolic conversion proof 未実装 |
| WAV-RANGE-008 range validation/byte offset/slice exactness | Kani local + proptest integration | Pass | I/O、allocation、PCM、diagnostic |
| WAV-WRITER-009 writer parameter/header/backpatch | proptest primary + selective Kani | NotRun | selective arithmetic proof 未実装 |
| WAV-ROUNDTRIP-010 writer/reader round-trip | proptest primary | Property Tested Pass / formal Deferred | symbolic end-to-end Kani は Deferred |
| WAV-CORRUPT-011 corruption/truncation no-panic | proptest primary | Property Tested Pass / formal Deferred | whole-call Kani は timeout evidence のまま |
| MP4-SIG-001 MP4 signature | Kani local + proptest | Pass | free/mdat 先頭 MP4、decode capability |
| MP4-BOX-002 box header boundary | Kani local | Pass | box semantic、sample table、PCM |
| MP4-SCAN-003 box scan progress | Kani local | Pass | 4 valid box 以上、深い nesting |
| MP4-STRUCT-004 required boxes/facts | proptest primary | Property Tested Pass / formal Deferred | whole-structure formal proof 未実装 |
| MP4-TABLE-005 sample table arithmetic | Kani local + proptest roundtrip | NotRun | proof 未実装 |
| MP4-PCM-DECL-006 PCM codec declaration/capability | Kani local + proptest/concrete matrix | Pass | parser integration、sample table、PCM extraction/conversion、payload semantics、diagnostics、I/O、allocation |
| MP4-PCM-EXTRACT-007 PCM extraction arithmetic | Kani local + proptest | NotRun | proof 未実装 |
| MP4-PARTIAL-008 partial ingest | proptest primary | Property Tested Pass / formal Deferred | formal proof は適用しない |
| MP4-ROUNDTRIP-009 round-trip | proptest primary | Property Tested Pass / formal Deferred | symbolic end-to-end Kani は Deferred |
| MP4-CORRUPT-010 corruption no-panic | proptest primary | Property Tested Pass / formal Deferred | whole-call Kani は Deferred |
| AVI-CHUNK-001 RIFF chunk/LIST boundary | Kani local | Pass | nested LIST/OpenDML/idx1 semantic |
| AVI-STREAM-002 strh/strf declaration | proptest primary + selective Kani | NotRun | 完全な declaration correspondence、任意長 walker |
| AVI-MOVI-003 movi packet id/kind/body | Kani local | NotRun | body boundary、4件以上、Vec accumulation、無制限 iteration |
| AVI-TIME-004 movi_order timing | Kani local + proptest integration | NotRun | remainder/accumulation/stream separation |
| AVI-PCM-005 WAVEFORMATEX PCM decode | Kani local + proptest | NotRun | PCM conversion semantics、任意長 copy |
| AVI-VIDEO-006 video metadata/features | proptest primary | Property Tested Pass / selective formal partial | feature integration は proptest primary |
| AVI-PARTIAL-007 partial ingest | proptest primary | Property Tested Pass / formal Deferred | formal proof は適用しない |
| AVI-ROUNDTRIP-008 round-trip | proptest primary | Property Tested Pass / formal Deferred | symbolic end-to-end Kani は Deferred |
| AVI-CORRUPT-009 corruption no-panic | proptest primary | Property Tested Pass / formal Deferred | whole-call Kani は Deferred |

## Open decision

本 sliceでは次を決定しない。

- Kani runner、bounded/lazy/分割など tactical execution strategy の最終形。
- Lean 等の強い手法を導入するか、導入対象を何に限定するか。
- 4 要素以上の scan/iteration、nested walker、任意長 copy/dynamic Vec をどこまで local model で拡張するか。
- legacy whole-call proof を runner registry から物理削除する時期。代替 assurance と Required 判定が揃うまで退役判断は個別に行う。
- 既存 `docs/specs/formal-verification.md` との統合方法。変更中の文書との競合を避け、後続 sliceで統合する。

## RF 還元判定

Ruviform へ還元できる候補はあるが、本 slice では Issue 起票や既存 Issue 更新を行わない。候補は、固定長 Write+Seek sink / KaniSink、RIFF chunk boundary walker と WAVEFORMATEX/PCM conversion の検証済み共通契約、diagnostic error payload 分離の設計知見である。既存の ruviform 側関連 Issue が重複領域を担当するため、新規起票が必要と判断した場合は別途 Coordinator と確認する。

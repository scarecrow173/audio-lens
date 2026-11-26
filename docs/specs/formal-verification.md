---
id: "SPEC-034FormalVerification256"
type: "spec"
status: "proposed"
title: "formal verification (D 層) 導入: Property 起点の手法選択と WAV codec / MP4・AVI container 標的 (Issue #256)"
created: "2026-11-20"
updated: "2026-11-26"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/audio-lens/issues/256"
  changes:
    added:
      - type: "file"
        path: "crates/al-media/tests/formal_wav.rs"
      - type: "file"
        path: "crates/al-media/tests/formal_containers.rs"
      - type: "file"
        path: "tools/run-formal-kani.mjs"
      - type: "doc"
        path: "docs/specs/formal-verification.md"
      - type: "dir"
        path: "evidence/al256/"
    modified:
      - type: "file"
        path: "crates/al-media/Cargo.toml"
      - type: "file"
        path: "mise.toml"
      - type: "file"
        path: ".agents/skills/verify-audio-lens/SKILL.md"
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

# formal verification (D 層) 導入: Property 起点の手法選択と WAV codec / MP4・AVI container 標的

Issue #256。検証環境の理想形(`.devin/verify-env-survey/synthesis.md`
§1)が置く 7 層のうち、D 層は「高コストバグ領域への形式的検証」
を担う。本文書は D 層の設計(器の選択規則)と、第 1 標的
WAV codec・第 2 標的 MP4/AVI container parse の現行検証構成・bounds・
assumptions・coverage 記録をまとめた正本である。実行記録
(コマンド・exit code・verdict 表・変異 fail 証拠)は
`evidence/al256/` の台帳が持つ。

なお、POLICIES.md §6 が採用する Property 起点の方針では、proptest、
Kani、TLA+、Lean は固定された昇格路ではなく、Property・保証種別・
実行予算に応じて選ぶ補完的な手法である。Issue #274 における現行
inventory と target verification structure は
`docs/designs/al274-formal-verification-boundary.md` と
`evidence/al274-boundary/` が保持し、本文書の旧「昇格路」表現は
未移行の記述として読み替える。文書責務の全面再構成は audio-lens#278
に残す。

## 器の選択規則

採用する規則は pstack `create-formal-verification-skill` の
「問いが器を選ぶ」である。テストは書いたケースだけを調べ、
形式的検証は宣言した範囲の全ケースを調べる。器はその範囲の形で
決まる。

| 問いの種類 | 器 | 本リポジトリでの使いどころ |
| --- | --- | --- |
| 実コードを生成入力で広く打つ(サンプリング+shrink) | proptest | parser/codec の round-trip・不変量のランダム検査。`cargo test` に載り追加 toolchain 不要 |
| 実コードが任意入力で性質を保つか(bounded 網羅) | Kani | parser/codec の任意入力安全・round-trip・算術の不変量。`cargo kani` でコードに直接かかる |
| 手順・イベントの悪い順序がありうるか(設計モデル) | TLA+ | job resume/cancel・staged publish の順序検査 |
| 全サイズで成立する証明が要る性質 | Lean 4 | 解析数学の不変量・baseline compare の決定性 — 文書化された promise に限る |
| 並行 interleaving(実コード・共有状態) | loom | RT 境界の lock-free 構造があれば。現状候補少なめ |

proptest、Kani、TLA+、Lean の間に固定の昇格順は設けない。proptest
は toolchain 追加なしに常時 `cargo test` で回る統計的 property
検査として残し、Kani は宣言した bound 内の網羅が必要で実行予算に
収まる Property に使う。状態機械やイベント順序は TLA+、全サイズの
数学的性質は Lean を検討する。各 Property の必要保証・粒度・境界・
実行予算は POLICIES.md §6 と Issue #274 の ledger に従って決める。

## 第 1 標的: WAV codec (`crates/al-media/src/wav.rs`)

第 1 標的には「破損がユーザーに届き、テストが届きにくい」契約を
持つ最強の対象を選ぶ。WAV codec は decode(`decode_wav`・
`wav_frame_count`・`read_wav_range`)と write(`WavWriter`)の双方
があり、同 encoding の再 encode が lossless という明示の往復契約
(wav.rs `WavEncoding` doc)まで揃う。layout 走査 `wav_layout` は
checked_add・範囲照合で任意入力への耐性を意図して書かれており、
その意図を検証対象として固定できる。`al-media` に `unsafe` は
存在しない。

### 検証する property(coverage 記録)

`crates/al-media/tests/formal_wav.rs` 一ファイルに proptest suite
と `#[cfg(kani)]` bounded proof harness を同居させる。

| property | 検証する契約 |
| --- | --- |
| `decode_wav_never_panics_on_arbitrary_bytes` | 任意 `&[u8]`(0..=4096B)で panic しない。返り値は `Ok` または `Err(MediaError)` のみ |
| `frame_count_and_range_reads_hold_the_contract` | `wav_frame_count`/`read_wav_range` も同じ no-panic 契約を持つ。layout 失敗なら全 read が失敗し、countable かつ decodable なら全 decode と frame 数・sample slice が一致する。countable だが decodable でない入力では全 read が失敗する |
| `writer_output_round_trips_losslessly` | 5 encoding × channels 1..=8 × 任意 rate × frames について、`WavWriter`→`decode_wav` が metadata(channels/rate/bits/sample_format/codec)と samples を正確に往復する(1..=3 分割 write 含む)。`wav_frame_count` と `read_wav_range` が書き出し layout と一致する |
| `writer_new_rejects_exactly_the_documented_invalid_params` | `WavWriter::new` が channels==0・rate==0・block_align/byte_rate overflow のときに限り失敗する |
| `write_samples_rejects_exactly_unaligned_lengths` | `write_samples` が frame 境界でない長さに限り失敗する |
| `mutated_or_truncated_headers_never_panic` | 有効 WAV の header field 上書き・任意位置 truncate で 3 read 入口が panic しない |
| `kani_proofs::decode_wav_never_panics_signed` | RIFF/WAVE signature を持つ 48B 入力で `decode_wav` が panic しない(al#274 で entry point × signature class に分割) |
| `kani_proofs::wav_frame_count_never_panics_signed` | 同入力で `wav_frame_count` が panic しない |
| `kani_proofs::read_wav_range_never_panics_signed` | 同入力 + u8 由来の任意 start/end で `read_wav_range` が panic しない |
| `kani_proofs::wav_reads_never_panics_unsigned` | signature を持たない入力で 3 read 入口が panic せず早期 reject する — signed 3 proof と合わせて全入力領域を網羅 |
| `kani_proofs::writer_round_trip_pcm_s16` / `_pcm_s24` / `_pcm_s32` / `_float32` / `_float64` | symbolic params の writer 出力が正確に往復する — encoding 別に 1 proof ずつ(al#274 で 5 択 symbolic encoding を分割) |
| `wav_range_plan_valid_ranges_exact` | constructor-admitted WAV data layout 全域と valid public u64 frame index について、受理・frame count・byte offset/byte length/exclusive byte end・data extent containment・算術安全性を検証する。input は start/frame_count/tail constructive decomposition で exact に表す |
| `wav_range_plan_invalid_ranges_error` | invalid exact complement(`start>=end` または `end>total`)を full u64 frame index で全件拒否し、compact error payload の total frames が一致することを検証する。valid proof と分岐空間が exact complement である |
| `wav_chunk_boundary_step_exact` | full u64 の chunk offset/file length と full u32 の chunk size について、boundary helper が全入力を valid/invalid/terminal の exact complement へ写すこと、`body_start`/`body_end`/pad/next offset/file length containment/少なくとも 8 byte の進行/残余量減少/算術安全性を検証する |
| `wav_chunk_scan_progress_bounded_exact` | 初期 offset 12・`file_len>=12` の scan model で、terminal exact end と trailing bytes、valid step の単調増加と残余量減少、invalid 終端を検証する。small scan bound は高々 3 valid chunk で、full u64 file length と full u32 size 列を保ち、4 valid chunk 以上は未証明領域に残す |

proptest 側が覆うのは統計的サンプリング(既定 case 数)の範囲で
あり、Kani 側は宣言した bound 内の網羅を検証する。両者は同じ
property 言語を別の保証種別で検査する補完関係であり、proptest の
Pass を Kani の bounded exhaustive proof と同値に扱わない。

### bounds・assumptions・model/product の区別

- **入力 bound**: proptest の任意 byte 列は 0..=4096B。Kani の
  decode 入力は 48B 固定(最小 RIFF/WAVE header 44B + data 数 B を
  覆い、全 `wav_layout` 分岐と数回の sample loop を通せる最小の
  実用 bound)。いずれも「任意入力の安全性」を bound 内で述べる
  もので、より大きな入力への主張は持たない。
- **writer パラメータ bound**: Kani 側は channels 1..=4・rate
  1..=192_000・1 frame と、writer の有効空間内に制約する
  (symbolic `f64` 変換が支配的な検証コストであり、1 frame で
  write → decode → slice の全経路を通せる)。proptest 側は
  channels 1..=8・rate <= u32::MAX/64・frames 0..=72 の
  ランダム範囲。
- **unwind bound**: al#274 で proof を単一責務へ分割し、unwind
  は各 proof が実際に辿る最悪 loop trip 数に合わせた。
  no-panic 系は `decode_wav`/`read_wav_range`(signed)が
  `#[kani::unwind(40)]`(layout walk + data body の sample loop)、
  `wav_frame_count`(signed)と unsigned 系が
  `#[kani::unwind(16)]`(layout walk / signature reject のみ)。
  writer round-trip・header 系は `#[kani::unwind(32)]`。
  初版のより大きな bound([u8;128]+unwind160 / 2 frames+unwind64)
  および al#256 の 3 入口結合 proof(unwind 56)はこの環境で
  45min cap を超過した — trade-off の実測は
  `evidence/al256/`・`evidence/al274/` に記録する。unwind
  assertions は有効のままなので、bound 不足は黙って網羅を
  欠くのではなく FAILED として表れる。
- **sample の表現可能性**: 整数 encoding は範囲内の整値 f64、
  Float32 は f32 表現値、Float64 は非 NaN 値だけを往復対象にする
  (doc 契約は clamp+round / cast を約束し、任意値の厳密往復は
  約束しない)。現行 generator は NaN のみを除外し、Infinity を
  除外しない。NaN は `!=` が成立しないため除外する。
- **model/product 区別**: 検証が写すのは wav.rs の文書化契約で
  あり、product 意味論が正本である。fail が出たとき、
  counterexample が文書化契約に反すれば product defect、検証側が
  契約を強く読みすぎていれば model error と見分ける。具体例と
  して、`wav_frame_count` は layout 走査だけで計数するため
  「countable だが decodable でない」ファイルが存在しうる —
  これは不具合ではなく契約の形であり、suite はその区別を
  property として固定している。
- **WAV-RANGE-008 exact proof**: `crates/al-media/src/wav.rs` の
  production pure helper `WavDataLayout::range_plan` を symbolic
  scalar だけで検査する。input domain は decode 可能 encoding に
  絞らない constructor-admitted layout 全体である。valid proof は
  `end = start + frame_count`、`total = end + tail`、
  `data_len = start*block_align + frame_count*block_align + tail*block_align`
  という exact constructive decomposition を使う。I/O、allocation、
  PCM decode、diagnostic formatting、公開 API 全体 no-panic は
  この local proof の保証外である。writer fixture を使う旧
  `header_reads_valid_ranges_slice_exact` / `header_reads_invalid_ranges_error`
  は exact replacement が Pass した後に retire した。
- **WAV-CHUNK-003/WAV-SCAN-004 exact proof**: `crates/al-media/src/wav.rs`
  の production pure helper `wav_scan_position` / `wav_chunk_step` を
  symbolic scalar だけで検査する。boundary proof は input narrowing
  assume を置かず、full u64 offset/file length と full u32 size の
  全数値領域を対象にする。`offset > file_len` は production 到達外の
  total extension であり、terminal trailing 側へ分類する。scan proof
  の `file_len>=12` は RIFF/WAVE header 確認後に scan へ入る exact
  constructor precondition、高々 3 valid chunk は small scan model の
  明示 bound である。bounded model の成功を unbounded scan termination
  と読み替えず、one-step measure decrease と finite unrolled model の
  役割を分ける。I/O、allocation、PCM decode、diagnostic formatting、
  fmt/data semantic、duplicate 判定、公開 API 全体 no-panic は保証外
  である。詳細な境界と coverage は
  `evidence/al274-chunk-scan-proof/boundary-analysis.md` を正本とする。
- **coverage の残り**: `WavEncoding` は PcmU8/PcmS8 を emit でき
  ない(decode-only)ため、writer 起点の round-trip は 5 encoding
  に限る。PcmU8 の decode は mutated/structured 入力側の
  no-panic 契約と、`read_wav_range` 経路の一部として間接に覆う。

## 第 2 標的: MP4/AVI container parse (`mp4.rs`/`avi.rs`/`format.rs`)

第 2 標的は第 1 標的と同じ hardened-layout 形を持つ
`decode_mp4`/`decode_avi`/`detect_format`(いずれも純粋 `&[u8]`
入口)である。demux 契約は codec 契約より広い: container 破損は
`Malformed`、compressed/missing audio はエラーではなく integrity
observation(partial ingest)、declared facts(brands・track 宣言・
skipped_boxes・stts/ctts timing)と PCM 抽出が「最初に復号できる
`soun`」の規則で決まる。`al-testkit` の `write_mp4`/`write_avi`
が writer 側の正本となり、spec → bytes → decode → facts の
round-trip を生成検査できる。

`crates/al-media/tests/formal_containers.rs` 一ファイルに proptest
suite と `#[cfg(kani)]` bounded proof harness を同居させる。

### 検証する property(coverage 記録)

| property | 検証する契約 |
| --- | --- |
| `container_decoders_never_panics_on_arbitrary_bytes` | 任意 `&[u8]`(0..=4096B)で `decode_mp4`/`decode_avi`/`detect_format` が panic しない。返り値は `Ok`/`Err(MediaError)`/`Option` のみ |
| `mp4_writer_output_round_trips_through_decode` | `write_mp4` 出力を必ず受理し、ftyp/mvhd facts・track 宣言(handler/codec/timescale/duration/language/sample_count/sync_samples/audio dims/video dims)・`skipped_boxes`・stts DTS と ctts PTS の timing・`packet_lengths`・`pcm_tracks`・先頭 decodable `soun` の PCM 復号・残り `soun` の `decode_unavailable` observation が spec と一致する |
| `avi_writer_output_round_trips_through_decode` | `write_avi` 出力を必ず受理し、先頭 `vids` の metadata(rawvideo codec・寸法・bpp・frames/fps/duration)+features、先頭 `auds` の PCM 復号または `audio_unavailable`、両 stream の `movi_order` packet timing(`dwStart*dwScale` 起点・`dwSampleSize` remainder 則・独立 clock)が spec と一致する |
| `compressed_audio_is_an_observation_not_an_error` | compressed のみの `soun`(`mp4a`/`ac-3`/`alac`/`ec-3`)を含む MP4 が `Err` にならず、`audio` None + `soun` 個数分の capability observation になる(partial ingest) |
| `missing_or_undecodable_auds_is_audio_unavailable` | `auds` なし・非 PCM `auds`・`wb` を持たない `auds` の AVI が `Err` にならず、`audio` None + `audio_unavailable`(capability)+video/timing は健在 |
| `detect_format_follows_signature_rules` | `RIFF`+`WAVE`/`AVI `・offset 4 `ftyp`→`Mp4`・それ以外/短い入力→`None` の署名規則が doc 契約 model と一致(切り詰め view 含む) |
| `mutated_or_truncated_containers_never_panic` | writer 生成物への byte 上書き+truncate が panic しない |
| `kani_proofs::mp4_never_panics_ftyp_signed` / `avi_never_panics_riff_signed` | `ftyp`/`RIFF`+`AVI ` signature を持つ 64B 入力で `decode_mp4`/`decode_avi` が panic しない(bounded 網羅・al#274 で entry point × signature class に分割。`detect_format` は先頭 <=12B のみ読む近似無料の入口として各 proof に同乗) |
| `kani_proofs::containers_never_panics_unsigned` | いずれの signature も持たない入力で 3 入口が panic せず早期 reject する — signed 2 proof と合わせて全入力領域を網羅 |
| `kani_proofs::detect_format_signature_rules` | len<8/8/12 の境界長で署名規則を網羅(RIFF 優先の順序を含む) |
| `kani_proofs::mp4_writer_output_decodes` | symbolic `soun` パラメータ(channels 1..=2・rate 1..=192000・4 frame s16)の write_mp4 出力が復号+PCM 往復する |
| `kani_proofs::avi_writer_output_decodes` | concrete video(4x2・2 frame)+symbolic PCM `auds` の write_avi 出力が復号+PCM 往復+両 stream の timing 一致 |
| `kani_proofs::mutated_mp4_never_panics` / `mutated_avi_never_panics` | writer 生成物への 1 byte 上書き(任意位置・任意値)で panic しない |
| `kani_proofs::truncated_mp4_never_panics` / `truncated_avi_never_panics` | writer 生成物の任意長 truncate で panic しない — al#274 で mutate と分離。mutate+truncate の結合入力は proptest `mutated_or_truncated_containers_never_panic`(最大 16 箇所編集+truncate)が担保する |
| `mp4_box_header_step_exact` | full u64 の box offset/region limit、full u32 size field、full u64 largesize field について、production pure helper が size32/size64(size==1)/size0、truncated header、size/largesize minimum、checked arithmetic、region limit を exact complement へ写すこと。valid box の body_start/body_end/end containment、少なくとも header 分の進行、残余量減少を検証する |
| `avi_chunk_step_exact` | full u64 の region-relative offset/region length と full u32 chunk size について、production pure helper が empty/exact end、1..7 byte trailing、valid chunk、invalid chunk を exact complement へ写すこと。body_start/body_end/pad-inclusive next offset、region limit、少なくとも 8 byte の進行、残余量減少を検証する |

### bounds・assumptions・model/product の区別

- **入力 bound**: proptest の任意 byte 列は 0..=4096B。Kani の
  decode 入力は 64B 固定 — ISO-BMFF は `stsd` 到達まで約 60B の
  入れ子を要するため、top-level box 走査と早期 header 検証を覆う
  bound(全 size の主張ではない)。
- **生成範囲**: MP4 は track 1..=3・`soun`/`vide` 混在。
  `soun` entry は decodable 系(sowt/twos 16bit、sowt/twos 8bit、
  `raw ` unsigned、lpcm v2 signed+packed LE/BE、ipcm+pcmC LE/BE)
  と unavailable 系(compressed fourcc・ipcm pcmC 無・lpcm
  unpacked・12bit sowt・mid-frame・0ch/0rate)を網羅する。mdhd
  timescale は 1..=192000(0 は Malformed なので対象外)。
  AVI は vids BGR24 width {4,8,16,32,64}(word-align)×height
  1..=16・fps 1..=60・frames 0..=4、auds は PCM s16 1..=4ch・
  非 PCM(tag 0x55)・absent を網羅、`dwStart` 0..=8、mux は
  hdrl 順序差と packet interleave の双方。
- **16.16 fixed rate の wrap**: v0/v1 `soun` entry の sample rate
  は 16.16 fixed-point で書かれ、rate > 65535 は下位 16bit に
  折り返す(65536/131072 → declared rate 0 → malformed
  observation)。model は emitted field でなく decoder が見る
  declared rate を写す — container の表現限界であり不具合では
  ない。
- **unwind bound**: al#274 で decode/format 系 8–48、writer
  round-trip 系 48–64、mutation/truncation 系 64–80。WAV 同様、
  この環境の 45min cap 内に収めるための最小実用 bound で、
  unwind assertions 有効のため不足は FAILED として表れる。
- **model/product 区別**: `skipped_boxes`(minf の非 stbl 子と
  top-level extras が path 記録される)、`pcm_tracks` 所属(宣言
  capability であり復号成功とは独立)、secondary `soun` の
  capability observation、上記 rate wrap は doc 契約として model
  が写す。counterexample が文書化契約に反すれば product defect、
  model が強く読みすぎていれば model error と見分ける方針は
  第 1 標的と同じ。
- **MP4-BOX-002/AVI-CHUNK-001 exact proof**: `crates/al-media/src/mp4.rs`
  の production pure helper `mp4_box_boundary_step` と
  `crates/al-media/src/avi.rs` の production pure helper
  `avi_chunk_step` を symbolic scalar だけで検査する。input narrowing
  assume は置かず、MP4 は full u64 offset/limit・full u32 size32・
  full u64 largesize、AVI は full u64 region-relative offset/region
  length・full u32 size を対象にする。`at > limit` と
  `offset > region_len` は production 到達外の total extension であり、
  それぞれ truncated/trailing 側へ分類する。production `box_header`
  と `Chunks::next` が同じ helper を呼び、tag/id 解釈と error 文言
  生成だけを shell に残す。I/O、allocation、`Vec` decode、PCM/video
  decode、diagnostic formatting、MP4 semantic/sample table、AVI
  LIST/movi/timing、公開 API 全体 no-panic は保証外である。one-step
  measure decrease を無制限 scan termination の機械証明と読み替えない。
  詳細な境界と coverage は
  `evidence/al274-mp4-avi-proof/boundary-analysis.md` を正本とする。
- **coverage の残り**: sample table は単一 chunk の uniform 形のみ
  (multi-chunk `stsc`・複数 `stts` run・`elst`/`edts`・配列形
  `stsz` は未生成)。`fpcm`/`fl32`/`fl64`/`in24`/`in32` の PCM
  復号は未生成(layout 規則は同型)。AVI は先頭 `vids`/`auds`
  各 1 本(`dc`/`pc` kind・`idx1`・非 DIB 圧縮・`dwLength` ずれは
  未生成)。free/mdat 先頭 MP4 の `detect_format` 非認識は
  signature-rule property の分岐に依存する。

## 手法適合候補: 第 2 標的以降(固定昇格路ではない)

| 器 | 候補標的 | 根拠 |
| --- | --- | --- |
| Kani | MP4 `decode_mp4`(`mp4.rs:1187`、純粋 `&[u8]` 入口)・AVI `decode_avi`(`avi.rs:370`、`VideoFeatureConfig` 併用)の任意入力安全+round-trip | **第 2 標的として proptest+Kani harness を導入済み**(本書 §第 2 標的)。WAV と同型の hardened layout を持ち、第 1 標的の harness 形がそのまま移植できる |
| Kani | `al-job` job 状態遷移関数・tolerance compare 算術 | 単位の小さい決定的関数で bounded 化しやすい |
| TLA+ | `al-job` `runner.rs` の resume/cancel、`fsutil` staged publish の順序 | 順序・設計レベルの問いであり、設計モデルとして写す。pstack `statechart` skill で結果を解釈する |
| Lean 4 | `al-locate`/`al-analysis` の解析数学不変量(spectral/feature 恒等式)・baseline compare の決定性 | 全サイズで成立する性質で、文書化された promise に限る |

synthesis.md §4.3 の Kani 標的表には `RevisionedStore` 不変量
(revision 単調・CAS 意味) が挙がるが、実体は `ui/src/view-state.ts`
の TypeScript 実装であり Rust Kani の標的に載らない。Rust D 層
では対象外として扱い、検証が要るなら TypeScript 側の property
test(fast-check 等)で別途扱う — 同じ property 名を勝手に
Rust 標的へ読み替えないのが、model/product 区別の約束である。

## Kani 実行環境と provision runbook

`cargo kani` は Windows native では動かず、Linux/macOS が対象。
本 lane は `tools/run-formal-kani.mjs` が実行環境を検出する形に
なっている(`mise run test-formal-kani` が入口)。

1. native(Linux/macOS)で `cargo kani` が使えればそのまま実行。
2. Windows では `wsl.exe -l -q` の distro を列挙し、WSL 内の
   `cargo kani --version` が通る distro を使う。`/mnt` 経由の
   ビルド I/O を避けるため、cargo が必要とする source tree
   (Cargo.toml/Cargo.lock/.cargo/crates/dictionaries —
   al-core/build.rs の lockfile 走査と al-media lib test の
   crate 外 include_str! が要る最小集合)を
   `~/.cache/al-formal-kani/src` へ mirror してから実行する。
   cargo の target-dir 設定・env 変数は変更せず、build 出力は
   mirror 内の既定 `./target` に収まり、Windows 側 repo の
   `target/` には異 OS の artifact を書き込まない。mirror 更新は
   `target/` だけ保持して source を再展開するため、dep build
   cache は再実行間で持続する。
3. 環境不在・未 provision では verdict `not_run`/`unsupported` +
   理由 + provision 手順を出力して exit 0。環境不在は lane
   失敗ではない(POLICIES §3)。実走して verification 結果が
   出れば `pass`/`fail` で exit 0/1。kani-driver は harness ごとの
   結果を process 終了まで flush しないため、lane は各
   `kani_proofs` proof を `cargo kani -p al-media --tests
   --harness <name>` で逐次実行し、完了した proof の verdict を
   個別に保持する(`#[test]` proptest は対象外)。
   `AL_KANI_LOG_DIR` を指定すると、各 harness の raw 出力と最終
   JSON verdict がその dir に残る(証拠記録用の env-gated 機能)。
   `AL_KANI_HARNESSES="a,b,c"` で登録名の subset だけを実行できる
   (al#274 で追加した env-gated 機能 — 未指定は全件で既定挙動
   不変、未登録名は fail-closed で not_run)。

provision 手順(WSL2 Ubuntu 内):

```bash
cargo install --locked kani-verifier
cargo kani setup    # CBMC toolchain を ~/.kani に展開する
```

## verdict の実測(2026-11-24 residual run)

WSL2 `Ubuntu`(kani-verifier 0.68.0 + CBMC 6.11.0、envelope
RAM 31GiB + swap 8GiB・16 vCPU、無競合)で sync commit `342aea7`
に対して本 lane を実走した(各 harness 45min budget)。verdict
は `not_run` — **1/9 verified**、fail-closed exit 0。

- `detect_format_signature_rules`(unwind 8)のみ
  `VERIFICATION: SUCCESSFUL`(169 checks、0.63s)。
- `decode_never_panics_on_bounded_input` は envelope-bound:
  cbmc RSS が ~1GiB/min で増大し ~30.4GiB / free 0GiB で budget
  切れ。31GiB 物理 envelope が原因なので、時間を増やしても
  この box では verdict が取れない。
- 残り 7 件は solver-bound: RSS 3.3–11.4GiB で頭打ちの SAT
  grind で 45min 未収束。メモリ余力が残るため、より大きな
  budget または専用 runner では verdict が取れる見込みが残る。
- `VERIFICATION: FAILED` + counterexample は 0 件 —
  環境起因 fail でも反例 fail でもなく budget 未収束。

各 harness の raw 出力・lane verdict・環境記録は
`evidence/al256/residual/`、実行台帳は
`evidence/al256/verification.md` §residual が持つ。上記 8 件の
未収束は al#274 で支配要因分析・proof 分割を行った —
再構成後の harness 構成・verdict は `evidence/al274/` と次節の
記録を参照。

## stub モデル化の判断記録(al#274、2026-11-25)

al#274 の solver 固定床に対し、test-only の `#[kani::stub]` を代表
3 proof で計測した。product `crates/al-media/src/` は変更せず、stub
model は `crates/al-media/tests/kani_stub_models/mod.rs` に閉じる。
Kani 0.68.0 ではこの attribute が unstable なため、lane runner は
`cargo kani` に常時 `-Z stubbing` を渡す。これは属性機構を有効化
するだけで、stub attribute を持たない harness の検証意味論は変えない。

### 採用した stub

- `std::fmt::format -> kani_stub_models::fmt_format_empty` を
  `wav_reads_never_panics_unsigned` と
  `read_wav_range_never_panics_signed` に適用した。replacement は
  空文字列を返すだけで、formatting loop・`PadAdapter`・memchr scan を
  proof から外す。
- 採用根拠: 両 proof は decode/read の `Result` を捨て、panic しない
  ことだけを検証し、`MediaError::reason` の人間向け文字列・error
  message の内容・formatting 自体の panic/alloc 挙動を property に
  使わない。したがって診断文字列を空にしても、これらの proof が
  検証する契約は変わらない。
- 検証外になるもの: error message の内容、formatting 自体の
  panic/alloc 挙動、message 長に依存する allocation 挙動。これらは
  product の parse/write/memory-safety 契約ではなく診断表現の挙動と
  して分離する。

### 採用しなかった stub

- `std::alloc::handle_alloc_error` を infeasible にする replacement は
  semantic 上 `--no-malloc-may-fail` と整合するが、ablation では
  failure endpoint の `rt_error` event を消せただけで、代表 proof の
  wall-clock・peak RSS・product 側到達を有意に改善しなかった。
  Layout 計算・capacity overflow・`TryReserveError` 分岐・RawVec grow・
  pointer validity 計装は残るため採用しない。
- `mp4_writer_output_decodes` への fmt stub は contract-equivalent
  ではない。decode 結果の `a.codec == "pcm_s16le"` は `format!` で
  作られる文字列を直接 assert するため、空文字列化は proof 自体を
  壊す。
- `std::fmt::write` fallback は測定・採用しない。compile smoke で
  `std::fmt::format` 指定が `format!` lowering 先へ実際に解決し、
  verbose 上 target 経路が消えることを確認できたためである。
- pointer/slice validity instrumentation(`to_isize`・
  `same_allocation`)、`memcmp`、tag/codec 文字列、Vec/RawVec private
  internals は stub しない。これらは memory-safety や parse/write の
  検証意味論そのものであり、no-op 化は等価モデル化ではなく検証弱化に
  なる。

### ablation の実測要約

600s cap の baseline/fmt-only/alloc-only/combined matrix(補助証拠)
では、fmt stub は WAV 2 proof で `memchr`/`PadAdapter` event を 0 にし、
no-stub control が product/testkit function event 0 件で stdlib 計装内に
留まったのに対し、`wav_layout` iteration 16–19 まで到達した。一方で
verdict は 600s では得られず、wall-clock は cap で同じ、unsigned の
peak RSS は増えた。range では peak RSS が 8471→6744MiB に下がった。
したがって fmt stub は「診断 formatting の人工的な床」を除去して次の
支配因へ進めるが、それだけで代表 proof の verdict を取れるほど
solver 時間を下げるものではない。

採用後の canonical 代表 subset(1500s/proof、2026-11-25)でもこの解釈が
確認された。fmt stub を採用した WAV 2 proof では `Stub:` 解決と
`memchr`/`PadAdapter` 0 件が残り、range は `wav_layout` iteration 29、
unsigned は `decode_samples` まで到達した。しかし両者とも timeout で、
peak RSS は range 10324MiB・unsigned 13762MiB だった。fmt stub を採用
しない `mp4_writer_output_decodes` も timeout(peak RSS 7150MiB、
`write_mp4` 到達)であり、3 件とも `VERIFICATION: SUCCESSFUL` / `FAILED`
は得られなかった。したがって canonical でも「床は浅い診断経路から
product 側へ移ったが、verdict が取れるほど solver 時間は下がらない」
という結論を維持する。

残る支配因は pointer/slice validity instrumentation(`to_isize`・
`same_allocation`)、slice bounds panic machinery、parser の分岐、
RawVec/Layout/capacity 計算である。これらは検証対象の安全性・parse
意味論に属し、等価性を保ったまま `#[kani::stub]` で除去できる候補
ではない。詳細な計測表・raw log・canonical verdict は
`evidence/al274/stub-spike/` を参照する。

## 検証と再実行

正規入口は mise task である。

- `mise run test-formal`: proptest lane。native で常時実行できる
  第 1・第 2 標的の 1 コマンド再実行(`--test formal_wav --test
  formal_containers`)。
- `mise run test-formal-kani`: Kani lane。env-detect の結果に
  応じて pass/fail/not_run/unsupported を JSON verdict で返す。
  al#274 mp4/avi boundary slice 以降の登録 harness は 25 件(WAV 13 +
  MP4/AVI boundary 2 + legacy containers 10)。
  `AL_KANI_HARNESS_GROUP="wav-chunk-scan"` は
  WAV-CHUNK-003/WAV-SCAN-004 の 2 proof、`"wav-range"` は
  WAV-RANGE-008 の 2 proof、`"mp4-avi-boundary"` は
  MP4-BOX-002/AVI-CHUNK-001 の 2 proof を選択する。group と subset
  指定は排他で、未指定は全件の既定挙動である。lane verdict は
  per-harness detail に加え、選択 proof 数・verified proof 数・
  total elapsed ms・max observed RSS KiB・計測可否を持つ additive
  `cost_summary` を返す。
- 変異 fail 証拠・実行環境・sync commit SHA・verdict 表は
  `evidence/al256/verification.md` と `actions.jsonl`、al#274
  の分析・分割・再実走記録は `evidence/al274/` が持つ。

## Sources and Relations

- Issue: <https://github.com/scarecrow173/audio-lens/issues/256>
- 継続 Issue: <https://github.com/scarecrow173/audio-lens/issues/274>(solver-bound 原因調査・budget 削減)
- 設計正本: `G:/repositry/audio-products/.devin/verify-env-survey/synthesis.md` §4.3
- 対象コード: `crates/al-media/src/wav.rs`・`crates/al-media/src/mp4.rs`・
  `crates/al-media/src/avi.rs`・`crates/al-media/src/format.rs`・
  `crates/al-media/tests/formal_wav.rs`・
  `crates/al-media/tests/formal_containers.rs`
- lane runner: `tools/run-formal-kani.mjs`
- 証拠台帳: `evidence/al256/`・`evidence/al274/`・
  `evidence/al274-proof-redesign/`・`evidence/al274-chunk-scan-proof/`・
  `evidence/al274-mp4-avi-proof/`
- 関連 skill: `.agents/skills/verify-audio-lens/SKILL.md` の
  Formal verification 節

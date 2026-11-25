---
id: "SPEC-034FormalVerification256"
type: "spec"
status: "proposed"
title: "formal verification (D 層) 導入: 昇格路と WAV codec / MP4・AVI container 標的 (Issue #256)"
created: "2026-11-20"
updated: "2026-11-25"
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

# formal verification (D 層) 導入: 昇格路と WAV codec / MP4・AVI container 標的

Issue #256。検証環境の理想形(`.devin/verify-env-survey/synthesis.md`
§1)が置く 7 層のうち、D 層は「高コストバグ領域への形式的検証」
を担う。本文書は D 層の設計(器の選択規則と昇格路)と、第 1 標的
WAV codec・第 2 標的 MP4/AVI container parse の検証構成・bounds・
assumptions・coverage 記録をまとめた正本である。実行記録
(コマンド・exit code・verdict 表・変異 fail 証拠)は
`evidence/al256/` の台帳が持つ。

## 器の選択規則と昇格路

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

昇格路は proptest → Kani → TLA+ → Lean とする。proptest で property
を先に言語化し、同じ property を Kani で bounded 網羅化する順が
自然であり、proptest 側は toolchain 追加なしに常時 `cargo test`
で回る最も安い網羅的手段として残る。状態機械には op 列生成の
property test を TLA+ 着手前の段として置く。

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
| `kani_proofs::header_reads_valid_ranges_slice_exact` | writer 出力上で frame count が一致し、valid range(`start<end<=total`・u8 由来)の slice が全件一致する |
| `kani_proofs::header_reads_invalid_ranges_error` | 補集合の invalid range(`start>=end` または `end>total`)が全件エラーになる — 2 proof が合計で [0,255]×[0,255] の分岐空間を網羅する |

proptest 側が覆うのは統計的サンプリング(既定 case 数)の範囲で
あり、Kani 側は宣言した bound 内の網羅を検証する。両者は同じ
property 言語の異なる強度の段である。

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
  Float32 は f32 表現値、Float64 は非 NaN の有限範囲値だけを
  往復対象にする(doc 契約は clamp+round / cast を約束し、任意
  値の厳密往復は約束しない)。NaN は `!=` が成立しないため除外
  する。
- **model/product 区別**: 検証が写すのは wav.rs の文書化契約で
  あり、product 意味論が正本である。fail が出たとき、
  counterexample が文書化契約に反すれば product defect、検証側が
  契約を強く読みすぎていれば model error と見分ける。具体例と
  して、`wav_frame_count` は layout 走査だけで計数するため
  「countable だが decodable でない」ファイルが存在しうる —
  これは不具合ではなく契約の形であり、suite はその区別を
  property として固定している。
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
- **coverage の残り**: sample table は単一 chunk の uniform 形のみ
  (multi-chunk `stsc`・複数 `stts` run・`elst`/`edts`・配列形
  `stsz` は未生成)。`fpcm`/`fl32`/`fl64`/`in24`/`in32` の PCM
  復号は未生成(layout 規則は同型)。AVI は先頭 `vids`/`auds`
  各 1 本(`dc`/`pc` kind・`idx1`・非 DIB 圧縮・`dwLength` ずれは
  未生成)。free/mdat 先頭 MP4 の `detect_format` 非認識は
  signature-rule property の分岐に依存する。

## 昇格路: 第 2 標的以降の候補

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

## 検証と再実行

正規入口は mise task である。

- `mise run test-formal`: proptest lane。native で常時実行できる
  第 1・第 2 標的の 1 コマンド再実行(`--test formal_wav --test
  formal_containers`)。
- `mise run test-formal-kani`: Kani lane。env-detect の結果に
  応じて pass/fail/not_run/unsupported を JSON verdict で返す。
  al#274 以降の登録 harness は 21 件(WAV 11 + containers 10)。
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
- 証拠台帳: `evidence/al256/`・`evidence/al274/`
- 関連 skill: `.agents/skills/verify-audio-lens/SKILL.md` の
  Formal verification 節

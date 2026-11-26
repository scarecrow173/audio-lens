# al#274 canonical proof classification

baseline: `c91b8ce20c90733fa21098451cb768a74425884d`。

## canonical proof disposition

| proof | 現 verdict | disposition | existing proof の要求状態 | 根拠 | 未証明領域 |
| --- | --- | --- | --- | --- | --- |
| `decode_wav_never_panics_signed` | NotRun | 再構成 | Deferred | whole-call no-panic を WAV-CHUNK/WAV-SCAN/WAV-PCM/WAV-RANGE と proptest に分解する | 48byte 外の入力 / 長尺入力 / allocation/formatting 全体 |
| `wav_frame_count_never_panics_signed` | NotRun | 再構成 | Deferred | layout walk を scan/chunk/frame arithmetic に分解する | 48byte 外の入力 / 長尺入力 / allocation/formatting 全体 |
| `read_wav_range_never_panics_signed` | NotRun | 再構成 | Deferred | range validation/byte offset/slice exactness を Required local proof へ移す | 48byte 外の入力 / 長尺入力 / allocation/formatting 全体 |
| `wav_reads_never_panics_unsigned` | NotRun | 廃止/統合 | Deferred | unsigned early reject を signature/boundary proof と corruption proptest に吸収する | 48byte 外の入力 / 長尺入力 / allocation/formatting 全体 |
| `writer_round_trip_pcm_s16` | NotRun | proptest primary/再構成 | Deferred | round-trip は proptest primary、PCM conversion と writer header arithmetic を local proof 化 | multi-frame / 分割 write / clamp/round が効く範囲外入力 / NaN / PcmU8/PcmS8 |
| `writer_round_trip_pcm_s24` | NotRun | proptest primary/再構成 | Deferred | 同上。24bit sign extension/packing を conversion proof に含める | multi-frame / 分割 write / clamp/round が効く範囲外入力 / NaN / PcmU8/PcmS8 |
| `writer_round_trip_pcm_s32` | NotRun | proptest primary/再構成 | Deferred | 同上。32bit conversion/clamp を含める | multi-frame / 分割 write / clamp/round が効く範囲外入力 / NaN / PcmU8/PcmS8 |
| `writer_round_trip_float32` | NotRun | proptest primary/再構成 | Deferred | 同上。f64->f32 cast・NaN/Infinity 方針を分ける | multi-frame / 分割 write / clamp/round が効く範囲外入力 / NaN / PcmU8/PcmS8 |
| `writer_round_trip_float64` | NotRun | proptest primary/再構成 | Deferred | 同上。f64 passthrough・NaN/Infinity 方針を分ける | multi-frame / 分割 write / clamp/round が効く範囲外入力 / NaN / PcmU8/PcmS8 |
| `header_reads_valid_ranges_slice_exact` | NotRun | 再構成 | Required（replacement verdict: NotRun） | valid range arithmetic/slice exactness を代表的局所 proof に再設計する | fixture 以外の layout / u64 全域 / 任意 writer 出力 |
| `header_reads_invalid_ranges_error` | NotRun | 再構成 | Required（replacement verdict: NotRun） | invalid range rejection を代表的局所 proof に再設計する | fixture 以外の layout / u64 全域 / 任意 writer 出力 |
| `mp4_never_panics_ftyp_signed` | NotRun | 再構成 | Deferred | box header/box scan/sample table boundary に分解する | 64byte 外の入力 / 深い nested box/list / 実尺 container / allocation/formatting 全体 |
| `avi_never_panics_riff_signed` | NotRun | 再構成 | Deferred | RIFF chunk/movi packet boundary と timing arithmetic に分解する | 64byte 外の入力 / 深い nested box/list / 実尺 container / allocation/formatting 全体 |
| `containers_never_panics_unsigned` | NotRun | 廃止/統合 | Deferred | unsigned early reject を signature/boundary proof と arbitrary-input proptest に吸収する | 64byte 外の入力 / 深い nested box/list / 実尺 container / allocation/formatting 全体 |
| `detect_format_signature_rules` | Pass | Kaniに残す/再構成 | Required | small pure property。len boundary matrix と RIFF/ftyp 競合を拡張して維持する | len 0..6/9..11/13以上の個別入力 / prefix-only 性質の接続証明 |
| `mp4_writer_output_decodes` | NotRun | proptest primary/再構成 | Deferred | symbolic end-to-end をやめ、serialization/PCM extraction の local proof へ分解する | multi-track / compressed audio / ctts/edit list / non-uniform sample table |
| `avi_writer_output_decodes` | NotRun | proptest primary/再構成 | Deferred | 同上。movi/timing/pcm の local proof へ分解する | non-PCM/missing auds / multi-stream/interleave / video compression variants |
| `mutated_mp4_never_panics` | NotRun | 廃止/統合 | Deferred | mutation safety を box walker proof と proptest に統合する | mutation+truncation 複合 / 複数 edit / writer-generated 以外の構造 / 実尺入力 |
| `truncated_mp4_never_panics` | NotRun | 廃止/統合 | Deferred | truncation safety を box walker proof と proptest に統合する | mutation+truncation 複合 / 複数 edit / writer-generated 以外の構造 / 実尺入力 |
| `mutated_avi_never_panics` | NotRun | 廃止/統合 | Deferred | mutation safety を RIFF/movi walker proof と proptest に統合する | mutation+truncation 複合 / 複数 edit / writer-generated 以外の構造 / 実尺入力 |
| `truncated_avi_never_panics` | NotRun | 廃止/統合 | Deferred | truncation safety を RIFF/movi walker proof と proptest に統合する | mutation+truncation 複合 / 複数 edit / writer-generated 以外の構造 / 実尺入力 |

## target verification structure

| Property | 製品/形式 | 必要保証 | 粒度 | 境界 | 手法 | 実行予算 | 要求状態 | 未証明領域 |
| --- | --- | --- | --- | --- | --- | --- | --- | --- |
| WAV-SIG-001<br>RIFF/WAVE signature の受理/拒否 | WAV | Bounded Exhaustive local + Property Tested | pure signature predicate | 先頭 12byte と入力長だけ | Kani local + proptest | proof 2分以内・4GiB未満 | Required | prefix-only 性質の接続証明 |
| WAV-RIFF-002<br>RIFF size と入力長の整合 | WAV | Bounded Exhaustive local arithmetic | header arithmetic helper | u32 size/u64 length/checked arithmetic | Kani local | proof 5分以内・4GiB未満 | Required | 実ファイル streaming reader 全経路 |
| WAV-CHUNK-003<br>chunk offset/size/padding の境界安全性 | WAV | Bounded Exhaustive local | pure chunk boundary walker | chunk header/size/body_end/pad/file_len の算術。I/O・Vec・診断を境界外 | Kani local + proptest mutation | proof 10分以内・4GiB未満 | Required | 大きな chunk 数・未知 chunk semantic |
| WAV-SCAN-004<br>chunk scan の進行性・終了性・trailing byte 判定 | WAV | Bounded Exhaustive local/model | walker state transition | offset 単調増加・終了条件・残 byte。small chunk count | Kani local | proof 10分以内・4GiB未満 | Required | 無制限 chunk 数・I/O error 系列 |
| WAV-FMT-005<br>fmt/data presence・duplicate・構造整合 | WAV | Property Tested primary + selective local | structural parser invariant | presence/duplicate/order/unknown chunk skip | proptest primary + selective Kani | 現行 lane 内、selective proof 5分以内 | Required | fmt 拡張・非 PCM 形式の完全適合 |
| WAV-PCM-LAYOUT-006<br>format tag/bits/channels/rate/block_align/byte_rate 検証 | WAV | Bounded Exhaustive local | sample_layout validation | format/bits/channel/rate/align/rate product gate | Kani local + proptest mutation | proof 5分以内・4GiB未満 | Required | 未対応 codec capability 判定全体 |
| WAV-PCM-CONV-007<br>PCM byte→f64変換と writer encode の clamp/round/cast | WAV | Bounded Exhaustive local + Property Tested | pure conversion function | u8/s16/s24/s32/f32/f64 の幅・endianness・sign extension・writer round/clamp/cast | Kani symbolic conversion + proptest round-trip | proof 10分以内・4GiB未満 | Required | 長尺 sample loop の性能・allocation |
| WAV-RANGE-008<br>frame count/range validation/byte offset/sample slice | WAV | Bounded Exhaustive local | range arithmetic helper | total/start/end/block_align/data_offset/byte_len/slice index。u64 boundary | Kani local + proptest integration | proof 5分以内・4GiB未満 | Required | 実 I/O short read・allocation 失敗 |
| WAV-WRITER-009<br>writer parameter/header/backpatch/padding | WAV | Property Tested + selective Bounded Exhaustive | parameter gate + fixed-header serialization | block_align/byte_rate overflow・header field・pad/backpatch。fixed sink | proptest primary + Kani local/concrete matrix | proof 10分以内・4GiB未満 | Required | 長尺 streaming write・filesystem 永続化 |
| WAV-ROUNDTRIP-010<br>writer/reader round-trip | WAV | Property Tested primary | end-to-end codec integration | 5 encoding・channels/rate/frame・分割 write。representable input | proptest primary、Kani selective concrete matrix は Deferred | 現行 lane 内、optional proof 10分以内 | Required | clamp/round が効く任意 f64、NaN、PcmU8/PcmS8 writer 不在 |
| WAV-CORRUPT-011<br>破損・切断入力で panic しない | WAV | Property Tested primary + optional bounded integration | whole API integration | 任意 byte・header mutation・truncation | proptest primary、Kani whole-call は Deferred | 現行 lane 内 | Required | 任意長・実尺・全 corruption 組合せの形式網羅 |
| MP4-SIG-001<br>MP4 signature 判定 | MP4 | Bounded Exhaustive local + Property Tested | pure signature predicate | detect_format の先頭 byte/len | Kani local + proptest | proof 2分以内 | Required | ftyp を持たない legal ISO-BMFF は現行契約で未検出 |
| MP4-BOX-002<br>box header size32/size64/size0/overflow/limit の境界 | MP4 | Bounded Exhaustive local | box_header pure boundary | at/limit/size/tag/body_start/body_end 算術 | Kani local | proof 10分以内・4GiB未満 | Required | semantic box type 解釈 |
| MP4-SCAN-003<br>child box scan の進行性・終了性 | MP4 | Bounded Exhaustive local/model | child_boxes walker state | start/end/at/next 単調増加と終了条件 | Kani local | proof 10分以内・4GiB未満 | Required | 多数 box・深い nesting の performance |
| MP4-STRUCT-004<br>必須 box・skipped_boxes・declared facts | MP4 | Property Tested primary | structural demux integration | moov/trak/mdia/mdhd/hdlr/stbl/stsd・skipped paths | proptest primary | 現行 lane 内 | Required | fragmented MP4・edit list・multi-run table の完全網羅 |
| MP4-TABLE-005<br>sample table offset/count/size/time 計算 | MP4 | Bounded Exhaustive local | sample-table arithmetic | stts/ctts/stsc/stsz/stco/cumulative DTS/PTS/offset | Kani local + proptest roundtrip | proof 10分以内・4GiB未満 | Required | multi-chunk/編集リスト全組合せ |
| MP4-PCM-DECL-006<br>PCM codec declaration/capability 判定 | MP4 | Property Tested + selective exhaustive matrix | codec classification | sowt/twos/raw/lpcm/ipcm/fpcm/pcmC/channel/bits/rate | proptest primary + Kani concrete matrix | proof 10分以内 | Required | 新 codec/fourcc の未知領域 |
| MP4-PCM-EXTRACT-007<br>PCM extraction offset/width conversion | MP4 | Bounded Exhaustive local + Property Tested | sample extraction arithmetic/conversion | chunk offset/sample size/width/endian/sign conversion | Kani local + proptest | proof 10分以内 | Required | compressed codec・長尺 payload・performance |
| MP4-PARTIAL-008<br>compressed/missing audio の partial ingest | MP4 | Property Tested | integration behavior | compressed codec/missing audio/capability observation | proptest primary | 現行 lane 内 | Required | 全 compressed codec の実 decode 能力 |
| MP4-ROUNDTRIP-009<br>write_mp4/read facts/timing/PCM round-trip | MP4 | Property Tested primary | writer/reader integration | generated spec matrix | proptest primary + optional concrete Kani matrix | 現行 lane 内 | Required | 広い spec 空間の形式的網羅 |
| MP4-CORRUPT-010<br>破損・切断入力で panic しない | MP4 | Property Tested primary + optional bounded integration | whole parser integration | mutation/truncation/arbitrary bytes | proptest primary、Kani whole-call は Deferred | 現行 lane 内 | Required | 複合 corruption・実尺・全構造の形式網羅 |
| AVI-CHUNK-001<br>RIFF chunk/LIST boundary/padding の安全性 | AVI | Bounded Exhaustive local | RIFF chunk walker | id/size/body_end/pad/form type/region limit | Kani local | proof 10分以内・4GiB未満 | Required | nested LIST/OpenDML/idx1 の全 semantic |
| AVI-STREAM-002<br>strh/strf stream declaration 検証 | AVI | Property Tested + selective local | stream header validation | vids/auds・scale/rate/start/sample_size/strf payload | proptest primary + selective Kani matrix | proof 10分以内 | Required | 未知 codec/圧縮形式の完全網羅 |
| AVI-MOVI-003<br>movi packet id/kind/body boundary の安全性 | AVI | Bounded Exhaustive local | movi chunk walker | NNdc/NNwb id・stream number・kind・body boundary/padding | Kani local | proof 10分以内・4GiB未満 | Required | zero-length/drop frame 以外の codec semantic |
| AVI-TIME-004<br>movi_order timing の累積・remainder・独立 clock | AVI | Bounded Exhaustive local | timing arithmetic | dwStart*dwScale・len*scale/sample_size・remainder・stream別累積 | Kani local + proptest integration | proof 10分以内・4GiB未満 | Required | 長尺 packet 列の性能・全 interleave |
| AVI-PCM-005<br>WAVEFORMATEX PCM decode | AVI | Bounded Exhaustive local + Property Tested | shared waveformatex/PCM conversion | WAV fmt/data と同じ format/bits/block align/conversion | Kani local + proptest | proof 10分以内 | Required | strf 拡張・非 PCM codec |
| AVI-VIDEO-006<br>video metadata/features の整合 | AVI | Property Tested | integration behavior | vids metadata・DIB feature extraction | proptest primary | 現行 lane 内 | Required | 非 DIB/圧縮 video・画像処理全体 |
| AVI-PARTIAL-007<br>missing/undecodable auds の partial ingest | AVI | Property Tested | integration behavior | missing/non-PCM/empty auds/capability observation | proptest primary | 現行 lane 内 | Required | 全 codec capability |
| AVI-ROUNDTRIP-008<br>write_avi/read video/audio/timing round-trip | AVI | Property Tested primary | writer/reader integration | generated spec matrix | proptest primary + optional concrete Kani matrix | 現行 lane 内 | Required | 広い spec 空間の形式的網羅 |
| AVI-CORRUPT-009<br>破損・切断入力で panic しない | AVI | Property Tested primary + optional bounded integration | whole parser integration | mutation/truncation/arbitrary bytes | proptest primary、Kani whole-call は Deferred | 現行 lane 内 | Required | 複合 corruption・実尺・全構造の形式網羅 |

## assurance pack の実行形態

proptest 13件を常時実行 baseline に保ち、Kani は signature/PCM conversion/chunk-box boundary/scan progress/range/timing arithmetic の小さい assurance pack へ再構成する。whole-call symbolic proof は代替が Pass するまで legacy inventory として残し Required proof から外す。予算は「各 Required local proof 10分以内・peak 4GiB未満。WAV/MP4/AVI sequential pack 60分以内。関連変更の property 単位選択実行 10〜20分以内を目標。予算は後続実装 slice の実測で見直す」。CI は Daily Full CI へ集約する非ブロッキング形で、PR 必須 blocking Kani lane は推奨しない。

## RF 還元判定

候補あり。ただし新規 Issue 起票や既存 Issue 更新は本 slice では行わない。`ruviform#1015` が PCM WAV backend・cfg(kani) 診断置換・共通検証支援の再評価を既に担当しており、重複 Issue を作らない。AudioLens 側の候補は `ruviform#1015` の入力情報として共有可能

- 固定長 Write+Seek sink / KaniSink の test-only 検証支援。Ruviform slice4 で有効性が実測済み
- RIFF chunk boundary walker と WAVEFORMATEX/PCM conversion の検証済み共通契約
- diagnostic error payload を検証対象から分離する設計知見。cfg(kani) 置換ではなく lazy diagnostic/contract型の再設計として評価

Coordinator 確認: 未実施。Issue 起票/更新不要と判断したため。

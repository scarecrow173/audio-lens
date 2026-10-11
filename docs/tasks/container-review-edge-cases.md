---
id: "TASK-034dLdqYk3faqjkGcbspVS"
type: "task"
status: "done"
title: "al-media: コンテナ解析の残存エッジケース(PR #25/#27 レビュー LOW)"
created: "2026-10-10"
updated: "2026-10-11"
owners: ["scarecrow173"]
relations:
  source: []
  implements:
    - "docs/plans/residual-fixes.md"
  implemented-by: []
  depends-on:
    - "docs/plans/residual-fixes.md"
    - "docs/designs/residual-fixes.md"
    - "docs/adr/residual-issue-boundaries.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related: []
  refines: []
  refined-by: []
  derives-from: []
  derived-by: []
  verifies: []
  verified-by:
    - "docs/test-specs/residual-fixes.md"
  references:
    - "https://github.com/scarecrow173/audio-lens/issues/28"
  defers: []
  deferred-by: []
---

# al-media: コンテナ解析の残存エッジケース(PR #25/#27 レビュー LOW)

## Work

#28 の残件を以下の原文棚卸しに沿って確認する。Issue 全体を対応単位とし、修正・文書化・承認済み defer の各結果と証拠を項目ごとに残す。本実施では coordinator の承認により task 分割を省略し、項目別証拠と論理別コミットでこの対応を保持する(承認記録は下記)。

## Classification

- `bug-fix`
- design/plan/test-spec の承認と本 Dispatch の実装依頼に基づき実施。判断項目は coordinator の個別承認後に実装。

## Upstream Documents

- `docs/plans/residual-fixes.md`
- `docs/specs/residual-compliance.md` AC-28
- `docs/designs/residual-fixes.md`
- Issue #28

## Dependencies

spec/ADR はチェーンの受理可能な status を満たし、design/plan と test-spec は承認済みのため status todo。

## Affected Files

- `crates/al-media/src/avi.rs`
- `crates/al-media/src/mp4.rs`
- `crates/al-media/src/video.rs`
- `crates/al-media/src/metadata.rs`
- `crates/al-media/src/decoder.rs`, `dib.rs`, `stage.rs`
- `crates/al-analysis/src/avsync.rs`, `stage.rs` と関連テスト
- `crates/al-dataset/src/stage.rs`
- `crates/al-viz/src/`

## Issue Inventory (2026-10-10)

### 背景

PR #25(`feat(al-media)`)と PR #27(`fix(dataset,viz)`)のレビューで検出された非ブロッカー指摘。部分 ingest 契約の境界は確立済みだが、以下の精度・整合性改善が残る。

### 項目

- `avi.rs` `tick_advance`: `len*scale/sample_size` をパケットごとに切り捨てるため、`dwSampleSize` 未整列ボディでタイムラインが最大 ~1 tick/パケットドリフトする(余りを繰り越すべき)
- `mp4.rs`: `stsc`/`stco` の妥当性は PCM 抽出時にしか検証されない。非デコード対象トラックのテーブル破損は未検証のまま通る(コーデックによって fatal か否かが変わる)
- `video.rs`: スクラッチ luma プレーンが `mem::take` で毎フレーム空になり `resize` で再確保される。「フレーム間再利用」のコメントと実装が食い違う
- `avi.rs`: 非フレーム `vids` パケット(`pc` 等)が `dwScale` タイムラインスロットを消費するため、`movi_order` 時刻がフレーム index×scale の意味論とずれる。意図的だがドキュメント化が必要
- integrity kind の命名ドリフト: `audio_decode_unavailable`(AVI)/ `decode/input unavailable`(MP4)/ `video_decode_unavailable` — 1 概念に 3 表記
- avsync の `dwStart` リベースは video 側のみ。`auds` の非ゼロ `dwStart` は相殺されない(設計注記はあるが `dwStart` が意味を持つ入力で実オフセットを捨てる)
- OpenDML AVI(`movi` 内 `LIST 'rec '`)はパースできるがパケット 0 で全 unavailable になり、「OpenDML 非対応」の明示理由がない
- `audio: null` のとき `metadata.json` の source_hash がファイルハッシュを失う(issue #3 の要求メタデータ)
- `dataset_input_unavailable.path` がメタデータ成果物パスを冗長に複製
- `encoding_for` の接頭辞一致が手編集された不正 codec 名を許容しうる
- viz の `expect("inputs")` は declared.json が常に JSON のため実質空のチェック
- AVI のセカンダリ `auds` は MP4 のセカンダリ `soun` より観測情報が少ない
- capture/input 不在時の timeline 欠落が他のスキップと違い記録されない
- 病的ヘッダの checked 時計算術が `Malformed` へエスカレートする境界の妥当性確認

Refs #25 #27

## Steps

1. 原文の各項目を実装と既存 spec に照合し、修正か判断かを記録する。
2. 違反する契約を固定する回帰テストを追加する。
3. 対象テストを実行し、期待する失敗を確認する。
4. 所有境界で修正する。判断項目は承認後に着手する。
5. 対象検証を再実行する。
6. 項目ごとの証拠と文書を更新する。
7. 意味のある変更単位でコミットする。

## Done When

- [x] 未整列 AVI 音声パケットの時刻が累積有理時計と一致する。MP4 の非デコードトラックにも同じテーブル検証が適用される。音声が unavailable でも入力ハッシュと利用不能理由が残る。luma の再利用、非フレーム時刻、dwStart、OpenDML、integrity 名称、二次音声、欠落 timeline の扱いを各境界ケースで検証する。
- [x] 原文全項目に修正・文書化・承認済み defer と検証証拠の対応がある。
- [x] 関連 spec/ADR/design/test-spec が実施内容と一致する。

## Verification

- [x] `mise exec -- cargo test -p al-media -p al-dataset -p al-viz` が exit 0。原文項目に対応した回帰テストを含む。
- [x] `mise exec -- cargo fmt --all -- --check` と `mise exec -- cargo clippy --workspace --all-targets -- -D warnings` が exit 0。
- [x] `git diff --check` が exit 0。
- [x] 文書と Issue 項目の対応を review する。

## 項目別の結果と証拠

以下のテスト名は `crates/al-media/tests/{registry,mp4,ingest}.rs`、
`crates/al-media/src/{avi,video}.rs`、`crates/al-analysis/tests/avsync.rs`、
`crates/al-dataset/tests/integration.rs`、`crates/al-dataset/src/stage.rs`、
`crates/al-viz/tests/viz.rs` にある。表の順序は原文棚卸しと一致する。

| 項目 | 結果・所有境界 | 検証証拠 |
| --- | --- | --- |
| 1. 未整列 auds の tick 余り | 修正: AVI packet clock が stream 単位で有理分子の余りを繰越す。u128 中間算術を使用 | `unaligned_audio_packets_keep_fractional_ticks_across_packets`: scale=3/sample_size=2、1 byte×100、direct/probe とも floor(i×3/2) |
| 2. 非デコード MP4 の stsc/stco | 修正: parse-stbl で全トラックの run 順序、chunk 範囲、spc、stsd index、stsz 件数、EOF 範囲を検証 | `nondecoded_tracks_validate_chunk_mapping_in_direct_and_registry_probe`, `sample_past_eof_is_structural_failure_for_all_codecs`, `excessive_mapped_bytes_are_structural_for_all_codecs`, `empty_complete_sample_table_remains_valid` |
| 3. luma の再利用 | 修正: 共通特徴抽出が比較済み前フレームの buffer を decoder に返し、二枚を交互に再利用 | `luma_planes_are_recycled_after_previous_frame_comparison`: frame 2 以降の capacity 保持、pointer[i]=pointer[i-2]、luminance/diff 一致。既存 DIB テストも通過 |
| 4. 非フレーム vids の時計 | 文書化(承認済み): pc 等も packet-slot を消費。frame index は db/dc だけを数えるので index×scale と時刻は異なる | `non_frame_vids_chunks_dont_desync_frame_indices` と `docs/designs/ingest-av.md`。消費側は persisted ticks を使う |
| 5. integrity kind 命名 | 修正: `decode_unavailable` に統一し detail.media_kind と共通 UnavailableKind (`capability`/`malformed`/`io`) を保存。stage で別名を合成しない | `avi_audio_errors_are_typed_and_independent_clocks_survive`, `zero_rate_pcm_failure_is_malformed_in_direct_and_registry_results`, `mid_sample_pcm_data_is_integrity_observation_not_failure`, `incomplete_pcm_frame_is_local_malformed_in_direct_and_registry`。構造不正は fatal、PCM 不正は局所 malformed |
| 6. auds/vids の dwStart | 修正(承認済み): 音声 PCM 開始を共通原点とし、映像 tick/timebase から選択音声 start_tick/timebase を引く。実相対 offset を保持 | `ingest_then_cheap_recovers_planted_av_offset`: video/audio start=(0,0),(5,0),(5,24000),(5,48000)、期待 delta=0.2,0.7,0.2,-0.3 s。`equal_stream_origins_cancel_without_discarding_relative_offset`, `unavailable_clock_origin_does_not_produce_valid_sync` |
| 7. OpenDML/nested rec LIST | 修正(承認済み): movi 内 rec LIST を InputUnavailable として明示拒否。decoder 拡張はしない | `nested_rec_list_is_explicitly_unsupported_in_direct_and_probe`: direct/probe とも OpenDML/nested rec 理由を返し、packet 0 として通過しない |
| 8. audio null の source_hash | 修正: decoder provenance/manifest params に加え metadata.json の独立 source_hash を保存 | `avi_with_undecodable_audio_still_ingests_observations`, `compressed_only_mp4_completes_with_metadata_and_no_pcm`: audio null でも入力 bytes の sha256 と一致 |
| 9. dataset_input_unavailable.path 重複 | 修正: metadata 一パス一 artifact。PCM 不在時はその metadata record の role を unavailable にする | `partial_input_metadata_has_one_manifest_owner`: 同一パスの record が一つ、source_hash 保持。CLI の compressed-only end-to-end も通過 |
| 10. encoding_for 接頭辞 | 修正: native decoder が出力する12種類の完全 codec 名のみ受理 | `decoded_codec_names_are_exact`: 正当名を受理し、s16/pcm_s16/pcm_s16evil 等を拒否 |
| 11. viz の inputs 検査 | 修正: declared JSON を除外し、各宣言入力の Audio または metadata JSON のパスと store を照合 | `declared_json_alone_does_not_validate_materialized_inputs`: declared のみでは失敗。metadata-only の正当入力は CLI 結合で通過 |
| 12. AVI 二次 auds 観測 | registry で観測済みの codec/rate/channels/packet_lengths を確認し、非選択理由の integrity 保存を追加 | `first_avi_audio_stream_is_the_only_decoded_stream`: 一次 PCM だけを decode、二次 metadata・独立時計・canonical 非選択理由を保持 |
| 13. timeline 欠落 | 修正: capture なし、inputs role なし、capture audio なしの全分岐を skipped に記録 | `timeline_absence_is_recorded_without_inputs_or_capture`, CLI compressed-only pipeline、既存 viz tests |
| 14. checked 時計算術の境界 | 文書化・検証(承認済み): i64 に表現不能な開始/累積時計は構造不正 Malformed。大きいが表現可能な時計は受理 | `avi_clock_range_is_a_structural_boundary_in_direct_and_probe`: u32::MAX start/scale=1 は受理、開始積と packet 累積加算 overflow は direct/probe とも Malformed |

未承認 defer はなく、全14項目を修正または文書化・境界検証で処理した。
rec LIST の扱いは未対応領域を明示する境界修正であり、OpenDML decode 対応の実装ではない。

## 判断と承認

2026-10-11、本 Dispatch の question `msg_7c43909b855e` に coordinator が回答。
同一 task の項目別証拠+論理別コミット、packet-slot 意味論の文書化、
音声/映像共通原点への dwStart 修正(design/avsync 更新を含む)、
rec LIST の InputUnavailable 境界、i64 overflow の Malformed 維持を承認した。
status follow-up `msg_ee08d0808850` でも dwStart 方針を確認した。

## 実行結果(2026-10-11)

- `mise install --locked python uv pipx:apm-cli pipx:serena-agent` と `mise run setup`: exit 0。setup の lock 再解決差分は戻し、依存変更なし。
- `mise exec -- cargo test -p al-media`: exit 0、127件。
- `mise exec -- cargo test -p al-media -p al-dataset -p al-viz`: exit 0、al-media 127件、al-dataset 30件、al-viz 16件、合計173件。
- `mise exec -- cargo test -p al-analysis avsync`: exit 0、avsync unit 6件。
- `mise exec -- cargo test -p al-analysis --test avsync`: exit 0、結合2件(四組の dwStart を含む)。
- `mise exec -- cargo test -p al-cli --test cli compressed_only_mp4_runs_the_default_pipeline_end_to_end`: exit 0、1件。
- `mise exec -- cargo fmt --all -- --check`: exit 0。
- `mise exec -- cargo clippy --workspace --all-targets -- -D warnings`: exit 0。
- `git diff --check`: exit 0。

RED 確認: 未整列時計は期待148 tickに対し99、非デコード不正stscは誤成功、
source_hash は null、metadata パス record は2件、codec s16 は誤受理、
declared-only viz は誤成功、timeline skip は欠落、dwStart の期待0.7秒は約0.21秒、
rec LIST は明示未対応にならず、不完全PCMは capability/decoded と誤分類された。
luma は recycle callback 契約テストの compile failure を先に確認した。
各修正後の対応テストは上記 suite で成功した。

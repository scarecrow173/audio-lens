---
id: "IMPL-034QqbkXUnlzmJBg7NmVtx"
type: "impl"
status: "completed"
title: "Ruviform rev bump + consumer regression verification (Issue #127)"
created: "2026-11-02"
updated: "2026-11-02"
owners: []
relations:
  source: []
  changes:
    added:
      - type: "doc"
        path: "docs/impl/ir/ruviform-rev-bump-consumer-regression-verification-issue-127.md"
    modified:
      - type: "code"
        path: "Cargo.toml"
      - type: "code"
        path: "Cargo.lock"
      - type: "doc"
        path: "docs/impl/ir/README.md"
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
  related:
    - "docs/designs/ruviform-dependency-ci.md"
  refines: []
  refined-by: []
  derives-from: []
  derived-by: []
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
metadata:
  experiments:
    adopted: []
    rejected: []
---

# Ruviform rev bump + consumer regression verification (Issue #127)

## Summary

AudioLens が消費する Ruviform git 依存を
`080c228d197ebc645b8f81cd0082be442abaf10c` から
origin/main `7c430212efe4c14612968e107cdb5f9233c88952`
(Issue #233 merge 時点)へ一括 bump し、Issue #127 の consumer
回帰を repo 内検証範囲で実測した。公開 API 差分は全て additive
または非消費面で、AudioLens 側の call-site 修正は 0 件。
既存テスト 542 件全緑、退行なし。

## Context

Issue #127(AudioLens と Synvelis を Ruviform の対等 consumer
として追従)の継続タスク。Ruviform に graph IR/typed runtime、
transport scheduler、media reader/prefetch(#197+#233)、WAV
encode(#191)、generic probe(#185)、param events(#186)、timed
MIDI(#188)、sequential offline render(#198)、multi-window
host(#181)の大型 merge wave が入ったため、pin 更新と回帰評価を
行う。手順・再現性の規約は
`docs/designs/ruviform-dependency-ci.md` §1-§2 に従う
(rev 一括置換 → Cargo.lock 単一 revision 確認 → mise task で検証)。

## Implementation

### Changes

- `Cargo.toml` `[workspace.dependencies]` の ruviform 14 entry の
  `rev` を `080c228d…` → `7c430212…` に一括置換(規約どおり全
  entry 同一 SHA)
- `Cargo.lock`: `cargo update -p ruviform-media-core` で再解決。
  全 15 ruviform source(直接 14 + 推移 ruviform-rt)が単一
  `?rev=7c430212…#7c430212…` に統一されたことを確認。旧 rev
  参照 0 件
- AudioLens ソースの call-site 変更: **なし**(後述の影響分析で
  全差分が additive / 非消費面と確認)

### Resolved revision

- before: `080c228d197ebc645b8f81cd0082be442abaf10c`
- after:  `7c430212efe4c14612968e107cdb5f9233c88952`
  (`git ls-remote` の `refs/heads/main` と一致 — main HEAD)
- 差分: 144 files、+32054/-605(docs 除く)。主要 merge:
  ruviform-graph M1(#184)、typed graph audio runtime M2(#195)、
  rational beat + variable PPQN(#189)、transport span
  scheduler(#190)、MediaSink/StagedOutput + io-wav encoder(#191)、
  generic probe(#185)、media reader + bounded prefetch + RT
  fill(#197)、prefetch 契約改善(#233)、param events(#186)、
  plugin bridge M4(#196)、timed MIDI(#188)、sequential offline
  render(#198)、multi-window host(#181)、cross-repo consumer
  CI(#219)、各種 CI/macOS/lint 修正

### Impact analysis(AudioLens 消費面)

消費 API を列挙し、消費する crate の差分を個別に確認した。

- **ruviform-media-io**(al-media が消費): `DecodeMode` enum 新設、
  `Demuxer::seekable()` / `Demuxer::make_decoder(id, mode)` が
  default impl 付きで追加、`MediaSink`/`StagedOutput`/
  `PublishOptions` export 追加。`Decoder`/`Demuxer`/`Error`/
  `ErrorKind`/`Frame`/`Packet`/`CodecId`/`CodecDescriptor`/
  `CodecFormat`/`StreamInfo`/`StreamFacts`/`MediaSource` の既存
  契約は無変更 → **additive のみ**
- **ruviform-io-symphonia / ruviform-io-avi**(al-media が消費):
  各 demuxer に trait `make_decoder(id, mode)`・`seekable()` を
  実装追加。AudioLens が呼ぶ inherent `make_decoder(id)` /
  `make_native_decoder(id)` / `open_demuxer(source)` は同一
  signature で残存(inherent method が名前解決で優先、arity も
  異なり衝突なし)。symphonia は構築時に seekability を probe
  する内部変更のみ → **compatible**
- **ruviform-media-core**(al-media が消費): `Fact`/`StreamId`/
  `StreamKind`/`MediaTime`/`TimeBase`/`TimestampProvenance` 等の
  消費 file(fact/stream/time/observe/timecode/transport/error)は
  無変更。`music.rs` のみ **breaking 変更**(`TempoMap`/
  `TempoEvent`/`TimeSignatureEvent` が `Tick` ではなく
  `TickPosition`/`BeatPosition`/`Ppqn` 引数に、`Ppqn::new` が
  `Result` 返却に — #189 rational beat)だが AudioLens は music
  API を import していないため影響なし
- **ruviform-audio-analysis**(al-analysis が消費):
  `ChannelLevel::measure` 抽出の内部 refactor、`LevelMeter` の
  数値契約同一 → **compatible**
- **ruviform-numeric**(al-analysis/al-locate が消費):
  `ResamplePlan::from_params` の u32→usize を checked 化
  (64-bit で挙動同一)、`ALGORITHM_ID` 不変 → **compatible**
- **ruviform-exec**(al-job が消費): `OfflineJob::spawn`/
  `cancel`/`Progress`/`JobScope`/`CancellationToken`/
  `OfflineConfig` 無変更。`spawn_with_cancel_after`・
  `Progress::bump_by` 追加のみ → **additive のみ**
- **ruviform-ui**(al-gui が消費): `UiRuntime::handle`/
  `dispatch_intent`/`poll`/`drain_jobs` の signature・
  単一 window 挙動(CloseRequested で job cancel + `closing`
  event)は無変更。`HostEvent` に `Moved`/`Destroyed` variant 追加
  (`#[non_exhaustive]` のため source 互換)。`ServiceContext` に
  `origin: Option<WindowId>` field 追加 — AudioLens は
  `ServiceContext` を literal 構築せず `&mut` で受けるのみ。
  `window` module / `handle_window`/`drain_jobs_routed`/
  `cancel_window_jobs` は multi-window 向け新規 additive →
  **compatible**(単一 window 経路は契約不変と doc に明記)
- **ruviform-ui-webview**(al-gui `webview` feature が消費):
  `run`/`HostConfig`/`detect_backend`/`webview_available` 無変更。
  `session.rs` の `run_multi`/`SessionConfig` 追加と
  `build_webview` の内部 refactor のみ → **compatible**
- **ruviform-audio-core / video-core / dsp / math / viz**:
  audio-core は `midi_input` 追加のみ、その他は差分なし →
  **無変更**
- **toolchain**: ruviform 側 rust 1.99.0 pin は無変化
  (AudioLens pin と一致を維持)。serena-agent lazy 化のみ

### Decisions During Implementation

- **backend entry point を維持**: 新 trait `Demuxer::make_decoder
  (id, mode)` への移行は行わず、既存の inherent
  `make_decoder(id)`/`make_native_decoder(id)` 呼出しを維持した。
  両者は同一契約の別入口であり、現行呼出しは意図どおり
  canonical/native を選んでいる(強制移行ではない)
- **Ruviform Issue 起票なし**: 消費面の破壊的変更・契約違反は
  検出されなかった。`music.rs` の breaking change は Ruviform
  が所有する意図的契約変更(#189)で AudioLens 非消費面のため
  起票対象外と判断

## Related Experiments

### Adopted

なし。

### Rejected

なし。

## Validation

- `git ls-remote … refs/heads/main` = `7c430212…`(pin = main HEAD)
- `mise exec -- cargo fetch --locked`: pass(private fetch、
  GCM/system git 経由、lockfile 再現)
- `mise exec -- cargo tree`: ruviform source は全 entry 単一
  rev `#7c430212`、symphonia 0.6.1 単一 version
- `mise run fmt` / `check` / `lint`: pass(clippy `-D warnings`
  clean。biome は既存の JS asset warning/info のみ、新規指摘なし)
- `mise run verify-toolchain`: rustc/cargo 1.99.0 + rustfmt/clippy
- `mise run check-cli-headless`: 10 headless crate に GUI
  framework dep なし(#127 GUI/headless 分離項目)
- `mise run test`: **542 passed / 0 failed**(46 test binary)
- `mise run check-ui-webview` / `lint-ui-webview` /
  `test-ui-webview`(al-gui `webview` feature lane、wry/tao 実窓
  host target 含む): pass、5 テスト全緑
- `mise run verify-ci`(fmt → `check-ci` → `lint-ci` → `test-ci`、
  `--profile ci`、`target/ci/` 分離): pass、46 test binary 全緑

### Consumer checklist(#127)との対応

- Media ingest(WAV/AVI/MP4/AAC、native PCM format/bit depth、f64
  native 経路、raw PTS/DTS/timebase、provenance、corrupt/
  discontinuity/underrun/EOF/silence 区別、暗黙 f32 化なし):
  **検証済み** — al-media テスト群(wav/avi/mp4/registry/
  ruviform_deps/native 経路 tests)が全緑。`Frame::NativeAudio`
  f64 経路・endianness・edit list 適用を含む
- DSP/分析 f64 経路(FFT/STFT/NCC/GCC-PHAT/band/golden/tolerance):
  **検証済み** — al-analysis/al-locate の golden・characterization
  系テスト全緑
- Offline/Output(chunk 並列解析 vs stateful 逐次、float64 WAV
  dataset/export): **検証済み** — al-analysis 並列 stage
  (Issue #54/#71 系)・al-service export/job テスト全緑。
  ruviform io-wav encoder は AudioLens 非消費(独自 WAV 実装を
  維持)のため新 Encoder 影響なし
- Job/Dataset(resume/cancel/manifest/partial output/GUI・CLI 共有):
  **検証済み** — al-job/al-service/al-dataset テスト全緑。
  `RUVIFORM_REV` は `al-core/build.rs` が Cargo.lock から導出し
  単一 rev でない場合 build fail(#115)。provenance 一致は
  `al-media/tests/ingest.rs` と `al-analysis/src/records.rs` が
  `decoder.version` / `params.ruviform.rev` ==
  `al_core::RUVIFORM_REV` を直接 assert(#115/#117 回帰項目)
- GUI/headless(単一 window close/job cancel 不変、CLI に
  WebView/GPU 非依存): **検証済み** — `check-cli-headless` pass、
  `UiRuntime::handle` 単一 window 契約不変を差分で確認、
  al-gui headless テスト(service_ops)と webview feature lane
  (check/lint/test)全緑。実窓の GUI 手動挙動確認は未実施
- 性能・平台: **一部未検証** — 本検証は Windows dev/ci profile。
  macOS/Linux lane・benchmark/baseline 計測は未実施

## Risks

- `music.rs` の `Tick`/`TempoMap` 系は意図的 breaking(#189)。
  AudioLens が将来 music API を消費する場合は新 signature に
  追従が必要
- `ServiceContext.origin` 追加により literal 構築している他
  consumer(Synvelis)は追従が必要になる可能性(本 repo 範囲外)
- `seekable()`/`DecodeMode` は宣言的契約の前半で、consumer 側の
  実シーク経路は未実装のまま(既存挙動と同一、悪化なし)

## Follow-ups

- macOS/Linux lane・baseline 計測は別トリガ(CI/別 Issue)で実施
- Issue #127 本体の checklist に今回の検証結果を反映するかは
  coordinator/PR レビューで判断(本 record が根拠)

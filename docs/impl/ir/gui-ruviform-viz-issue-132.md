---
id: "IMPL-DDwFaI0Gd2SuWUe3iM02A"
type: "impl"
status: "completed"
title: "Ruviform 可視化部品で Analysis Dataset・同期 Timeline・Feature 閲覧 headless core(Issue #132)"
created: "2026-11-03"
updated: "2026-11-03"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/audio-lens/issues/132"
  changes:
    added:
      - type: "code"
        path: "ui/src/viz/"
      - type: "code"
        path: "ui/test/viz-decode.test.ts"
      - type: "code"
        path: "ui/test/viz-sources.test.ts"
      - type: "code"
        path: "ui/test/viz-timeline.test.ts"
      - type: "code"
        path: "ui/test/viz-features.test.ts"
      - type: "code"
        path: "ui/test/viz-style.test.ts"
      - type: "doc"
        path: "docs/tasks/gui-ruviform-viz-issue-132.md"
      - type: "doc"
        path: "docs/impl/ir/gui-ruviform-viz-issue-132.md"
    modified:
      - type: "code"
        path: "tools/sync-ruviform-ui.mjs"
      - type: "code"
        path: "ui/src/port.ts"
      - type: "code"
        path: "ui/src/gateway.ts"
      - type: "code"
        path: "ui/src/index.ts"
      - type: "code"
        path: "ui/test/fakes.ts"
    deleted: []
    renamed: []
    moved: []
    generated:
      - type: "code"
        path: "ui/vendor/ruviform/ui/viz/"
  implements:
    - "docs/tasks/gui-ruviform-viz-issue-132.md"
  implemented-by: []
  depends-on:
    - "docs/impl/ir/gui-ruviform-controls-issue-131.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/impl/ir/issue-105-gui-app.md"
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

# Ruviform 可視化部品で Analysis Dataset・同期 Timeline・Feature 閲覧(Issue #132)

## Summary

#131 の headless 統合層上に、Analysis Dataset の永続値を
vendored ruviform `ui/viz` の data port 契約へ適合させる
`ui/src/viz/` を新設した。`al.view.waveform`/`al.view.spectrogram`
が publish する `RVBW`/`RVST` payload を、bounded viewport
query(`QueryBinding` 経由)+ `AlServicePort.pull` の範囲読みで
復号し、`f64` 精度・`NaN` 欠測・STFT 軸写像を保持したまま
供給する。加えて `SyncTimeline`(track/alignment/time_map/
event marker)と `FeatureCatalog`(verbatim record + provenance
解決)、AudioLens theme → `VizStyle` ブリッジを実装した。
実 desktop GUI shell が無いため実窓検証は未検証として記録する。

## Context

task `docs/tasks/gui-ruviform-viz-issue-132.md`、Issue
[#132](https://github.com/scarecrow173/audio-lens/issues/132) に従う。
サービスの publish 契約(`{source, bytes, meta}`)は
`crates/al-service/src/dataset.rs`、wire format は ruviform
`crates/ruviform-viz/src/encode.rs`(pin `e9e6abe`)を正本とする。

## Implementation

### Vendored `ui/viz` の取り込み

- `tools/sync-ruviform-ui.mjs` の同期対象に `ui/viz` を追加し
  `ui/vendor/ruviform/ui/viz/` を `e9e6abe` から再生成
  (`mise run check-ui` の `--check` で REV 整合を確認)。
  `ui/viz` は `WaveformDataSource`/`MeterDataSource`/
  `SpectrumDataSource` の data port、`Rvf*` element、scene
  builder、`VizStyle` を持つ framework-neutral 層。

### `ui/src/port.ts` — `pull`

- `AlServicePort` に `pull(source, offset, limit)` を追加。
  `al.view.*` op が返す `{source, bytes, meta}` descriptor の
  `source` を `rvf://data` 相当の範囲読み port として読む
  (service 側 `PagedBytes` と同契約 — EOF short read 許容、
  unknown source は error)。

### `ui/src/viz/decode.ts` — strict decoder

- `RVBW`: 16B header(magic/version/bins)+ 48B/bin
  (`start u64, end u64, min/max/rms f64, count u64`)。
  upstream `encode.rs` の doc 表記 `44 B` は stale で、実 wire は
  `count u64` を `+40..+48` に書く 48B(crate 側 layout test が
  `rec[40..48]` を読むことで確認)。48B で decode/encoder を統一。
- `RVST`: 24B header(magic/version/dtype/cols/rows/frame_start)
  + row-major `rows×cols`(row 0 = 最低 Hz)。`f32`/`f64` は
  IEEE NaN をゼロコピー view で透過、`u8` の `255` は再正規化
  せず sentinel として残す。
- magic/version/dtype/長さの不整合は例外で失敗する
  (破損を空データへ畳まない)。

### `ui/src/viz/sources.ts` — dataset → viz adapter

- `DatasetWaveformSource`(`WaveformDataSource` 実装): query は
  `QueryBinding.request` 経由で `waveform/<session>/<inputId>/<ch>`
  sourceId + dataset fingerprint revision + `{start_s,end_s,bins}`
  を `al.view.waveform` へ bounded 転送。descriptor 検証後に
  `pull` で宣言 `bytes` 以内を chunked 読みし `RVBW` 復号。
  covered range は `LodBytes.start/end` meta(encode 側の権威値)
  から秒へ写像し `unitsPerBucket` を計算。channel 選択は
  per-channel op に分解し、channel 間の bin 数不一致は error。
- `DatasetSpectrogramSource`: `spectrogram/<session>/<inputId>/
  <channel>/<rows>` sourceId で rows を source grammar へ載せ、
  `cols` は `buckets` budget として `al.view.spectrogram` へ届く
  (gateway `sourceOp` が `cols=q.buckets`/`rows=src.rows` を
  forward)。`SpectrogramTileView` は `frame f は f*hop 始点` 規約で
  `colStartSeconds`/`colEndSeconds`/`rowStartHz`/`rowEndHz` の
  軸写像を持つ。
- 共通: `AbortSignal` → `binding.cancelAll()` で in-flight 全体を
  cancel(port `cancelQuery` にも届く)。stale/over-budget/
  cancelled は `VizQueryFailure`/`AbortError` として表面化し、
  結果を適用しない。`maxBytes` は `QueryBinding` の
  over-budget 判定へそのまま渡す。

### `ui/src/viz/timeline.ts` — `SyncTimeline`

- `al.ds.overview`/`al.ds.locate`/`al.ds.locate_detail`/
  `al.ds.events` の応答から、capture 秒を共有軸とする
  timeline モデルを構築。
- track: manifest inputs と timeline audio artifact の差分を
  `available`/`missing` で表現(audio 無し input を黙って
  消さない)。extent は capture 系 track の最大 duration。
- alignment: `source_input_id`/`capture_input_id` ペアで索引し
  status/validity/confidence/agreeing_methods/capture range/
  detail/time_map を verbatim 保持。
- `captureToSourceS`: `locate_detail` 応答の `{result, derived}`
  envelope を unwrap し、`result.normalized_source_range`/
  `capture_range`/`source_pcm.sample_rate` で定数オフセット写像
  + `time_map` segment 補間(`TimeMap::capture_to_source` と
  同規約 — anchors が無い/range 外は `undefined` で写像を
  捏造しない)。time_map 経路の rate は normalized rate
  (capture rate への resample 座標)を使う。
  - 既知のギャップ: 現行 dataset は `time_map.json` を job dir
    のみに persist し `locate/` role へ materialize しない
    (al-dataset `materialize_locate`)。`timeMaps` 引数経由の
    供給(`timeMapViewFromWire` で serde wire 形→view 変換)まで
    time_map 経路は到達しない。dataset 側 materialize は別
    issue 扱い。
- `markers`: `Rvf*.overlays` にそのまま渡せる
  `{at,label,color}` 形状。位置の無い event 行は marker に
  せず `eventRows` 側へ残す。

### `ui/src/viz/features.ts` — Feature 閲覧

- `FeatureCatalog`: `al.ds.features` の record を verbatim 保持
  (validity/confidence/support_count/lineage/`invalid_reason`
  を落とさず、`truncated` も保持)。`inspect` は record の
  `_artifact` path → `al.ds.artifact` で detail/provenance を
  解決し、path 無し record は `undefined`(経路を捏造しない)。
- `featureSpectrumSource`: 数値列 `value` を持つ record のみ
  `SpectrumDataSource` へ適合。`axis` は必須 — vendored
  `SpectrumInfo.axis` は Hz 型のみで bin index を `hz` へ
  流すと非周波数軸を Hz と偽るため、axis の無い record は
  `undefined`(spectrum truth を捏造しない。非 Hz 軸表現は
  Ruviform `BinAxis` の責務 — upstream 拡張要)。

### `ui/src/viz/style.ts` — theme ブリッジ

- `vizTokenMap`/`resolveAlVizStyle`: `AUDIOLENS_THEME` の
  `viz.*` token を `resolveVizStyle` の token map へそのまま渡す
  (`palette[i]←viz.series.i`、`state.gapFill←viz.gap.fill` 等)。
  theme 差替えは paint のみ変え、data query・revision・再解析に
  影響しない(ruviform VizStyle 契約どおり)。generic renderer に
  AudioLens 診断語彙は持ち込まない。

### `ui/src/gateway.ts` — sourceId grammar 拡張

- `waveform|spectrogram/<session>/<inputId>[/<channel>][/<rows>]`:
  channel は既定 0、`rows` は spectrogram のみ正の整数。
  数値 segment は canonical な 10 進(`^\d+$`)のみ受理 —
  `""`/`0x1`/`3.0`/`1e1`/空白込みを `Number()` で黙認しない。
  `events|overview` は trailing segment を拒否、waveform は
  4・spectrogram は 5 segment 上限。
- `sourceOp`: spectrogram は `cols=q.buckets ?? 1024` +
  `rows=src.rows`(定義時のみ forward)、waveform は
  `bins=q.buckets`、features は `limit=q.buckets`。
- `query` は dataset fingerprint を `sourceRevision` として
  stamp し、reply `bytes` を byte budget 判定へ供給。

### `ui/test/fakes.ts` — 実応答形への拡充

- `FakeDataset` に timeline inputs・event/locate/feature/
  artifact/STFT(`window`/`hop`/`bins`)・`waveformFn`/
  `spectrogramFn`・`timelineInputs` 差分を追加。
- `al.view.waveform`/`al.view.spectrogram` は実 service と同じ
  covered range/frame 換算で `RVBW`/`RVST-f32` を emit して
  `data` レジストリへ publish、`{source,bytes,meta}` を返す。
- `pull` はレジストリから offset/limit の範囲読み(EOF short
  read、unknown source は `not_found`)。
- `encodeRvbw`/`encodeRvstF32` を export し decode 単体 test が
  同一 wire writer を再利用する。
- `advanceDataset` が overview fingerprint を変え、stale
  検出の検証を可能にする。

## Verification

- `mise run check-ui` — vendor `REV` check + `tsc --noEmit`: PASS
- `mise run test-ui` — vendor check + node:test:
  112 tests / 36 suites 全 PASS(#131 から +43 tests 増、
  #132 追加分)
- `mise run lint` — biome(ui/ + tools/)・cargo clippy: PASS
- `mise run check` — Rust workspace: PASS(TS/JS のみの変更で
  回帰なし)
- review(protocol 2 名)後の修正: >4ch fan-out の自己 evict
  防止(`maxInflight=max(4,channels)`)、`pullChunkBytes<=0`
  無限ループ防止、`locate_detail` の `result` envelope
  unwrap、time_map normalized-rate 除算・wire 形
  (`capture_range`)変換 `timeMapViewFromWire`、数値 segment
  厳密 parse、反転/非整数 viewport・範囲外 channel の
  error/unsupported 表面化、RVST 非 aligned decode の
  aligned copy、fake の channel bound/row bound/publish 名/
  frame clamp/truncated 境界を実 service へ揃正、
  `featureSpectrumSource` の axis 必須化。
- 未検証: 実 desktop ウィンドウ上の描画・同期スクロール・
  theme 切替・実 WebView の `rvf://data` 登録。実 GUI 環境
  (#105 の webview shell)がこの worktree では未整備のため、
  headless 契約検証をもって完了とし実窓証跡は残さない。

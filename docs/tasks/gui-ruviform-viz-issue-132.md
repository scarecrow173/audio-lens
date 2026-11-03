---
id: "TASK-2ZeVHW4ux9HuAQOf46bGt"
type: "task"
status: "done"
title: "Ruviform 可視化部品で Analysis Dataset・同期 Timeline・Feature 閲覧を構築(AL#132)"
created: "2026-11-03"
updated: "2026-11-03"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/audio-lens/issues/132"
  changes:
    added: []
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/plans/ruviform-foundation-migration.md"
  implemented-by:
    - "docs/impl/ir/gui-ruviform-viz-issue-132.md"
  depends-on:
    - "docs/tasks/gui-ruviform-controls-issue-131.md"
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

# Ruviform 可視化部品で Analysis Dataset・同期 Timeline・Feature 閲覧を構築(AL#132)

## Work

#131 の headless GUI 統合層(vendored ruviform
`ui/{core,components,binding}` + `AlGuiGateway` + `AUDIOLENS_THEME`)
の上に、永続化済み Analysis Dataset を vendored ruviform `ui/viz`
(#260)の data port 契約へ適合させる層を構築する。Issue
[#132](https://github.com/scarecrow173/audio-lens/issues/132) を正本とする。

- `al.view.waveform`/`al.view.spectrogram` が publish する
  `RVBW`/`RVST` payload を bounded range/tile/LOD query +
  `rvf://data` 相当の範囲 pull で復号し、ruviform viz の
  `WaveformDataSource` と同形の tile port へ供給する
- `f64` 精度・IEEE `NaN`(欠測 bucket/`255` sentinel)を
  wire のまま保持し、表示側で補間・0 埋め・逆正規化しない
- STFT の frame/hop/sample_rate と uniform bin Hz 軸で
  秒・Hz 写像を保持する
- source/capture/reference の対応関係は AudioLens view model
  (`SyncTimeline`)が所有し、alignment の status/validity/
  confidence/agreeing_methods/capture range/detail/time_map を
  verbatim 保持する — 写像不能箇所は `undefined` で返し捏造しない
- Feature 閲覧は `al.ds.features` の record verbatim +
  `_artifact` path 経由の `al.ds.artifact` detail/provenance 解決
  で行い、数値列のみ `SpectrumDataSource` frame 化する
- Ruviform 側に AudioLens 診断語彙を持ち込まず、`VizStyle`/
  reference renderer は AudioLens theme token の供給のみ
  (paint のみ切替、data query・再解析に影響しない)
- stale revision/cancel/over-budget は `QueryBinding` の
  outcome 語彙で表面化し、古い generation を描画しない
- 長尺 dataset を viewport 外まで全量読みしない(bounded
  op params + 宣言 bytes 以内の pull のみ)
- `al-cli` とサービスを共有し、GUI/WebView 依存を解析
  core/CLI へ持ち込まない
- 実 desktop GUI shell(#105)が無い環境では headless 統合層 +
  テストを成果物とし、実 GUI/ウィンドウ検証は未検証として記録

## Classification

- `normal-plan-task`

## Upstream Documents

- `docs/tasks/gui-ruviform-controls-issue-131.md`
- `docs/tasks/ruviform-gui-app.md`
- `docs/specs/gui-app-mvp.md`
- Issue [#132](https://github.com/scarecrow173/audio-lens/issues/132)

## Dependencies

- `docs/tasks/gui-ruviform-controls-issue-131.md`(gateway/sourceId
  grammar、QueryBinding、theme、port)
- 外部依存: Ruviform ui/viz(#260)の `RVBW`/`RVST` wire format・
  `WaveformDataSource`/`SpectrumDataSource`/`VizStyle` 契約 —
  ruviform main `e9e6abe` でマージ済み
- `crates/al-service/src/dataset.rs` の `al.view.*` publish 契約
  (`{source, bytes, meta}`)

## Affected Files

- `tools/sync-ruviform-ui.mjs`(vendored package に `ui/viz` 追加)
- `ui/src/port.ts`(`AlServicePort.pull` 追加)
- `ui/src/gateway.ts`(spectrogram `rows`/`cols` forwarding、
  sourceId grammar に `rows` segment)
- `ui/src/viz/`(新規: `decode.ts`/`sources.ts`/`timeline.ts`/
  `features.ts`/`style.ts`/`index.ts`)
- `ui/src/index.ts`(`viz/*` export)
- `ui/test/fakes.ts`(pull + publish レジストリ + RVBW/RVST
  emit + 実応答形の ds/view stub)
- `ui/test/viz-{decode,sources,timeline,features,style}.test.ts`
- `docs/tasks/gui-ruviform-viz-issue-132.md` /
  `docs/impl/ir/gui-ruviform-viz-issue-132.md`

## Done When

- [x] persistent dataset 由来の waveform/spectrogram が bounded
      query + range pull で viz data port へ届く(再解析・全量
      読みなし)
- [x] `f64` 精度と `NaN`/`255` sentinel の欠測表現が wire の
      まま保持される
- [x] STFT frame↔秒・bin↔Hz の軸写像が service と同規約で
      提供される
- [x] 同期 timeline が track availability・alignment 詳細・
      capture→source 写像・event marker を verbatim 保持で
      供給する
- [x] Feature record が validity/confidence/support/provenance
      ごと保持され、detail 解決は artifact path 経由のみ
- [x] stale/cancelled/over-budget が honest outcome で表面化する
- [x] AudioLens theme が `VizStyle` token 経路で paint のみ
      切替可能
- [x] headless 検証 task(`mise run check-ui`/`test-ui`)が green
- [ ] 実 GUI/ウィンドウ環境での検証(実環境未整備のため
      未検証として記録)

## Verification

- [x] `mise run check-ui`(vendor pin check + typecheck)
- [x] `mise run test-ui`(104 tests / 36 suites)
- [x] `mise run lint`(biome ui/ + tools/、cargo clippy)
- [x] `mise run check`(Rust workspace 回帰なし)
- [ ] 実ウィンドウ上の描画・同期スクロール・theme 切替
      (実 GUI 環境なし、未検証)

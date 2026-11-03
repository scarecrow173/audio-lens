---
id: "IMPL-APfHsvtdVEm01PMVUM2Jvg"
type: "impl"
status: "completed"
title: "Ruviform Controls・Gesture・Binding の GUI 統合 headless core(Issue #131)"
created: "2026-11-03"
updated: "2026-11-03"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/audio-lens/issues/131"
  changes:
    added:
      - type: "code"
        path: "tools/sync-ruviform-ui.mjs"
      - type: "code"
        path: "ui/"
      - type: "doc"
        path: "docs/tasks/gui-ruviform-controls-issue-131.md"
      - type: "doc"
        path: "docs/impl/ir/gui-ruviform-controls-issue-131.md"
    modified:
      - type: "code"
        path: "Cargo.toml"
      - type: "code"
        path: "Cargo.lock"
      - type: "code"
        path: "mise.toml"
      - type: "code"
        path: ".gitignore"
      - type: "code"
        path: ".worktreeinclude"
    deleted: []
    renamed: []
    moved: []
    generated:
      - type: "code"
        path: "ui/vendor/ruviform/"
  implements:
    - "docs/tasks/gui-ruviform-controls-issue-131.md"
  implemented-by: []
  depends-on:
    - "docs/impl/ir/issue-105-gui-app.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/impl/ir/ruviform-rev-bump-consumer-regression-verification-issue-127.md"
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

# Ruviform Controls・Gesture・Binding の GUI 統合(Issue #131)

## Summary

Ruviform UI lane(ui/core・ui/components・ui/binding)を
Cargo pin `e9e6abe` へ bump して取り込み、AudioLens 側に
framework-neutral な `ui/` TypeScript パッケージを新設した。
AudioLens 固有の論理ターゲット(`al.view.*`/`al.cfg.*`/
`al.job`/`al.media.preview`)を Ruviform の Command/Query
binding 契約へ写像する `AlGuiGateway`、製品テーマ
`AUDIOLENS_THEME`、4 画面 fixture(Job settings /
media preview / seek-range / analysis settings)を実装し、
happy-dom 上の headless 検証 69 テストを整備した。
実 desktop GUI shell が無いため実窓検証は未検証として記録する。

## Context

task `docs/tasks/gui-ruviform-controls-issue-131.md`、Issue
[#131](https://github.com/scarecrow173/audio-lens/issues/131) に従う。
サービス語彙は `crates/al-gui/src/service.rs`(#105、
`al.app.*`/`al.job.*`/`al.ds.*`/`al.view.*` op)を正本とし、
`ui/src/ops.ts` がそれを TS 側へ写像する。

## Implementation

### Ruviform pin bump

- `Cargo.toml` の ruviform rev 14 箇所を
  `b9f2a10` → `e9e6abe` へ更新し `Cargo.lock` 再解決。
  2 SHA 間で `crates/` 差分はゼロ(UI lane は `ui/` TS のみ
  追加)であり、Rust 側 `cargo check`/`cargo test` で
  回帰なしを確認済み。

### Vendored UI source 同期

- `tools/sync-ruviform-ui.mjs`: `Cargo.toml` の
  `ruviform-*` git dep から URL/SHA を一意に読み取り、
  分散や欠落を reject。system git(cargo と同一 credential
  経路)で `git archive` し `ui/core`・`ui/components`・
  `ui/binding` のみ `ui/vendor/ruviform/` へ展開、
  `REV` に SHA を記録。`--check` で整合検証。
- vendor tree は `.gitignore` 済み生成物(commit せず
  pin から再生成)。`.worktreeinclude` に登録し worktree
  間で引き継ぐ。

### `ui/` パッケージ(`@audiolens/gui-headless`)

- `src/port.ts`: `AlService`/`AlJobHandle` インターフェース。
  長時間処理は `al-cli`/service の `CallOutcome::Job` 相当の
  handle を返し、cancel 済み run が job_id を返さない場合は
  `al.job.list` で再発見する前提。
- `src/ops.ts`: `al-gui` service の op 語彙の TS ミラー。
- `src/view-state.ts`: `RevisionedStore<P>` — 単調増加
  revision 付きの authoritative 値。subscribe 時に即時
  projection を push し、stale projection(rev 逆行)を
  破棄する。
- `src/gateway.ts`: `AlGuiGateway`(BindingPort 実装)。
  - `al.view.seek`/`al.view.channel`/`al.view.range`(
    `{low,high}` 保持、`sel-lo`/`sel-hi` 両 binding が同一
    target に commit)、`al.cfg.<path>`、`al.cfg.stages.<stage>`
    (数値→enabled stage 集合)、`al.job`(run/resume/cancel)、
    `al.media.preview`(play/pause/resume/stop、Job とは独立)
    の論理ターゲットを dispatch。
  - `dispatchFor(intent, ctx, ack)` が binding 固有の ack
    closure へ receipt を返す(`BindingPort.dispatch` 自体は
    ack を取らないため、consumer 側で per-binding 配線)。
  - missing source→error、stale revision→stale、
    unsupported→unsupported、service 例外→error を区別し、
    確定値や完了を偽装しない。
  - Query は sourceId(例 `waveform:<session>:<role>:<name>`)
    → `al.view.waveform`/`al.view.spectrogram`/`al.ds.*` へ
    写像し、`sourceRevision`/`generation`/`start`/`end`/
    byte bound を伝達。AbortSignal と `cancelQuery` 双方で
    cancel。byte 超過は `over-budget`。
- `src/theme.ts`: `AUDIOLENS_THEME`(dark Reference Skin 基盤
  + teal accent + AudioLens semantic/viz 色)。theme は
  data-only で操作ロジック・binding を変更しない。
- `src/screens/shared.ts`: `ViewLink` + `bindControlElement`
  系の mount scaffolding。`bindRangeEdges` は `valueLow`/
  `valueHigh` の controlled write(プロジェクション反映は
  gesture を発火しない Ruviform 契約)。
- `src/screens/`: job-settings(全 stage toggle + config
  numeric + Run/Resume/Cancel)、media-preview(seekbar +
  transport、Job ライフサイクルと分離)、seek-range
  (seekbar + range-selector、commit 済み `{low,high}` から
  `start`/`end` 付き query 発行)、analysis-settings
  (fader/numeric/toggle → `al.cfg.*` + RunSpec draft)。

### 精度・正直性

- `intent.value.plain` の `f64` を gateway→RunSpec まで
  丸めず保持。表示整形(`toFixed`)は presentation 境界のみで
  authoritative 値へ逆流しない(control descriptor の
  `step` snap は shared control 契約上の明示的丸めとして
  許容し、no-step 経路で verbatim を検証)。
- Dataset 表示 query は `al.job.run`/`resume`/`cancel` を
  呼ばない(テストで op 呼び出し監視)。
- delayed/reordered/duplicate receipt、ack timeout、
  detach/remount、2-window 転送(Job cancel しない・
  subscription 解放・attach 後に authoritative projection
  再読)を ViewLink/BindingState の契約どおり検証。

### Review hardening(cooperative + adversarial review の確定指摘)

- `nestConfig`/`al.cfg.*` commit で `__proto__`/`constructor`/
  `prototype` を含む path を拒否し prototype pollution を防止。
- `receiptLog` を直近 512 件に丸め、長期稼働での無制限増を防止。
- `openDataset` で overview 失敗時に `al.ds.close` を試行し
  session 孤児化を防止。
- `subscribe` は未宣言 target を lazy 宣言し、subscribe 先行でも
  後続 commit の projection を受け取る。
- query viewport/channel/rows に finite・非負整数 validation を
  追加し、`AbortSignal` を `cancelQuery` 経由で service 側 cancel
  hook へ forward。
- range commit は非 finite・反転(`low > high`)を rejected とし、
  `seedRange` で初期値を一回だけ seed(remount で上書きしない)。
- 明示的に全 stage を off にした場合は `stages: []` を送り、
  「未選択=省略(service 既定全 stage)」と区別(`stagesEdited`)。
- run 実行中の 2 個目の run は rejected(単一 tracking slot を
  保護)。cancel action receipt は `accepted`(受理)で、最終結果は
  done 通知が `rejected:cancelled` として届く形に修正。
- job_id 未指定の resume は `al.job.list` で候補を再発見し、
  0 件・複数件は推測せず rejected。
- job-settings/analysis-settings の mount 時既定値は初回のみ
  seed し、remount でユーザー選択を巻き戻さない。
- 実 `ErrorBody` は `data` を持たないため fake からも除去し、
  `al.ds.overview` を実応答形 `{overview, timeline}` に修正。

## Verification

- `mise run check-ui` — vendor `REV` check + `tsc --noEmit`: PASS
- `mise run test-ui` — vendor check + node:test:
  69 tests / 23 suites 全 PASS
- `mise run lint` — biome(ui/ + tools/)・cargo clippy:
  PASS(review で検出した ui/ lint 逸脱を修正済み)
- `mise run check` / `mise run test`(Rust workspace、pin bump
  後): PASS(回帰なし)
- 未検証: 実 desktop ウィンドウでの表示・focus・keyboard・
  high-contrast・touch・実 2-window 移動。実 GUI 環境(#105 の
  webview shell)がこの worktree では未整備のため、headless
  契約検証をもって完了とし実窓証跡は残さない。

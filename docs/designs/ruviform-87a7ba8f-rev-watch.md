---
id: "DESIGN-ErtvZsCassKtIf2ZFlOxfb"
type: "design"
status: "approved"
title: "ruviform 87a7ba8f pin 追従の設計"
created: "2026-11-26"
updated: "2026-11-26"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/audio-lens/issues/127"
  changes:
    added:
      - type: "file"
        path: "docs/designs/ruviform-87a7ba8f-rev-watch.md"
    modified:
      - type: "file"
        path: "Cargo.lock"
      - type: "file"
        path: "tools/update-ruviform-rev.mjs"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/specs/ruviform-87a7ba8f-rev-watch.md"
  implemented-by: []
  depends-on:
    - "docs/specs/ruviform-87a7ba8f-rev-watch.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/designs/ruviform-e4c9f6f0-rev-watch.md"
---

# ruviform `87a7ba8f` pin 追従の設計

## 方針

lockfile-primary 方式を維持する。manifest に `rev`/`branch`/`tag` を書かず、
`node tools/update-ruviform-rev.mjs <sha>` が全 `ruviform-*` package を
単一 resolved SHA に原子解決する。失敗時は lockfile を変更しない
fail-closed 契約を維持する。`ui/vendor/ruviform/` は
`mise run sync-ui-deps` の生成物で、gitignore 済みのため commit しない。

本 slice では candidate freeze 後の upstream drift を追いかけない。
dispatch 時に再解決した `87a7ba8ff3fbe6d4502bea27d152d7f74c105899` を
評価対象とし、後で観測した `0fb97d1ba89e1fee154cdbbb72ceb2db15fdcd8c`
は次回 rev-watch へ送る。これにより、検証中に対象が動くことを防ぐ。

## 実施手順

1. `git ls-remote https://github.com/scarecrow173/ruviform.git
   refs/heads/main` で candidate を再解決する。dispatch 時の実測値は
   `87a7ba8ff3fbe6d4502bea27d152d7f74c105899`。
2. `node tools/update-ruviform-rev.mjs
   87a7ba8ff3fbe6d4502bea27d152d7f74c105899` を実行し、25 個の
   ruviform package を単一 SHA に更新する。
3. `mise run sync-ui-deps` で `ui/vendor/ruviform` を追随させ、
   `ui/vendor/ruviform/REV` が candidate と一致することを確認する。
   vendor は生成物なので commit しない。
4. `mise exec -- cargo fetch --locked` で locked dependency を fetch する。
5. `mise run check-ruviform-lock`、`mise run check-ruviform-rev`、
   `mise exec -- node tools/check-ruviform-rev.mjs --require-vendor` を
   実行し、lockfile-primary・単一 SHA・vendor REV 一致を確認する。
6. canonical 回帰 lane を実行し、結果を
   `evidence/al127/87a7ba8f-verification.md` に記録する。

`tools/update-ruviform-rev.mjs` の最終案内文は、存在しない
`verify-ci` ではなく `sync-ui-deps`・`cargo fetch --locked`・
`check-ruviform-lock`・`check-ruviform-rev` を示すように修正した。
これは案内文の stale task 名修正であり、pin 更新アルゴリズムや
fail-closed 契約は変更していない。

## 互換性判断

### lockfile / transitive dependency

`Cargo.lock` の差分は、25 個の ruviform source fragment と `gpui` の
source switch のみである。`[[package]]` section の追加・削除はなく、
無関係な version bump はない。upstream `ruviform-ui`/`ruviform-ui-test` が
追加した `proptest` は dev-dependency のため consumer 依存解決へ波及せず、
AL lock の `proptest` entry は既存 1 件のままである。

`gpui` は upstream `ruviform-ui-gpui` の dependency 変更により、
crates.io `0.2.2` から `scarecrow173/gpui` tag `v0.2.2-rf2`（peeled
commit `b3144f286b942fbb5f478da73db13a826cef054a`）へ切り替わる。
upstream 側の説明では crates.io 0.2.2 を基礎に DirectComposition target
を non-topmost にする修正を含む。AL 側で source を戻さず、
`build-ui-gpui` の Native Build/Link で採用可否を検証する。

### 数値・解析系

`ruviform-math/src/dsp.rs` はコメント参照のみの差分で、production
意味論は変わらない。`ruviform-audio` の ProbeDeck capture push は
`debug_assert!` 内副作用から常時実行へ修正されているが、AL は
`ProbeDeck` を直接参照しない。既存 consumer oracle（golden/reference/
rf628_oracle/ruviform_deps）で f64・長尺・PTS・gap・Validity の非回帰を
確認する。

### plugin-core/plugin-host

`Plugin::buses` の signature は `&[BusDescriptor]` から
`Option<&[BusDescriptor]>` へ変わる破壊的変更である。ただし AL は
plugin API を実装・利用しない（`crates/`/`tools/`/`ui` の grep 実測）。
したがって AL 側適合コードは不要で、transitive compile が通れば採用可能
と判断する。AL が将来 plugin host を提供する場合は、この契約変更を
その導入 slice の設計条件として扱う。

wire version 5→9 と GUI floating/editor verbs、dirty/param notices、
report drain surface は upstream L1 plugin boundary の拡張である。
AL の永続化形式・CLI/GUI command schema ではないため、本 slice では
consumer 影響なしと分類する。`HostCommand` は `#[non_exhaustive]` で、
追加 verb は AL の exhaustive matching を壊さない。

### UI/GPUI

`ruviform-ui` の `InputSpec`/`InputValueSpec` と `ViewHost::submit_input`
は additive である。`CommandParam`/`CommandParams` は runtime-entered
value の typed profile として追加されたが、既存
`CommandIntent::Action.params` は旧 pin にも optional/default の
`serde_json::Value` として存在し、serde 形状は変わらない。したがって
AL の TypeScript gateway と Rust command handler が想定する既存 wire は
非回帰と判断する。

`ComponentSpec`/`ScenarioCapability` は `#[non_exhaustive]` で、
`Input`・`OutOfProcessPluginHost` の追加は既存 consumer の exhaustive
match を壊さない。AL の `test al-gui`/`test-ui-gpui` で scenario parity
を確認する。

`ruviform-ui-gpui` の IMM32 IME・native surface 変更と gpui fork switch
は real GUI リスクを伴う。headless slice では semantic lane と native
build/link のみを採用条件とし、interactive real window・pixel・OS input・
IME capability の Pass は主張しない。

## 新 API の採用/見送り

- `InputSpec`/`InputValueSpec`/`submit_input`: 見送り。AL GUI に
  free-form input widget の要件はまだない。
- `CommandParam`/`CommandParams`: 見送り。AL の既存 action は typed
  profile を必要としない。必要になった場合は consumer schema と一緒に
  別 slice で設計する。
- plugin GUI/editor verbs・dirty/param notices・wire v6-v9: 見送り/
  NotApplicable。AL は plugin host を提供しない。
- IMM32 IME・real window capability: Deferred。interactive session での
  実測前に capability claim へ転用しない。

## リスク

- upstream main は candidate freeze 後も進む。`0fb97d1b` は次回
  rev-watch で再評価する。
- `87a7ba8f` 自体に upstream covering CI run はない。直近 ancestor
  `ab6dc99f` の success と `ab6dc99f..87a7ba8f` が production code 差分
  なしであることを補助証拠とするが、AL 側 merge 後 CI が統合検証の正本
  である。
- gpui source switch は dependency provenance の変更であり、crates.io
  release と bit-identical ではない。本 slice では build/link と semantic
  lane の成功を確認するが、実 GUI 表示・pixel・IME は未検証として残す。
- plugin-core/host の大規模差分は AL 非消費のため compile 影響に限定
  されるが、将来 plugin host を導入する場合は `Plugin::buses` 契約と
  wire v9 を前提に再設計する必要がある。
- local/headless lane の成功は統合受入ではない。macOS/Linux を含む
  OS-L1 統合判定と Windows GUI-L2 は merge 後 CI・後続 slice の証拠で
  判断する。

---
id: "DISC-034Vc1kRZ36gwoKV3ifQy1"
type: "discovery"
status: "resolved"
title: "Legacy WebView 棚卸し（AL#127 / RF#465 ゲート）"
created: "2026-11-10"
updated: "2026-11-10"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/audio-lens/issues/127"
    - "https://github.com/scarecrow173/ruviform/issues/465"
    - "https://github.com/scarecrow173/ruviform/issues/419"
  changes:
    added:
      - type: "file"
        path: "docs/discovery/issue-127-legacy-webview-inventory.md"
    modified: []
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
    - "docs/designs/ruviform-271e2d41-rev-watch.md"
    - "docs/specs/ruviform-271e2d41-rev-watch.md"
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

# Legacy WebView 棚卸し（AL#127 / RF#465 ゲート）

## Summary

Issue #127 の Legacy 撤去 Gate に従い、AudioLens に残存する旧
WebView Host/feature/TS vendor/IPC/CI/配布の実在箇所を棚卸しし、
「残存（維持）/ 撤去候補 / 独立 Issue 化すべきもの」に分類した。
本 slice では記録のみで物理撤去は行わない。旧 `al-viz` 廃止
（AL#106）とは別系統の撤去対象である。

## Exploration Goal

- 旧 WebView 経路の実在箇所（コード・feature・CI・配布）を
  特定し、撤去時の影響範囲を確定する。
- 各箇所が「現役の build/test/配布経路」か「文書上の言及のみ」かを
  区別する。
- 撤去の前提条件（RF#465 upstream、AL 側 GPUI/実機/pixel の独立
  gate）を明示する。

## Key Issues and Assumptions

- `al-gui` bin は 2 系統: `al-gui`（`webview` feature 必須、
  `ruviform-ui-webview` host）と `al-desktop`（`gpui` feature 必須、
  AL#166 の GPUI backend composition root）。旧 WebView は現役の
  実行・CI lane であり、単なる死にコードではない。
- 旧 WebView/Browser での成功は GPUI・実 OS・pixel の acceptance
  を代替しない。`gui-real` CI job は `not_run` placeholder であり
  native GUI pass の証拠ではない。
- `ui/vendor/ruviform` は webview 専用ではなく binding/guitest
  共用の vendored source であり、webview 撤去では消えない。

## 棚卸し表（実在箇所と分類）

凡例: **維持** = 撤去対象外または撤去時も残る / **撤去候補** =
Legacy 撤去 slice で除去対象 / **独立 Issue** = 別 Issue での
判断・実施が必要。

| # | 箇所 | 内容 | 分類 | 備考 |
| --- | --- | --- | --- | --- |
| 1 | `Cargo.toml:77` | workspace dep `ruviform-ui-webview`（Git、rev なし） | 撤去候補 | `al-gui` の `webview` feature が唯一の consumer。feature 撤去と同時に除去 |
| 2 | `crates/al-gui/Cargo.toml:16,30` | optional dep + `webview = ["dep:ruviform-ui-webview", "ruviform-ui-webview/webview"]` feature | 撤去候補 | feature ごと除去。`gpui`/`gpui-test` feature は別系統で維持 |
| 3 | `crates/al-gui/Cargo.toml:38-40` | `[[bin]] al-gui`（`required-features = ["webview"]`） | 撤去候補 | 旧 WebView host 実行体。`al-desktop`（`required-features = ["gpui"]`）が後継 |
| 4 | `crates/al-gui/src/bin/al-gui.rs` | WebView host 本体。`run_multi`/`detect_backend`/`webview_available`/`SessionConfig`/`WebViewWindowConfig` を使用し driver-neutral session を webview driver に接続 | 撤去候補 | 唯一の `ruviform-ui-webview` 直接 consumer |
| 5 | `crates/al-gui/src/bin/al-desktop.rs` | GPUI desktop bin（対比記録。webview 非依存） | 維持 | AL#166 の GPUI 経路。撤去対象ではない |
| 6 | `crates/al-gui/src/assets.rs` | GUI 共通 asset（webview/gpui 共用の可能性） | 維持 | webview 固有でないため撤去対象外。撤去時に利用側を再確認 |
| 7 | `mise.toml:179-189` | `check-ui-webview`/`lint-ui-webview`/`test-ui-webview` task | 撤去候補 | feature 撤去と同時に除去。`check-ci-workflow.mjs` 等の整合も必要 |
| 8 | `.github/actions/webview-verify/action.yml` | webview feature の check/lint/test を OS matrix で実行する composite action | 撤去候補 | `ci.yml` の webview-verify job が唯一の caller |
| 9 | `.github/workflows/ci.yml:274-313` | `webview-verify`（ubuntu/windows matrix）と `webview-verify-macos` job | 撤去候補 | 現役 CI lane。撤去は CI 定義・`check-ci-workflow.mjs`・集計 job の同時更新が必要 |
| 10 | `.github/workflows/ci.yml:324-389` | `gui-real` job（`not_run` placeholder、status=not_run を証跡出力） | 維持 | 実 GUI gate は未接続のまま正直に記録する placeholder。webview 撤去とは独立 |
| 11 | `tools/check-ci-workflow.mjs` | CI job 構成の期待値に `webview-verify`/`webview-verify-macos` を含む | 撤去候補 | lane 撤去時に同時更新必須（現状は webview lane を要求） |
| 12 | `tools/check-headless-deps.mjs:47,53,70` | headless crate への GUI 依存混入を検査する guard が `ruviform-ui-webview`/`webview2-*` を GUI 依存として認識 | 維持 | dep が存在する間は guard の知識として必要。撤去後にエントリ整理を検討 |
| 13 | `ui/vendor/ruviform/**` + `REV` | vendored ruviform UI source（`sync-ui-deps` 生成物） | 維持 | webview 限定ではなく `ui/binding`/`ui/guitest` が共用。pin 追従で再生成される |
| 14 | `ui/binding`・`ui/guitest`（TS） | binding/query epoch 契約と semantic oracle。WebView host API ではなく driver-neutral 層 | 維持 | webview 撤去後も AL の TS headless 統合が利用 |
| 15 | IPC/binding path | `al-gui.rs` → `ruviform-ui-webview` の `run_multi` session 接続（host 側）。TS 側は driver-neutral で webview 固有 IPC を持たない | 撤去候補（#4 に包含） | webview 固有の IPC は host bin と `ruviform-ui-webview` crate に閉じる |
| 16 | 配布/packaging | installer/bundle 等の packaging 定義は存在しない。配布物は cargo bin（`al-gui`/`al-desktop`）のみ | 維持（該当なし） | webview 専用の配布経路は現状なし。将来 packaging を作る場合は GPUI 側を前提に設計する |
| 17 | `docs/designs/gui-app-architecture.md`、`docs/{designs,specs,test-specs}/webview-host-api-migration*.md`、`docs/impl/ir/issue-105-gui-app.md` | 旧 WebView host の設計・移行仕様・実装記録 | 維持 | 履歴文書。撤去 slice で「Legacy/廃止予定」注記の更新を検討（独立 Issue） |

## Alternatives and Comparison

- **今すぐ物理撤去**: 不採用。RF#465 の consumer 撤去 Owner Issue
  と AL 側の独立精度 gate（GPUI 実機・pixel・cross-consumer）が
  未充足であり、Issue #127 の受入条件も「棚卸しと分類の記録まで」
  と明示する。
- **撤去候補を独立 Issue 化**: 採用。撤去は複数ファイル・CI・
  guard・文書にまたがる論理単位であり、#4 の受入 gate 成立後に
  別 slice として実施するのが適切。

## Tentative Conclusions and Hypotheses

- 旧 WebView の実在箇所は「`al-gui` bin + `webview` feature +
  optional dep + webview-verify CI lane 群 + 関連 tool/mise task」
  に限定される。これ以外（vendor・binding・guitest・docs 履歴・
  `al-desktop`・`gui-real` placeholder）は撤去対象ではない。
- 撤去の実質的前提: (a) RF#465 upstream の撤去契約確定、
  (b) AL 側で GPUI 経路が WebView 経路の提供能力
  （window/desktop 操作・診断）を独立 gate で満たすこと、
  (c) `gui-real` 等の実機 gate の整備。現時点では (b)(c) が
  NotRun のため撤去は時期尚早。
- `al-viz` 廃止（AL#106）とは別系統: `al-viz` は旧可視化 crate
  の廃止であり、本棚卸しの WebView host/feature/CI は GUI backend
  の世代交代に関する撤去対象。

## Open Questions

- 撤去を実施する AL 側 Issue の起票タイミングと Owner（RF#465 の
  consumer 撤去 Owner Issue 確定後）。
- `check-headless-deps.mjs` の webview 系エントリを撤去 slice で
  整理するか、GUI 依存の一般知識として残すか。
- `gui-app-architecture.md` 等の文書へ「Legacy」注記を入れるタイミング。

## Promotion Candidates

- 撤去候補一覧（#1-#4, #7-#9, #11, #15）→ 撤去 slice の spec 入力。
- 「旧 WebView 成功 ≠ GPUI/実機/pixel acceptance」の原則 →
  test-spec R9 と PR 本文の NotRun 欄に反映済み。

## Sources and Relations

- Issue #127（受入条件: 棚卸しと分類の記録まで、物理撤去なし）
- ruviform#465（staged WebView removal inventory、upstream 側）
- ruviform#419（GPUI adoption/native capability gate）
- 関連 AL 文書: `docs/designs/gui-app-architecture.md`、
  `docs/specs/webview-host-api-migration.md`、
  `docs/test-specs/webview-host-api-migration.md`、
  `docs/impl/ir/issue-105-gui-app.md`

## Evidence

- `Cargo.toml:77`、`crates/al-gui/Cargo.toml:16,30,38-40`、
  `crates/al-gui/src/bin/al-gui.rs`、`crates/al-gui/src/bin/
  al-desktop.rs` の実読
- `mise.toml:179-189`（webview lane task 定義）
- `.github/workflows/ci.yml:270-313,324-389,448`（webview-verify
  job、gui-real placeholder、needs/集計）
- `.github/actions/webview-verify/action.yml`（composite action）
- `tools/check-ci-workflow.mjs:14,215-216,235-236,305-306,335`、
  `tools/check-headless-deps.mjs:14,47,53,70`
- `ui/package.json` に webview 依存なし、`ui/vendor/ruviform` は
  `sync-ui-deps` 生成物であることを確認

---
id: "GUID-EJZYFSEFHKBPX9SV24D2E0JFFW"
type: "guide"
status: "active"
title: "リリース Gate: macOS 実測検証手順"
created: "2026-11-05"
updated: "2026-11-05"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/audio-lens/issues/149"
  changes:
    added: []
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
    - "../designs/full-ci-ci-summary-issue-158.md"
    - "../designs/audiolens-independent-ci.md"
  refines: []
  refined-by: []
  derives-from: []
  derived-by: []
  verifies: []
  verified-by: []
  references: []
  defers: []
---

# リリース Gate: macOS 実測検証手順

## 適用条件

macOS 対応を表明する次のイベントすべてに適用する:

- 配布物の作成・公開(インストーラ/バイナリ/nightly 等)
- リリース候補(RC)・正式リリースのタグ付け
- 正式な macOS 互換性の受入れ・互換性表明

**PR の Required check `summary` が緑であることは macOS PASS を
意味しない。** macOS leg は `summary` の needs・判定対象から
物理分離されている(#157/#158 で維持)。Issue #158 以降、
`*-macos` legs は全イベントで常時実行され、結果は Full CI
verdict `ci-summary` が fail-closed で集約する — macOS 失敗/
未完了は `ci-summary` 失敗となるが、`summary` は Required の
ため PR 緑 + macOS 赤の状態は merge 可能であり得る。リリース
判断は必ず本手順の実測結果で行う。

## 必須証拠(候補 SHA ごと)

リリース対象候補のコミット SHA について、該当範囲の macOS 実測が
すべて PASS であることを確認する:

| 証拠 | lane | 内容 |
| --- | --- | --- |
| Golden/f64/Raw timing/Stream provenance/Stage resume/Job/Dataset/Export | `verify-macos` | `cargo test` 系全回帰(LFS fixture 使用) |
| CLI | `verify-macos` | `al-cli` ci profile ビルド + headless smoke |
| UI | `ui-verify-macos` | vendor 整合 + tsc + happy-dom suite |
| WebView | `webview-verify-macos` | wry/webkit2gtk headless 検証 |

- 上記 lanes の実行は対象 SHA で完了した CI run の
  `verify-macos`/`ui-verify-macos`/`webview-verify-macos` job が
  `success` であることで確認する
- `ci-summary` job が `success` であることを確認する
  (macOS legs を含む全必要 job の fail-closed verdict)。
  leg が queue 滞留・未実行・失敗のままの状態では macOS 対応を
  表明してはならない(選択実行の `NotApplicable` 分類は #158 で
  廃止 — 全 leg が常時実行されるため「変更なし = 実測不要」の
  根拠は存在しない)

## 手順

1. リリース候補 SHA を確定する(通常は merge 済み `main` HEAD)
2. 当該 SHA の CI run(main push または `workflow_dispatch`)を
   確認し、`verify-macos`/`ui-verify-macos`/`webview-verify-macos`
   が**実行済みで success**であることを確認する:
   `gh run list --branch main --commit <SHA>` →
   `gh run view <run-id> --log` または Checks タブ
3. `ci-summary` job の集約表を確認し、全 lane `success` を
   確認する。macOS leg が queue 滞留・skipped・失敗なら完了を
   待つか rerun する(runner 逼迫で未実行のまま PASS 扱いに
   しない — `ci-summary` 自身が Pending/failure のまま残る)
4. いずれかが `Fail`/`Unsupported` の場合、macOS 対応表明は
   保留とし、失敗の修正をリリースブロッカーとして扱う
5. 実測結果(SHA・run URL・各 lane 結果)をリリース記録に
   添付する

## 注意

- `ci-summary` は Required context ではないため、失敗していても
  merge は可能(Required は macOS 非含有 scope の `summary`)。
  リリース判断は check の機械的な緑ではなく本手順の確認による
- PR `summary` の緑 + macOS leg 未実行(queue 滞留)のまま
  リリースしない — `ci-summary` が `Pending`/失敗のままなら
  実測未完了である
- gui-real(実 GUI 自動試験)は従来どおり not_run placeholder
  であり、本手順の必須範囲には含まれない。実 GUI 検証が必要な
  場合は別途手動手順で実施する

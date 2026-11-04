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
    - "../designs/impact-based-ci-issue-149.md"
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
意味しない。** Issue #149(2026-11-05 追加合意)により、macOS
leg は `summary` の needs・判定対象から物理分離され、非ブロッ
キング check `macos-summary` に記録される。PR 緑 + macOS 赤の
状態は merge 可能であり得るため、リリース判断は必ず本手順の
実測結果で行う。

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
- `macos-summary` check が `Pass`(全 lane)であることを確認する。
  `Pending`/`NotRun`/`Fail`/`Cancelled`/`Unsupported` が 1 つでも
  残る状態では macOS 対応を表明してはならない
- leg が分類上 `NotApplicable`(正当 skip)の場合、その範囲は
  変更が存在しないため実測不要と扱えるが、根拠(分類証跡)を
  記録に残す

## 手順

1. リリース候補 SHA を確定する(通常は merge 済み `main` HEAD)
2. 当該 SHA の CI run(main push または `workflow_dispatch`)を
   確認し、`verify-macos`/`ui-verify-macos`/`webview-verify-macos`
   が**実行済みで success**であることを確認する:
   `gh run list --branch main --commit <SHA>` →
   `gh run view <run-id> --log` または Checks タブ
3. `macos-summary` check の集約表を確認し、全 lane `Pass` を
   確認する。`Pending`/`NotRun` があれば完了を待つか rerun する
   (runner 逼迫で未実行のまま PASS 扱いにしない)
4. いずれかが `Fail`/`Unsupported` の場合、macOS 対応表明は
   保留とし、失敗の修正をリリースブロッカーとして扱う
5. 実測結果(SHA・run URL・各 lane 結果)をリリース記録に
   添付する

## 注意

- `macos-summary` は非ブロッキング check のため、失敗していても
  merge は可能。リリース判断は check の機械的な緑ではなく
  本手順の確認による
- PR `summary` の緑 + macOS leg 未実行(queue 滞留)のまま
  リリースしない — `macos-summary` が `Pending` のままなら
  実測未完了である
- gui-real(実 GUI 自動試験)は従来どおり not_run placeholder
  であり、本手順の必須範囲には含まれない。実 GUI 検証が必要な
  場合は別途手動手順で実施する

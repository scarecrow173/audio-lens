---
id: "GUID-01MEQrzUeWpGxpaz28tpXGzDUj4cU"
type: "guide"
status: "active"
title: "Ruviform 依存の追従更新・検証・rollback 手順(Cargo.lock 正本)"
created: "2026-11-04"
updated: "2026-11-04"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/audio-lens/issues/140"
  changes:
    added: []
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements: []
  implemented-by: []
  depends-on:
    - "../designs/ruviform-lockfile-source-of-truth.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "../designs/ruviform-dependency-ci.md"
    - "./ruviform-private-access-ci.md"
    - "https://github.com/scarecrow173/ruviform/issues/219"
    - "https://github.com/scarecrow173/ruviform/issues/294"
  refines: []
  refined-by: []
  derives-from:
    - "../designs/ruviform-lockfile-source-of-truth.md"
    - "https://github.com/scarecrow173/audio-lens/issues/140"
links:
  self: "./ruviform-lockfile-update.md"
---

# Ruviform 依存の追従更新・検証・rollback 手順(Cargo.lock 正本)

AL#140 以降、Ruviform の解決 commit SHA の正本は `Cargo.lock` の
`[[package]].source` fragment(`git+<url>#<sha>`)である。manifest
(`Cargo.toml`)は `git` URL と `features` のみを宣言し、`rev` を
持たない。本書は candidate SHA への**明示的な追従更新**、検証、
rollback、provenance 記録の手順をまとめる。

## 用語と正本

| 対象 | 正本 | 確認コマンド |
| --- | --- | --- |
| Rust crate の解決 SHA | `Cargo.lock` fragment | `mise run check-ruviform-lock` |
| vendored `ui/*` TS の取得 SHA | `ui/vendor/ruviform/REV` | `mise run check-ui`(`--check` 内蔵) |

`check-ruviform-lock` は全 ruviform crate(推移依存を含む)が
同一 40 桁 SHA に解決されることを fail-closed で検証する。
ゼロ件・SHA 分裂・未知 source 形式・manifest との URL 不整合は
すべて非ゼロ終了。

## 通常運用(変更しない)

```bash
mise exec -- cargo fetch --locked   # lockfile 記録 SHA を取得
mise run check-ruviform-lock        # 単一 resolved SHA を検証
mise run verify-ci                  # fmt -> check-ci -> lint-ci -> test-ci
mise run sync-ui-deps && mise run check-ui && mise run test-ui
```

`--locked` 系コマンドは Cargo.lock を書き換えない。lockfile が
manifest と不整合なら cargo がエラーにする(lockfile を更新する
には本書の明示更新手順を使う)。

## 追従更新(candidate SHA への差し替え)

更新は専用の scratch branch / 更新 PR で行い、無検証の自動
main 追従は行わない。

1. **candidate SHA を確定する**。Ruviform 側の main HEAD または
   consumer-verify が提示する candidate を使う
   (Ruviform #219/#294 と連携)。

   ```bash
   git ls-remote https://github.com/scarecrow173/ruviform.git HEAD
   ```

2. **lockfile を明示更新する**。

   ```bash
   node tools/update-ruviform-rev.mjs <40-hex sha>
   ```

   内部では `cargo update -p <ruviform-pkg> --precise <sha>` を
   実行する。cargo は同一 git URL を単一 source id で解決するため
   更新は全 ruviform crate に原子的に適用され、candidate SHA に
   存在しない crate 構成では Cargo.lock を変更せず失敗する。
   更新後に本ツールが lockfile 契約を再検証する。
   manifest 編集は不要(AL#127 までの `rev` 14 件一括置換は廃止)。

   Ruviform #219 の consumer-verify からの candidate 注入も
   同じ操作(manifest に `rev` が無くても lockfile 経路で動く)。
   旧方式(manifest `rev` 書換え)との相互互換は、manifest に
   `rev` を復活させれば従来どおり lockfile が `?rev=<sha>#<sha>`
   形に戻るだけであり、本契約ツールはその形式も正しく読める。

3. **差分を記録する**。

   ```bash
   git diff Cargo.lock
   git diff --stat
   ```

   `Cargo.lock` 以外に差分が無いことを確認する(manifest・
   コードは不変のはず)。

4. **vendor と検証を再実行する**。

   ```bash
   mise run sync-ui-deps && mise run check-ui && mise run test-ui
   mise run verify-ci
   ```

5. **Golden/性能差分を評価する**。f64 値・入力時刻・音声/分析
   決定・Stage resume・Export の Golden は `cargo test`(test-ci)
   に含まれる。candidate SHA で差分が出た場合は非等値を更新 PR
   に明示しレビューを受ける。ベンチ比較が必要な場合は
   `tools/baseline` / `tools/perf-reeval` の既存手順に従う。

6. **承認済み PR のみ main へ**。CI(`ci.yml`)は手動 dispatch で
   `check-ruviform-lock` + UI lane も含めて検証する。

## rollback

```bash
git checkout -- Cargo.lock    # lockfile を直前の SHA へ即時復帰
mise run sync-ui-deps         # vendor を lockfile に追随して再生成
mise run check-ruviform-lock  # 復帰後の単一 SHA を検証
```

manifest 側は SHA を持たないため復帰対象は lockfile のみ。
未 commit の lockfile 変更を手放す操作のため、実行前に
`git status` で意図しない差分が無いことを確認する。
`update-ruviform-rev.mjs <旧SHA>` の再実行でも同等に戻せる。

## 失敗時の復旧

| 症状 | 原因候補 | 対処 |
| --- | --- | --- |
| `cargo fetch --locked` で lockfile 更新要求エラー | manifest 変更に lockfile が未追随 | `cargo update -p <pkg> --precise <sha>` か update ツールで明示更新 |
| `check-ruviform-lock` が `diverged` | lockfile 手編集・更新途中失敗 | `git checkout -- Cargo.lock` で復帰してやり直し |
| 同 `no packages resolved` | lockfile 未生成・URL 書換え事故 | Cargo.lock を復元。url.insteadOf 設定の見直し |
| `sync-ui-deps` が SHA を fetch できない | candidate SHA が ruviform に到達不能(force-push 等) | ruviform 側の到達可能 SHA を選び直す。pin commit は ruviform main 上に保つ運用 |
| `--check` が `vendor out of sync` | vendor 未同期 | `mise run sync-ui-deps` で再生成(vendor は手編集しない) |

## provenance 記録

- 実行時の ruviform revision は常に
  `node tools/ruviform-lock-rev.mjs` で lockfile から復元できる。
  更新 PR・検証記録・IR にはこの出力 SHA を併記する。
- CI では `check-ruviform-lock` step が resolved SHA をログに
  出力する。
- UI vendor は `ui/vendor/ruviform/REV` に SHA を記録し、
  `check-ui`/`test-ui` が lockfile との一致を検証する。

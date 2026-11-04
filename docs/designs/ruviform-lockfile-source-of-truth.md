---
id: "DESIGN-6TbEQKqXEHQV8F3VPpYZej"
type: "design"
status: "approved"
title: "Ruviform 依存の Cargo.lock 正本化設計"
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
  implements:
    - "../specs/ruviform-lockfile-source-of-truth.md"
  implemented-by: []
  depends-on:
    - "./ruviform-dependency-ci.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "./ruviform-dependency-ci.md"
    - "https://github.com/scarecrow173/ruviform/issues/219"
    - "https://github.com/scarecrow173/ruviform/issues/294"
  refines:
    - "./ruviform-dependency-ci.md"
  refined-by: []
  derives-from:
    - "../specs/ruviform-lockfile-source-of-truth.md"
    - "./ruviform-dependency-ci.md"
  derived-by: []
  verifies: []
  verified-by: []
  references:
    - "https://github.com/scarecrow173/audio-lens/issues/127"
    - "https://github.com/scarecrow173/audio-lens/issues/99"
  defers: []
  deferred-by: []
---

# Ruviform 依存の Cargo.lock 正本化設計

## Context

Issue [#140](https://github.com/scarecrow173/audio-lens/issues/140)
([spec](../specs/ruviform-lockfile-source-of-truth.md))。
main の現状(AL#137 適用後):

- `Cargo.toml` `[workspace.dependencies]` の Ruviform 直接 Git
  依存 14 entry が同一 `rev = "073a3ff…"` を列挙し、
  `Cargo.lock` は推移依存 `ruviform-rt` を含む 15 package で
  `source = "git+…?rev=<sha>#<sha>"` に解決している。
- `tools/sync-ruviform-ui.mjs` は manifest の `rev` 列挙を必須とし、
  そこから読んだ URL+SHA で `ui/*` を `ui/vendor/ruviform/` に
  展開する。manifest から `rev` を外すだけでは UI 同期が壊れる。
- `#99` 由来の設計(`./ruviform-dependency-ci.md`)は manifest
  `rev` を正本としていた。本設計はその §2 契約を改訂する
  (同ドキュメントの credential・CI 骨格は維持)。

決定: **解決された commit SHA の正本は `Cargo.lock` であり、
manifest は URL と features のみを宣言する**。`--locked` 系
コマンドは lockfile を変更しない契約を維持し、追従は
`cargo update … --precise` による明示操作とする。

## Components and Boundaries

### §1 manifest 契約

- `Cargo.toml` の ruviform-* 依存は `git = "<url>"` と必要な
  `features` のみを持つ。`rev`/`branch`/`tag` は書かない。
- Cargo の source id は `git+<url>` で統一され、同一 URL の
  全 ruviform crate は常に同一 resolved revision を共有する
  (cargo が同一 git source を 1 checkout で解決するため)。
- 上記は `cargo update -p <ruviform-pkg> --precise <sha>` の
  実験でも確認済み: precise 指定は source 単位に作用し、
  対象 SHA に存在しない crate がある場合は lockfile を変更
  せず fail-closed で失敗する。

### §2 lockfile 取得・検証契約(`tools/ruviform-lock-rev.mjs`)

resolved revision の取得と検証を担う**唯一の正本モジュール**。
CLI と import 可能な関数を同一ファイルで提供し、
`sync-ruviform-ui.mjs`・`update-ruviform-rev.mjs`・mise task
`check-ruviform-lock` がこれを共有する。

- **入力**: `<repo>/Cargo.lock`。`<repo>/Cargo.toml` は
  ruviform Git URL 集合の取得に使う。
- **パース契約**: 任意文字列 grep に依存しない。
  `Cargo.lock` を行構造として解釈する:`#` コメント行と
  `version = N` を除き、`[[package]]` セクション単位で
  `name`/`source` の `key = "…"` 単一行スカラーを取得する。
  `name`/`source` が単一行クォート形式でない・`[[package]]`
  内に `name` が無い等の未知形式は fail-closed。
- **ruviform crate の識別**: manifest の ruviform-* entry から
  `git = "<url>"` を収集し URL を正規化した集合と、各
  `[[package]].source` の git URL を正規化した値とで照合する。
  これにより manifest 未記載の推移 crate(例 `ruviform-rt`)も
  同一 URL 由来として検出できる。
- **URL 正規化**: `git+` 接頭辞除去 → scheme/authority 正規化
  (`https://`/`http://`/`ssh://git@`/`git@host:` を
  `host/path` へ) → 末尾 `/`・`.git` サフィックス除去 →
  小文字化。
- **revision 抽出**: git source 表記
  `git+<url>[?<query>]#<fragment>` を解釈する。`fragment` が
  40 桁小文字 hex でない、`#` 自体が無い、`git+` で始まら
  ないものは形式不明として fail-closed。`?rev=`/`?branch=`/
  `?tag=` 等の query は検査対象のまま残し、resolved revision
  は常に `#fragment` とする(manifest 指定子は正本ではない)。
- **検証規則**(すべて非ゼロ終了で fail-closed):
  1. ruviform 由来 package が 0 件 → ロック欠落として失敗。
  2. resolved fragment SHA が複数 → 分裂として失敗(全値を報告)。
  3. 正規化後の URL が manifest 集合に無い → 未知 source として失敗。
  4. `Cargo.lock`/`Cargo.toml` が読めない・形式不明 → 失敗。
- **出力**: 既定で解決 SHA と package 数を 1 行で表示。
  `--json` で `{rev, url, packages[]}` を機械読み取り可能に出力。

### §3 UI vendor 同期(`tools/sync-ruviform-ui.mjs`)

- 正本を manifest `rev` から本契約へ移す。`readPin()` は
  `ruviform-lock-rev.mjs` の `{rev, url}` を使い、URL は
  lockfile の source id 由来(= git archive の fetch URL)。
- `ui/vendor/ruviform/REV` に解決 SHA を記録する契約は維持。
  `--check` は「lockfile resolved SHA == REV && 4 package の
  `src/index.ts` 存在」を検証する。vendor tree は gitignore 済み
  生成物で、手編集せず `sync-ui-deps` で再生成する。
- 初回 checkout/worktree では `mise run sync-ui-deps` が
  lockfile SHA で `git fetch --depth 1 <sha>` + `git archive`
  を再現実行するため、取得 revision と依存 revision は常に一致する。

### §4 検証入口(mise task / CI)

- `[tasks.check-ruviform-lock]`: `node tools/ruviform-lock-rev.mjs`。
  lockfile 正本と単一 SHA を検証する軽量 check(3 OS 共通、
  Rust ビルド不要)。
- `check-ui`/`test-ui` は既存どおり先頭で
  `sync-ruviform-ui.mjs --check` を走らせ、UI vendor REV と
  lockfile SHA の不一致を fail-closed で止める。
- `ci.yml` に step 追加:`cargo fetch --locked` 直後に
  `mise run check-ruviform-lock`(resolved SHA を step log に
  記録=provenance)、UI lane として `setup-ui` → `sync-ui-deps`
  → `check-ui` → `test-ui` を追加し、依存解決 revision と
  UI 取得 revision の一致を CI で検証する。
- `cargo tree`/`cargo metadata` は人手の確認手段として併記する
  (機械契約は本ツールが担う)。

### §5 追従更新(明示操作)

- **更新コマンド**: `node tools/update-ruviform-rev.mjs <sha>`。
  40 桁 hex を検証後、`cargo update -p ruviform-media-core
  --precise <sha>` を実行し(§1 の通り source 単位で原子更新)、
  続けて本契約で新旧 SHA を報告する。candidate SHA の差し替えは
  この 1 操作で完結し、manifest 編集は不要。
- **更新 PR の流れ**: scratch checkout/branch で上記を実行 →
  lockfile diff + `verify-ci` + UI lane + Golden(f64/入力時刻/
  分析決定/Stage resume/Export)+ 性能差分を記録 → 承認済み
  PR のみ main へ。無検証の自動 main 追従は行わない。
- **Ruviform #219/#294 との整合**: consumer-verify が candidate
  SHA を注入する経路は「manifest rev 書換え」と「lockfile
  `--precise` 更新」の双方をサポートできる。本移行後は後者が
  正規経路であり、manifest に rev が無くても差し替えが動くこと
  を本 Issue で確認する。

### §6 rollback / provenance / error

- **rollback**: `git checkout -- Cargo.lock`(または旧 SHA で
  §5 を再実行)で直前の resolved SHA に即時復帰できる。
  manifest 側は SHA を持たないため対応不要。UI vendor は
  `sync-ui-deps` で lockfile に追随して再生成する。
- **provenance**: 実行時の ruviform revision は
  `node tools/ruviform-lock-rev.mjs` がいつでも lockfile から
  復元でき、CI では §4 の step log に出力される。解析 artifact
  の評価記録(IR/Golden 差分報告)では、このコマンド出力の
  SHA と `ui/vendor/ruviform/REV` を併記して provenance を
  追跡する(dataset schema の provenance 項目は対象外 —
  本件は依存解決の provenance)。
- **error 分類**: lockfile 未 commit/欠落、SHA 分裂、未知
  source 形式、manifest URL 集合の空、credential 失敗
  (cargo/git fetch 層)、vendor 未同期 — 各々が fail-closed で
  区別可能なエラーを出す。credential 層の失敗は従来どおり
  `git ls-remote` probe と cargo fetch の失敗として表面化する。

## Data and Control Flow

```text
Cargo.toml (git URL + features のみ、rev なし)
  -> cargo resolve
  -> Cargo.lock [[package]].source = "git+<url>#<sha>"   ★正本
       ├─ tools/ruviform-lock-rev.mjs   (取得・単一SHA検証)
       │    ├─ mise run check-ruviform-lock / ci.yml step
       │    ├─ tools/sync-ruviform-ui.mjs (REV と整合)
       │    └─ tools/update-ruviform-rev.mjs (precise 更新)
       └─ cargo fetch --locked (解決 SHA の取得・再現性)
```

## Risks and Trade-offs

- **manifest rev なしで `cargo update` を誤実行**: 通常の
  `cargo update -w` 実行は依存全体を浮動させ得るが、lockfile
  diff として可視であり `check-ruviform-lock` が分裂/逸脱を
  止める。自動追従しない契約は変わらない。
- **lockfile の手編集事故**: cargo が書き換える形式への
  手編集は不可避に許容されるファイルではないが、`#fragment`
  のみを検査する本契約は手編集混入でも fail-closed で検出する。
- **`cargo update --precise` の source 単位適用**: 単一 crate の
  み別 SHA へ移すことは cargo が許さないため、意図しない部分
  更新は起きない(§1 実験済み)。逆に「一部だけ古い SHA」の
  状態は lockfile 手編集でのみ発生し得るが、分裂として検出する。
- **新旧契約の共存期間**: PR マージ前後で manifest 記述と
  実コードの齟齬が生じ得るため、記述更新は実装と同一 PR で行う。

## Deferred Design Concerns

- main 自動追従 bot / dependabot 相当の導入(現行どおり不採用)。
- Synvelis 側との resolved revision 一致検証の機械化
  (契約は揃うが、同一 SHA 強制は別 Issue)。
- 更新差分レポート(Golden/性能)の自動生成テンプレート。

## Verification Notes

- `cargo fetch --locked` が manifest rev 撤去後の lockfile を
  変更せず成功すること。
- `node tools/ruviform-lock-rev.mjs` が `073a3ff…` を報告し、
  Cargo.lock の SHA を片方だけ改変した場合に分裂検出で
  失敗すること。
- `mise run sync-ui-deps` → `check-ui` → `test-ui` が通り、
  `ui/vendor/ruviform/REV` == lockfile SHA。
- `mise run verify-ci`(`fmt`→`check-ci`→`lint-ci`→`test-ci`)
  が PASS。本移行は同一 SHA での正本移し替えであり、
  Golden/性能差分は発生しないことを test 済みで確認する。
- `node tools/update-ruviform-rev.mjs <sha>` が lockfile を
  candidate SHA へ原子更新し、旧 SHA への `git checkout` で
  復帰できること。

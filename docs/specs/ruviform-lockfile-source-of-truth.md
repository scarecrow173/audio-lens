---
id: "SPEC-o0wlzeWCSUPIniAFtwqSCc"
type: "spec"
status: "approved"
title: "Ruviform Git 依存の Cargo.lock 正本化"
created: "2026-11-04"
updated: "2026-11-04"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/audio-lens/issues/140"
    - "https://github.com/scarecrow173/audio-lens/issues/127"
    - "https://github.com/scarecrow173/ruviform/issues/219"
    - "https://github.com/scarecrow173/ruviform/issues/294"
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
    - "../designs/ruviform-dependency-ci.md"
  refines: []
  refined-by: []
  derives-from: []
  derived-by: []
  verifies: []
  verified-by: []
  references:
    - "https://github.com/scarecrow173/audio-lens/issues/99"
  defers: []
  deferred-by: []
---

# Ruviform Git 依存の Cargo.lock 正本化

## What Should Be Built

`Cargo.toml` `[workspace.dependencies]` に列挙された Ruviform
Git 依存の固定 `rev` を撤去し、commit `Cargo.lock` の resolved
Git revision を唯一の正本とする依存管理契約へ移行する。
Issue [#140](https://github.com/scarecrow173/audio-lens/issues/140)
の Acceptance Gates が正本である。

具体的には次を構築する。

1. `Cargo.toml` の Ruviform Git 依存から `rev` を除去し、
   `Cargo.lock` を commit したまま `cargo build/test --locked`
   が lockfile を更新しない契約にする。
2. `Cargo.lock` の `[[package]].source` から Ruviform resolved
   revision を取得・検証する単一の契約(fail-closed)。
3. `tools/sync-ruviform-ui.mjs` の同期元を Cargo.lock へ移し、
   `ui/vendor/ruviform/REV` との整合を検証する。
4. 「manifest 固定 rev 必須」を前提にした記述・スクリプト・
   CI を更新し、依存解決 revision と UI TypeScript 取得
   revision の一致を CI で検証する。
5. candidate SHA で lockfile を明示更新する追従操作と、
   旧 SHA への rollback・provenance 追跡手順を定義する。

## Why Now

- [#127](https://github.com/scarecrow173/audio-lens/issues/127)
  の継続追従は manifest の `rev` 14 件一括更新で運用されてきたが、
  同一 SHA の列挙は manifest 責務と lockfile 責務の重複であり、
  Ruviform #219/#294 の consumer candidate 差し替えは lockfile
  を正本とする方が自然に整合する。
- Synvelis は manifest に固定 `rev` を持たず lockfile 解決で
  追従済みであり、consumer 間の契約を揃える。
- `sync-ruviform-ui.mjs` が manifest `rev` を必須としており、
  `rev` を単純撤去すると UI 同期が壊れるため、正本の移行と
  取得契約の単一化を同時に行う必要がある。

## Users and Value

- 継続追従の実施者: lockfile だけを更新対象にすればよく、
  manifest 列挙の更新ミスによる SHA 分裂を構造的に排除できる。
- CI / 検証ツール: 単一の取得・検証契約から resolved revision
  と UI vendor revision の一致を機械的に確認できる。
- レビュアー: 更新 PR の差分が Cargo.lock の source fragment に
  限定され、意図しない manifest 変更と混ざらない。

## Scope

### In Scope

- `Cargo.toml` Ruviform 直接 Git 依存(現行 14 entry)からの
  `rev` 除去。`git` URL・`features`・クレート構成は保持。
- `Cargo.lock` 正本の検証・取得契約(`tools/ruviform-lock-rev.mjs`)
  と、それを消費する `sync-ruviform-ui.mjs`・mise task・CI。
- lockfile 明示更新の追従操作(`tools/update-ruviform-rev.mjs`)
  と rollback・provenance・失敗時復旧の手順文書化。
- `docs/designs/ruviform-dependency-ci.md`・関連ガイド・
  コメント類の契約記述更新。

### Out of Scope

- `Cargo.lock` 自体の廃止、依存の無検証な浮動化、
  `main` への自動追従 bot(dependabot 等)の導入。
- Synvelis への別 revision 強制、Ruviform 側への製品固有
  要求の追加。
- 将来の Ruviform 共通 I/O / Observability 導入そのもの。
- `cargo vendor` / vendored Rust ソースの導入(従来どおり不採用)。

## Acceptance Criteria

- fresh checkout で `cargo fetch --locked` / `mise run check-ci` /
  `mise run test-ci` が単一 resolved SHA のまま成功し、
  manifest に固定 `rev` が存在しない。
- 取得・検証契約が全 Ruviform crate(推移依存を含む)の単一
  revision を fail-closed で検証する。ロック欠落・複数 rev・
  未知形式・未同期 UI はすべて非ゼロ終了。
- `sync-ui-deps` → `check-ui` → `test-ui` と既存
  GUI/headless/CLI 境界が通り、`ui/vendor/ruviform/REV` が
  lockfile resolved SHA と一致する。
- Ruviform 更新前後の Golden(f64/入力時刻/音声・分析決定/
  Stage resume/Export)と性能差分の評価手順が定義され、
  本移行(同一 SHA での正本移し替え)では差分が発生しない
  ことを確認する。
- 適用可能な OS・開発フロー・private/public アクセス・
  更新 PR の再現性が検証される。未検証項目は明示する。
- Ruviform #219、AudioLens #127/#99 との整合が文書化され、
  導入手順と失敗時の復旧がガイド/設計に記録される。

## Constraints

- `rev` 除去後も URL は manifest に残るため、取得契約は URL /
  host を正規化して ruviform crate を識別する。
- 任意文字列 grep に依存しない(Cargo.lock の `[[package]]`
  構造と git source 表記 `git+…[?query]#<sha>` の構造化
  解釈に基づく)。
- 通常の `cargo build/test --locked`・`cargo fetch --locked`
  は lockfile を変更しないこと。

## Non-Goals

- Ruviform の更新内容そのものの評価(各更新 PR の責務)。
- `Cargo.lock` を human-editable にする仕組みの構築。

採用設計は [design](../designs/ruviform-lockfile-source-of-truth.md)
を参照する。

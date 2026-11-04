---
id: "TSPEC-j4tOtpIhG5QAglrGV0BZjb"
type: "test-spec"
status: "proposed"
title: "Ruviform Cargo.lock 正本化の検証"
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
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related: []
  refines: []
  refined-by: []
  derives-from:
    - "../designs/ruviform-lockfile-source-of-truth.md"
    - "../plans/ruviform-lockfile-source-of-truth.md"
  derived-by: []
  verifies:
    - "../designs/ruviform-lockfile-source-of-truth.md"
    - "../specs/ruviform-lockfile-source-of-truth.md"
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# Ruviform Cargo.lock 正本化の検証

## Purpose

Issue #140 の依存管理契約が、「manifest に固定 rev が存在
しない」「Cargo.lock が唯一の正本」「検証・取得契約が
fail-closed」「UI vendor が lockfile revision と一致」ことを
保証する。検証対象は
[設計](../designs/ruviform-lockfile-source-of-truth.md) /
[計画](../plans/ruviform-lockfile-source-of-truth.md) の契約。

## Feature

Cargo.lock 正本の resolved revision 契約と消費者整合

## Rules

- Rule: `Cargo.toml` の ruviform-* 依存は `git`/`features` のみを
  持ち、`rev`/`branch`/`tag` を含まない。
- Rule: `Cargo.lock` の同 URL 由来 ruviform package(推移依存
  含む)は `source = "git+<url>#<sha>"` で、全て同一の 40 桁
  hex fragment を持つ。
- Rule: `tools/ruviform-lock-rev.mjs` は ruviform 由来 package
  0 件・SHA 分裂・未知 source 形式・manifest URL 集合空で
  非ゼロ終了(fail-closed)する。
- Rule: `cargo fetch --locked` / `cargo metadata --locked` /
  `cargo tree --locked` は lockfile を変更しない。
- Rule: `tools/sync-ruviform-ui.mjs --check` は
  `ui/vendor/ruviform/REV` が lockfile SHA と不一致・vendor
  未完成の場合に非ゼロ終了する。
- Rule: `tools/update-ruviform-rev.mjs <sha>` は candidate SHA
  へ lockfile を原子更新し、非 40 桁引数・対象 SHA に存在
  しない crate 構成では失敗する。
- Rule: `git checkout -- Cargo.lock` で旧 SHA に復帰できる。

## Examples

- `node tools/ruviform-lock-rev.mjs` → 単一 SHA と package 数を
  報告(例: `073a3ff…`, 15 packages)。
- lockfile の 1 package だけ fragment を改変 → 契約ツールが
  分裂値を列挙して失敗。
- `mise run check-ui` が REV==lockfile SHA で通り、vendor を
  消して `--check` すると失敗、`sync-ui-deps` で復旧。
- `update-ruviform-rev.mjs <存在しないcrateを含むSHA>` →
  cargo が fail-closed で lockfile 不変。

## Coverage

- 上記 Rule は `check-ruviform-lock` / `check-ui` / `test-ui` /
  `verify-ci` の自動検証と、更新・rollback 手順の手動検証で
  カバーする。Rust 側の単体テスト追加は不要(契約は lockfile
  と生成物の検証であり、解析ロジックを変更しない)。

---
id: "PLAN-4ifedHAkxHtbglJWu8ESIL"
type: "plan"
status: "approved"
title: "Ruviform Git 依存を Cargo.lock 正本へ移行する実装計画"
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
    - "../designs/ruviform-lockfile-source-of-truth.md"
  implemented-by:
    - "../tasks/ruviform-lockfile-source-of-truth.md"
  depends-on:
    - "../designs/ruviform-lockfile-source-of-truth.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "../designs/ruviform-dependency-ci.md"
  refines: []
  refined-by: []
  derives-from:
    - "../designs/ruviform-lockfile-source-of-truth.md"
  derived-by: []
  verifies: []
  verified-by:
    - "../test-specs/ruviform-lockfile-source-of-truth.md"
  references: []
  defers: []
  deferred-by: []
---

# Ruviform Git 依存を Cargo.lock 正本へ移行する実装計画

## Goal

[設計](../designs/ruviform-lockfile-source-of-truth.md) の契約を、
Issue [#140](https://github.com/scarecrow173/audio-lens/issues/140)
の作業単位 1〜6 として実装する。同一 SHA(`073a3ff…`)のまま
正本を manifest `rev` から `Cargo.lock` fragment へ移し替え、
Ruviform 側コードには一切触れない(解析差分ゼロが前提)。

## Scope

- manifest `rev` 除去 + lockfile `?rev=` query 除去(同一 SHA 保持)
- 単一取得・検証契約 `tools/ruviform-lock-rev.mjs` 新設
- `tools/sync-ruviform-ui.mjs` の正本移行
- `tools/update-ruviform-rev.mjs` による明示追従操作
- `mise.toml` task 追加、`ci.yml` に lock/UI 検証 step 追加
- `docs/designs/ruviform-dependency-ci.md`・ガイド・コメントの
  契約記述更新、rollback/provenance 手順の文書化

## File Map

| Path | Change | Purpose |
| --- | --- | --- |
| `Cargo.toml` | modify | 14 entry から `rev` 除去、規約コメント更新 |
| `Cargo.lock` | modify | `?rev=<sha>` query 除去(`#<sha>` 保持) |
| `tools/ruviform-lock-rev.mjs` | add | lockfile resolved rev の取得・検証契約 |
| `tools/update-ruviform-rev.mjs` | add | `--precise` 明示更新 + 前後検証 |
| `tools/sync-ruviform-ui.mjs` | modify | 正本を lockfile 契約へ移行 |
| `mise.toml` | modify | `check-ruviform-lock` task + コメント更新 |
| `.github/workflows/ci.yml` | modify | lock 検証 step + UI lane step 追加 |
| `.worktreeinclude` | modify | vendor 再生成元の記述更新 |
| `docs/designs/ruviform-dependency-ci.md` | modify | §2 契約・検証手順を lockfile 正本へ改訂 |
| `docs/guides/ruviform-private-access-ci.md` | modify | `git+rev` 記述・復旧手順の更新 |

## Task Breakdown

1. [tasks/ruviform-lockfile-source-of-truth.md](../tasks/ruviform-lockfile-source-of-truth.md) — 上記全変更を
   1 タスクとして実施(変更は相互依存のため分割しない)。

## Dependency Notes

- `rev` 除去と `sync-ruviform-ui.mjs` 契約移行は同一コミット
  境界で行う(片方だけでは UI 同期が壊れる)。
- `Cargo.lock` の `?rev=` 除去は manifest 変更と同時に行い、
  `cargo fetch --locked` が lockfile を書き換えないことを確認。

## Risk Register

| Risk | Mitigation |
| --- | --- |
| `rev` 除去後に cargo が lockfile を別 SHA に再解決 | lockfile は `#<sha>` fragment を持つため `--locked` で不変。実機で `cargo fetch --locked` の無差分を確認済み |
| sync ツールが正本遷移の隙間で失敗 | 契約モジュール導入と manifest 変更を同コミット化 |
| candidate `--precise` 更新の部分適用 | cargo が source 単位で原子更新(実証済み)+ 事後に契約検証 |
| CI workflow 変更と #142 の競合 | 追加 step を独立ブロックに留め、coordinator の merge に委譲 |

## Rollback

- コード側: `git checkout -- Cargo.lock` で旧 SHA へ復帰、
  `sync-ui-deps` で vendor 再生成。
- 本移行自体の rollback: ブランチ revert で manifest `rev`
  列挙へ戻す(移行前後で lockfile 解決 SHA は同一)。

## Verification Matrix

| Acceptance Gate | 検証手段 |
| --- | --- |
| fresh checkout で `--locked` 系が単一 SHA で成功 | `cargo fetch --locked` / `mise run check-ci` / `test-ci` |
| 全 ruviform crate の単一 revision | `check-ruviform-lock` + `cargo metadata`/`tree` 併記 |
| ロック欠落・複数 rev・未知形式・未同期 UI が fail-closed | 契約ツールの異常系検証(手動注入)+ `--check` |
| UI lane | `sync-ui-deps`→`check-ui`→`test-ui`、REV==lockfile SHA |
| Golden/性能差分評価 | 同一 SHA 移行のため差分なしを `test-ci` で確認、手順を文書化 |
| OS/アクセス/更新 PR 再現性 | 本機 Windows で検証、他 OS は CI matrix 依存・未検証と明記 |
| #219/#294 整合・導入/復旧手順 | 設計・ガイドへの記述 + `--precise` 注入の実機確認 |

判定基準の詳細は [test-spec](../test-specs/ruviform-lockfile-source-of-truth.md) を参照する。

---
id: "IMPL-xiM69SgT2jizDTQoeb"
type: "impl"
status: "completed"
title: "ruviform lockfile source of truth"
created: "2026-11-04"
updated: "2026-11-04"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/audio-lens/issues/140"
  changes:
    added:
      - type: "code"
        path: "tools/ruviform-lock-rev.mjs"
      - type: "code"
        path: "tools/update-ruviform-rev.mjs"
      - type: "doc"
        path: "docs/specs/ruviform-lockfile-source-of-truth.md"
      - type: "doc"
        path: "docs/designs/ruviform-lockfile-source-of-truth.md"
      - type: "doc"
        path: "docs/plans/ruviform-lockfile-source-of-truth.md"
      - type: "doc"
        path: "docs/tasks/ruviform-lockfile-source-of-truth.md"
      - type: "doc"
        path: "docs/test-specs/ruviform-lockfile-source-of-truth.md"
      - type: "doc"
        path: "docs/guides/ruviform-lockfile-update.md"
    modified:
      - type: "manifest"
        path: "Cargo.toml"
      - type: "lockfile"
        path: "Cargo.lock"
      - type: "code"
        path: "tools/sync-ruviform-ui.mjs"
      - type: "manifest"
        path: "mise.toml"
      - type: "code"
        path: ".github/workflows/ci.yml"
      - type: "code"
        path: ".worktreeinclude"
      - type: "doc"
        path: "docs/designs/ruviform-dependency-ci.md"
      - type: "doc"
        path: "docs/guides/ruviform-private-access-ci.md"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/tasks/ruviform-lockfile-source-of-truth.md"
  implemented-by: []
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related: []
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

# ruviform lockfile source of truth

## Summary

Ruviform Git 依存の resolved revision 正本を `Cargo.toml` の
固定 `rev` 列挙から `Cargo.lock` の `source` fragment へ移行した。
resolved SHA `073a3ff698c5bf9002fe21c7547b04917601e058` は不変で、
manifest から `rev` を除去しただけのため解析結果・Golden に
差分は生じない。

## Context

task `docs/tasks/ruviform-lockfile-source-of-truth.md`、
design `docs/designs/ruviform-lockfile-source-of-truth.md`、
test-spec `docs/test-specs/ruviform-lockfile-source-of-truth.md`
に従う(Issue #140、AL#99/#127/#131/#137、Ruviform #219 と整合)。

## Implementation

### Changes

- `Cargo.toml`: 14 件の Ruviform Git 依存から `rev` を除去
  (URL と `ruviform-io-symphonia` の `features = ["mp4","aac"]`
  は保持)、正本が Cargo.lock である旨の規約コメントに更新
- `Cargo.lock`: `source` の冗長な `?rev=<sha>` query を除去し
  `git+<url>#<sha>` に正規化(`#<sha>` fragment は保持)
- `tools/ruviform-lock-rev.mjs`(新規): lockfile の
  `[[package]].source` を構造的に解釈する単一の取得・検証契約。
  URL/host 正規化、`#<sha>`(40 桁 hex)fragment 抽出、同一 URL
  由来の全 ruviform crate が単一 SHA であることの fail-closed
  検証。ゼロ件・分裂・未知形式・混在 URL はすべて失敗
- `tools/sync-ruviform-ui.mjs`: 正本を manifest `rev` から
  契約ツールへ移行し、`ui/vendor/ruviform/REV` を lockfile SHA
  と照合する `--check` を維持
- `tools/update-ruviform-rev.mjs`(新規): candidate SHA を
  `cargo update --precise` で lockfile へ明示適用し、適用前後で
  契約検証を行う追従操作
- `mise.toml`: `check-ruviform-lock` task 追加、UI lane コメントを
  lockfile 正本に更新
- `ci.yml`: fetch 後に lock 検証 step、末尾に UI lane
  (setup-ui→sync-ui-deps→check-ui→test-ui)を追加し、依存解決
  revision と UI TypeScript 取得 revision の一致を CI で検証
- `docs/designs/ruviform-dependency-ci.md` と
  `docs/guides/ruviform-private-access-ci.md` を lockfile 正本
  契約へ改訂、`docs/guides/ruviform-lockfile-update.md` に
  更新・rollback 手順を新規作成

### Decisions During Implementation

- `?rev=` query の除去は `cargo fetch --locked` が lockfile を
  書き換えないことを実機で確認してから適用
- 更新ツールは scratch ではなく workspace で `--precise` を実行し
  契約ツールで事後検証する構成とした(cargo が git source 単位で
  原子更新することを実証済み。`ruviform-viz` を含まない旧 SHA への
  更新は lockfile 無変更のまま失敗することも確認)
- `ruviform-*` package が別 URL から解決されるケースは package
  名シードでは検出しない設計とし、未知 source 混入を fail-closed
  にした

## Validation

実施済み(本機 Windows):

- `cargo fetch --locked`: lockfile 無差分で成功
- `mise run verify-ci`(fmt + check-ci + lint-ci + test-ci):
  全 pass(Golden 12 件含む、0 failed)
- `mise run check-ruviform-lock`: 15 packages が単一 SHA
  `073a3ff…` で一致
- `mise run check-cli-headless`: GUI framework 依存なしを確認
- `mise run sync-ui-deps` → `check-ui` → `test-ui`: 全 pass
  (tsc 0 エラー、node:test 112 件全 pass)、
  `ui/vendor/ruviform/REV` == lockfile SHA
- 契約ツール異常系(ゼロ件・SHA 分裂・未知形式・別 URL 混入・
  非 40 桁 fragment): すべて fail-closed を手動注入で確認
- `update-ruviform-rev.mjs` 往復: `e9e6abe…` で全 15 package が
  一括更新されること、旧 SHA への rollback で `073a3ff…` に
  復帰することを確認
- `mise exec -- prek run --all-files`: 全 hook pass
- Ruviform #219 consumer-verify は manifest の rev あり/なし
  (unspecified-selector)双方を既に正規化対応済みであることを
  コード確認

未検証:

- macOS / Linux の実機検証(CI matrix に委譲、workflow 未 dispatch)
- GUI 実ウィンドウ動作(headless 境界のみ検証)
- 性能差分の定量評価(同一 SHA のため理論上差分なし。
  将来の SHA 更新時は更新 PR で測定する手順をガイドに記載)
- private repo の CI deploy key 経路(secret 未設定環境では
  dispatch できず。手順はガイドに記載済み)

## Follow-ups

- 次回 Ruviform 追従時は `update-ruviform-rev.mjs <sha>` で
  lockfile を更新し、差分と回帰結果を PR に記録する
- doc グラフ全体の pre-existing blocker(discovery 系の壊れた
  source relation 等)は本 Issue の範囲外として別途対応が必要

## Related Experiments

### Adopted

- なし

### Rejected

- なし

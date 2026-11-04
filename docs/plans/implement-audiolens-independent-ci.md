---
id: "PLAN-dmszyxRb1oaST3thMlruEm"
type: "plan"
status: "approved"
title: "AudioLens 独立 CI(#142)の実装計画"
created: "2026-11-04"
updated: "2026-11-04"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/audio-lens/issues/142"
  changes:
    added: []
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "../specs/audiolens-independent-ci.md"
    - "../designs/audiolens-independent-ci.md"
  implemented-by:
    - "../tasks/audiolens-independent-ci-issue-142.md"
  depends-on:
    - "../designs/ruviform-dependency-ci.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "https://github.com/scarecrow173/audio-lens/issues/140"
    - "https://github.com/scarecrow173/ruviform/issues/294"
  refines: []
  refined-by: []
  derives-from:
    - "../specs/audiolens-independent-ci.md"
    - "../designs/audiolens-independent-ci.md"
  derived-by: []
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
  test-spec-skip: "CI/workflow と補助スクリプトの変更であり test-spec
    対象のプロダクト挙動変更なし。検証は actionlint・
    check-ruviform-rev の正常/異常系ローカル実行・各 mise task で
    実施し IR に記録する"
metadata:
  experiments:
    adopted: []
    rejected: []
---

# AudioLens 独立 CI(#142)の実装計画

## Goal

[design](../designs/audiolens-independent-ci.md) /
[spec](../specs/audiolens-independent-ci.md) のとおり、dispatch 専用
`ci.yml` を PR/Push/Manual で起動する独立 CI へ拡張し、dual-mode
依存アクセス・scoped mise・lane 分離・resolved SHA ガード・証拠
レポートを実装する。実行単位は
[task: audiolens-independent-ci-issue-142](../tasks/audiolens-independent-ci-issue-142.md)、
実績記録は
[impl: audiolens-independent-ci-issue-142](../impl/ir/audiolens-independent-ci-issue-142.md)。

## Scope

単一ストリーム。文書(spec/design/plan/task)+ ツール + workflow +
ガイド更新。既存 task 意味の変更なし。

## File Map

| ファイル | 変更 |
| --- | --- |
| `.github/workflows/ci.yml` | 全面再構成(trigger/access-gate/lane/summary) |
| `tools/check-ruviform-rev.mjs` | 新規: resolved SHA 単一性 + REV 整合ガード |
| `mise.toml` | `[tasks.check-ruviform-rev]` 追加 |
| `mise.lock` | `mise lock` で actionlint の 7 platform entry 補完 |
| `docs/guides/ruviform-private-access-ci.md` | dual-mode 化の追従 |
| `docs/designs/ruviform-dependency-ci.md` | §10/Deferred の追従注記 |
| `docs/specs` `docs/designs` `docs/plans` `docs/tasks` `docs/impl/ir` + 各 `README.md` | 文書チェーンと索引 |

## Task Breakdown

1. 文書チェーン作成(spec/design/plan/task)+ 索引再生成
2. `tools/check-ruviform-rev.mjs` + `mise.toml` task 追加
3. `mise lock` による actionlint platform entry 補完
4. `.github/workflows/ci.yml` 再構成
5. ガイド・旧設計書の追従更新
6. ローカル検証(actionlint / biome / スクリプト実行 / prek)
7. IR 記録 + commit(文書/ツール/workflow で論理分割)

## Verification Matrix

| 検証 | 方法 | 完了条件 |
| --- | --- | --- |
| workflow 静的検証 | `mise exec -- actionlint .github/workflows/ci.yml` | pass |
| lockfile gap 補完 | `git diff mise.lock` で actionlint のみ entry 追加 | pass |
| SHA ガード正常系 | `node tools/check-ruviform-rev.mjs`(現 repo) | exit 0 + SHA 出力 |
| SHA ガード fail 系 | 一時的な lock/REV 改変で非ゼロ exit | 各 fail 経路が検出 |
| vendor 整合 | `node tools/check-ruviform-rev.mjs --require-vendor` | REV == lock SHA |
| 既存 task 非破壊 | `mise run fmt` / `check` / `check-ui` | pass |
| hygiene | `MISE_ENABLE_TOOLS=... mise exec -- prek run --all-files` | pass |
| 実 workflow 実行 | merge 後の初回 PR/run で確認(本 PR では未検証) | 未検証として記録 |

## Risks / Rollback

- workflow は merge まで e2e 検証できない(旧設計 §10 と同制約)
  → actionlint + 各 step のローカル再現で軽減
- #140 が先に merge された場合、manifest rev 撤去後も
  check-ruviform-rev は lockfile 側のみで成立する設計
- rollback: `git revert` で ci.yml を旧 dispatch-only 版へ戻せる
  (新規ファイル中心の変更)

## Dependencies

- Ruviform `consumer-verify.yml` の convention(access mode 解決・
  証拠表・scoped mise)
- `Cargo.lock` の ruviform source が `git+...#<sha>` 形式であること
  (rev pin・lockfile 主体の両形式で `#<sha>` fragment は共通)

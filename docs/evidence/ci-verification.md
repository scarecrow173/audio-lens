---
id: "EVID-sK4nPvXzQyLmWjRdTbGc"
type: "evidence"
status: "approved"
title: "検証記録: CI・ツールチェーン・Ruviform アクセス"
created: "2026-11-19"
updated: "2026-11-19"
owners: []
relations:
  source: []
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
  derives-from: []
  derived-by: []
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# 検証記録: CI・ツールチェーン・Ruviform アクセス

CI・ツールチェーン・Ruviform 依存解決に関する検証の事実記録。

## AL#99 Ruviform Git 依存・toolchain・private CI

- `cargo fetch` 成功(GCM 経由、9 entry 同一 rev `73190d7d…`)。
- `verify-toolchain`(rustc/cargo 1.99.0 pin 一致)、`fmt`/`check`/`lint`/`test`/`build`/`verify-ci`/`build-ci`/`check-cli-headless` 全 pass。
- actionlint clean、`prek run --all-files` 全 hook pass、`git ls-remote` 到達確認済み。
- smoke test `tests/ruviform_deps.rs` 4 crate は基礎型構築のみ(ruviform-dsp/audio-analysis は `version()` を公開しない)。
- **未実施**: ci.yml の e2e(workflow_dispatch は default branch merge 後のみ)、Windows runner の ssh 解決・鍵 permission。

## AL#140 lockfile 正本化

- `cargo fetch --locked` 無差分で成功、`mise run verify-ci`(fmt+check-ci+lint-ci+test-ci)全 pass(Golden 12 件・0 failed)。
- `mise run check-ruviform-lock`: 15 packages が単一 SHA `073a3ff…` で一致。
- `sync-ui-deps`→`check-ui`→`test-ui` 全 pass(tsc 0 エラー・node:test 112 件・`ui/vendor/ruviform/REV`==lockfile SHA)。
- 契約ツール異常系(ゼロ件・SHA 分裂・未知形式・別 URL 混入・非 40 桁 fragment)全 fail-closed を手動注入で確認。
- `update-ruviform-rev.mjs` 往復で `e9e6abe…` 全 15 package 一括更新→rollback で `073a3ff…` 復帰。
- **未検証**: macOS/Linux 実機(CI matrix 依存)、GUI 実ウィンドウ、private CI deploy key 経路(secret 未設定環境で dispatch 不可)。

## AL#141 Ruviform アクセス CI

- preflight でアクセスモード解決、`RUVIFORM_DEPLOY_KEY` 未設定時に認証 step で明示失敗(fail-closed)をローカル fixture で確認。
- 実 workflow の deploy-key 経路・3 OS 実機検証は未実施(secret 未設定環境で dispatch 不可)。

## AL#142 独立 CI

- actionlint・workflow 構造チェック・prek・`fmt`/`check`/`lint`/`test`-ci・CLI build・headless 依存検査・UI lane(setup-ui→sync-ui-deps→check-ui→test-ui)のローカル再現を実施。
- 実 workflow 実行(dispatch)は未実施。

## AL#143 #142 merge 後修正

- merge 後の CI lane 動作を検証(具体的な修正内容は extraction で未取得、AL#142 merge 後の修正として記録)。

## 未検証・制約

- 実 workflow 実行・deploy-key 経路・3 OS matrix の実機検証は当時全体を通して未実施。
- paths filter 不採用は意図的(prek 全域対応)だが、ドキュメントのみ変更でも CI が全 lane 起動する点は既知のトレードオフ。

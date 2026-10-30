---
id: "IMPL-034P70XgpUzok2HsL8bOrX"
type: "impl"
status: "completed"
title: "Ruviform Git 依存 SHA 固定・toolchain 検証・Private repo CI 連携"
created: "2026-10-30"
updated: "2026-10-30"
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
  implements:
    - "docs/tasks/ruviform-dependency-ci.md"
  implemented-by: []
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/designs/ruviform-dependency-ci.md"
    - "docs/guides/ruviform-private-access-ci.md"
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

# Ruviform Git 依存 SHA 固定・toolchain 検証・Private repo CI 連携

## Summary

AL#95 移行の依存基盤として、Ruviform の 8 crate を検証済みの
同一 commit SHA(`73190d7d26b3197b1da36749798b5952499d1b70`)の
git 依存で導入し、private fetch・toolchain 検証・headless 境界
guard・dispatch-only CI を実装した。解析・メディア実装の差替は
行っていない(AL#100/#101 のスコープ)。

## Context

`docs/designs/ruviform-dependency-ci.md`(approved)に基づく
`docs/tasks/ruviform-dependency-ci.md` の実装。上流の spec/ADR/
plan(AL#97 成果物)は別 worktree で未マージのため、front matter
relation が repository root 外を指す既知の manual finding が残る
(Phase-0 merge で解消)。

## Implementation

### Changes

- `[workspace.dependencies]` に 8 crate(ruviform-media-core /
  audio-core / video-core / media-io / math / dsp / audio-analysis /
  exec)を `git + rev` 固定で宣言し、`Cargo.lock` に
  `git+...?rev=<sha>#<sha>` source を記録した(単一 rev を確認)
- `.cargo/config.toml` に `net.git-fetch-with-cli = true` と
  `[env] GIT_LFS_SKIP_SMUDGE = "1"` を設定。private HTTPS fetch を
  system git(Git Credential Manager / CI の Deploy Key)に委譲する
- `publish = false` を workspace.package に置き、全 11 member
  crate が `publish.workspace = true` で継承するよう揃えた
  (git 依存は crates.io 公開不可のため、Ruviform と同一方針)
- `mise.toml` に actionlint 1.7.12、`check-ci`/`lint-ci`/`test-ci`/
  `build-ci`(`--profile ci` + `CARGO_INCREMENTAL=0`、成果物は
  `target/ci/` に分離)、`verify-toolchain`、`check-cli-headless`、
  `verify-ci` を追加
- `tools/verify-toolchain.mjs`: `mise current rust` の pin と
  `rustc`/`cargo` の解決バージョン一致 + rustfmt/clippy 存在を
  node で検証(3 OS 共通、POSIX 依存なし)
- `tools/check-headless-deps.mjs`: al-cli と解析/Job/Dataset 系の
  10 crate の `cargo tree` に GUI/WebView/GPU framework crate が
  無いことを検証(al-viz は AL#106 退役予定の表示 crate として
  設計上の例外を明記)
- `.github/workflows/ci.yml` を新設(workflow_dispatch 限定、
  3 OS、`permissions: contents: read`、read-only Deploy Key +
  `url.insteadOf` + GitHub 公開 host key による known_hosts 照合、
  `git ls-remote` probe、`cargo fetch --locked`、
  headless artifact build + boundary guard)
- `.pre-commit-config.yaml` に actionlint local hook を追加
- `docs/guides/ruviform-private-access-ci.md`: Deploy Key 発行・
  secret 登録・ローカル検証・ローテーション・PAT 代替手順の
  運用ガイド
- 4 crate(al-media/al-locate/al-analysis/al-job)に ruviform 基礎型
  import の smoke-test(`tests/ruviform_deps.rs`)を追加。
  ruviform-exec は AL#54 評価用に al-job の dev-dep として導入

### Decisions During Implementation

- **ruviform-io-symphonia は非採用**: AudioLens は symphonia
  0.5.5、ruviform io-symphonia は 0.6 を要求するため、本 Issue
  では依存せず lockfile で 0.5.5 単一を維持した(設計 §9 の
  Deferred。decode 移行は AL#100 で判断)
- **ruviform-rt は推移依存のみ**: 直接依存しないが
  media-core 等経由で lockfile に記録される(同一 rev)
- **smoke-test の `version()` 呼出しを部分撤収**: ruviform-dsp /
  audio-analysis は `version()` を公開しないため、当該 2 crate
  では基礎型構築のみを検証する
- **`al-cli` の `al-viz` 依存は例外として設計記録**: 現状 al-cli
  は al-viz(埋込み report アセット)に依存するが、guard の対象は
  GUI framework crate であり al-viz 自体は headless。退役は
  AL#106 のスコープとして guard コメントと設計 §7 に明記

## Related Experiments

### Adopted

なし。

### Rejected

なし。

## Validation

- `mise exec -- cargo fetch`: private repo から全 git 依存を
  取得成功(GCM 経由)。`Cargo.lock` の ruviform source は全 9
  entry(直接 8 + 推移 rt)が同一 `rev`/`#sha`
- `mise run verify-toolchain`: rustc/cargo 1.99.0 pin 一致、
  rustfmt/clippy 存在
- `mise run fmt` / `check` / `lint` / `test` / `build`:
  全て pass(既存契約維持。smoke-test 4 binary・全 test 緑)
- `mise run verify-ci`(fmt → check-ci → lint-ci → test-ci、
  `--profile ci`、`target/ci/` 分離): pass
- `mise run build-ci`: pass
- `mise run check-cli-headless`: 10 crate に GUI framework dep
  なし
- `mise exec -- actionlint .github/workflows/ci.yml`: clean
- `mise exec -- prek run --all-files`: 全 hook pass
  (actionlint hook は untracked 時点では対象外のため直接実行で
  確認。commit 後は hook 経由で発火する)

## Risks

- `RUVIFORM_DEPLOY_KEY` secret 未設定の間、CI は認証設定 step で
  明示失敗する(想定どおり fail-closed。登録手順はガイド参照)
- workflow_dispatch は default branch merge 後にのみ実行可能な
  ため、本 diff 上では ci.yml の e2e 検証未実施。actionlint +
  step 単位の静的確認 + ローカル再現手順で代替した
- Windows runner での `ssh` 解決(msys vs Win32-OpenSSH)と鍵
  permission 判定は初回 dispatch 実行で確認要。ガイドに
  トラブルシュートを記載
- 上流 relation 解消は Phase-0(AL#97)の merge 待ち

## Follow-ups

- `RUVIFORM_DEPLOY_KEY` secret 登録(運用者、ガイド §1-3)
- credential 設定完了後の `pull_request`/`push` 自動トリガ判断
  (設計 Deferred)
- AL#100 での decode backend(symphonia 0.5 vs ruviform
  io-symphonia 0.6)統一判断
- AL#54 での ruviform-exec OfflineDriver 適合性評価
- al-cli の al-viz 依存解体(AL#105/#106 と連携)

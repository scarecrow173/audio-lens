---
id: "IMPL-034R9TKOlzOgIYHn3BaBF3"
type: "impl"
status: "completed"
title: "Ruviform pin refresh to 073a3ff + consumer regression re-verification (Issue #127 follow-up)"
created: "2026-11-03"
updated: "2026-11-03"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/audio-lens/issues/127"
  changes:
    added:
      - type: "doc"
        path: "docs/impl/ir/ruviform-pin-refresh-073a3ff-consumer-regression-issue-127.md"
    modified:
      - type: "code"
        path: "Cargo.toml"
      - type: "code"
        path: "Cargo.lock"
      - type: "doc"
        path: "docs/impl/ir/README.md"
    deleted: []
    renamed: []
    moved: []
    generated:
      - type: "code"
        path: "ui/vendor/ruviform/"
  implements: []
  implemented-by: []
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/impl/ir/ruviform-rev-bump-consumer-regression-verification-issue-127.md"
    - "docs/impl/ir/gui-ruviform-controls-issue-131.md"
    - "docs/designs/ruviform-dependency-ci.md"
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

# Ruviform pin refresh to 073a3ff + consumer regression re-verification (Issue #127 follow-up)

## Summary

Issue #127 の継続追従として、ruviform git pin を
`e9e6abe514b8f3f0bfff2d976b8ca6551955ca09` から origin/main
HEAD `073a3ff698c5bf9002fe21c7547b04917601e058` へ更新した。
2 SHA 間の upstream 差分は ruviform 側 CI workflow + docs のみで
`crates/`・`ui/` のコード差分はゼロ。公開 API drift なし、
AudioLens 側 call-site 修正 0 件。全検証緑
(cargo 544 tests、ui npm 112 tests、webview lane、headless、
lint/fmt)。**影響分類: no impact**(消費面への影響なし、
pin 追従のみ)。

## Context

Issue #127(Ruviform 継続追従方針の owner issue)の follow-up
compat pass。前回 pin 更新(#131 で `e9e6abe` へ)以降に
ruviform main へ入った merge を評価対象とする。

- 依頼で挙げられた ui/binding(#261)・ui/showcase(#263)・
  consumer-verify gate 本体は、**既に現行 pin `e9e6abe` に
  含まれていた**(#131 で ui lane 取り込み済み)
- 新規に main へ入っていたのは `073a3ff` のみ:
  `ci(issue-219): consumer-verify anonymous-HTTPS access mode +
  棚卸し refresh`(ruviform PR #274)。ruviform 側の cross-repo
  consumer-verify workflow 更新 + AGENTS.md + docs の 8 ファイル

手順は `docs/designs/ruviform-dependency-ci.md` §1-§2 の規約に
従う(rev 一括置換 → Cargo.lock 単一 revision 確認 → mise task
で検証)。

## Implementation

### Changes

- `Cargo.toml` `[workspace.dependencies]` ruviform 14 entry の
  `rev` を `e9e6abe…` → `073a3ff…` へ一括置換(全 entry 同一 SHA)
- `Cargo.lock`: `cargo update -p ruviform-media-core` で再解決。
  15 ruviform source(直接 14 + 推移 ruviform-rt)が単一
  `?rev=073a3ff…#073a3ff6` に統一。旧 rev 参照 0 件
- `ui/vendor/ruviform/`:`node tools/sync-ruviform-ui.mjs` で
  再生成(gitignored 生成物。本 worktree では未生成だったため
  新規展開)。`REV` = `073a3ff…`
- AudioLens ソースの call-site 変更: **なし**

### Upstream 差分(e9e6abe..073a3ff)の影響分析

`git diff --name-only e9e6abe 073a3ff` = 8 files:

- `.github/workflows/consumer-verify.yml`
- `AGENTS.md`
- `docs/designs/issue-219-cross-repo-ci-gate-credential-runbook.md`
- `docs/discovery/issue-219-consumer-cross-repo-ci.md`
- `docs/impl/ir/issue-219-consumer-verify-anonymous-https-access-mode-refresh.md`
- `docs/specs/issue-219-consumer-compat-contract.md`
- `docs/tasks/README.md`
- `docs/tasks/issue-219-t5-anonymous-access.md`

`-- crates/ ui/` 限定の差分は **0 ファイル**。消費する全 crate
(media-core/media-io/io-avi/io-symphonia/audio-core/video-core/
dsp/math/numeric/audio-analysis/exec/ui/ui-webview/viz)に変更
なし。vendored `ui/*` TS(core/components/binding/viz)も
byte 同一(REV のみ更新)。API drift なし → adapter 変更不要。

内容: ruviform issue #219 の consumer-verify gate に
anonymous-HTTPS access mode を追加(deploy_key 未設定でも
public repo なら HTTPS で evidence を取れるようにする)+
audio-lens lane の `mise install` を lane 必須 tool
(rust/git-lfs/node)に scope する修正。全て ruviform 側 CI の
内部改善で、consumer の消費 API・behavior には無関係。

### Decisions During Implementation

- **pin は main HEAD 追従を維持**: 差分が CI/docs のみでも
  Issue #127 の「既定ブランチ追従を原則」方針に従い bump を
  実施。「更新不要」とせず pin を latest main に揃えた
- **vendored ui の差分検証は upstream diff で代替**:
  本 worktree の vendor tree は未生成だったため e9e6abe 時点の
  vendored copy との直接 diff は不可能。`ui/` 配下の upstream
  差分 0 件をもって vendored 内容が不変である根拠とした
  (sync 後 `tools/sync-ruviform-ui.mjs --check` が pin 一致を
  保証)

## Related Experiments

### Adopted

なし。

### Rejected

なし。

## Validation

- `git ls-remote … refs/heads/main` = `073a3ff…`(pin = main HEAD)
- `mise exec -- cargo fetch --locked`: pass
- `cargo update` 後の Cargo.lock: ruviform source 15 entry 全て
  単一 rev `073a3ff`、旧 rev `e9e6abe` 参照 0 件
- `node tools/sync-ruviform-ui.mjs`: synced @ `073a3ff698`
  → `ui/vendor/ruviform`(ui/core・components・binding・viz の
  `src/index.ts` 完備を `vendorComplete()` が確認)
- `mise run fmt`: pass
- `mise run lint`(biome check + clippy `-D warnings`): pass
- `mise run verify-toolchain`: rustc/cargo 1.99.0 + rustfmt/clippy
- `mise run check-cli-headless`: 10 headless crate に GUI
  framework dep なし
- `mise run test`(cargo test --workspace): **544 passed /
  0 failed**(46 test binary)。golden 系(al-media/al-analysis/
  al-locate tests/golden.rs)、locate/tracking・consensus の判断
  回帰、avsync、ruviform_deps provenance テスト全緑
- `mise run check-ui`: vendor `--check` + `tsc --noEmit` clean
- `mise run test-ui`(npm test、happy-dom): **112 tests / 0 fail**
- `mise run check-ui-webview` / `lint-ui-webview` /
  `test-ui-webview`(al-gui `webview` feature lane、wry/tao 実窓
  host target): pass、5 テスト全緑

### Issue #127 consumer checklist 対応

- Media ingest / DSP・解析 f64 / Offline・Output / Job・Dataset /
  GUI・headless: **再検証済み**(上記全テスト緑、前回 IR と
  同一カバレッジ)
- 性能・平台: **未検証**(本 pass は Windows dev profile。
  macOS/Linux lane・baseline 計測は従来どおり別トリガ)

## 影響分類(Issue #127 記録用)

**no impact** — pin `e9e6abe → 073a3ff` は upstream の
ruviform 側 CI workflow + docs 変更のみで、消費コード差分ゼロ、
API drift ゼロ、AudioLens 側修正ゼロ、全 consumer 回帰緑。
証拠は本 record の Validation 節。

## Risks

- ruviform `073a3ff` の commit message が、consumer-verify CI
  の audio-lens lane 実走で「latent consumer-side export path
  leak(3 al-cli cli tests)on ubuntu-latest」を検出したと記録。
  AudioLens 側の潜在不具合の可能性があるが、本 worktree は
  Windows lane のため再現確認・issue 起票判断は別途必要
  (本 repo に該当 issue は未起票 — 2026-11-03 時点)
- macOS/Linux lane・baseline 計測は未実施(従来どおり)

## Follow-ups

- ubuntu-latest での al-cli export path leak 疑義:
  ruviform consumer-verify の証跡を確認し、AudioLens issue の
  起票要否を coordinator / レビューで判断
- Issue #127 本体の checklist への反映は coordinator 経由
  (本 record が根拠)

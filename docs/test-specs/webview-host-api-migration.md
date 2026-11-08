---
id: "TSPEC-IRsd8bYu51nBjXIjYYEPMS"
type: "test-spec"
status: "approved"
title: "al-gui webview host API 移行のテスト仕様"
created: "2026-11-08"
updated: "2026-11-08"
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
  derives-from:
    - "docs/plans/webview-host-api-migration.md"
  derived-by: []
  verifies:
    - "docs/specs/webview-host-api-migration.md"
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

## 検証方針

失敗は compile 時の unresolved import であり、runtime の webview
挙動ではない。検証は「pin `863a7a9d` で `al-gui --features webview`
の全 target が check/clippy/test を通ること」と、event routing /
close・job policy が `DesktopSession` の既存 headless fixture で
担保されていることの確認に集中する。実 OS window は lane の対象外。

## テスト

### 自動(mise task、実行済み)

- `mise run check-ui-webview`
  (`cargo check -p al-gui --features webview --all-targets`):
  `bin/al-gui` を含む全 target の型チェック。移行前は
  `error[E0432]` で失敗、移行後 pass(実測)。
- `mise run lint-ui-webview`
  (`cargo clippy -p al-gui --all-targets --features webview --
  -D warnings`): warning ゼロを要求。実測 pass。
- `mise run test-ui-webview`
  (`cargo test -p al-gui --features webview`): lib/integration
  テスト一式。`DesktopSession` の FakeDriver lifecycle fixture
  (`tests/desktop.rs`)が close/job policy を headless で検証する。
  実測 pass。

### 非対象(明示)

- 実 OS window での webview 起動・IPC 往復: webview-verify lane の
  契約上 compile/lint/test のみ。実機確認が必要なら別タスクで
  手動検証として記録する(NotRun)。
- CI `webview-verify` job 自体: PR 作成後の CI 実行で確認する。

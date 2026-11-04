---
id: "SPEC-TW82peIZOd6TntYAFQf8Vw"
type: "spec"
status: "approved"
title: "AudioLens 独立 PR/Push CI: 公開/非公開 Git 依存対応と回帰 Gate"
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
  implements: []
  implemented-by:
    - "../designs/audiolens-independent-ci.md"
    - "../plans/implement-audiolens-independent-ci.md"
  depends-on:
    - "../designs/ruviform-dependency-ci.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "https://github.com/scarecrow173/audio-lens/issues/140"
    - "https://github.com/scarecrow173/audio-lens/issues/141"
    - "https://github.com/scarecrow173/audio-lens/issues/127"
    - "https://github.com/scarecrow173/ruviform/issues/294"
    - "https://github.com/scarecrow173/ruviform/issues/219"
  refines:
    - "../designs/ruviform-dependency-ci.md"
  refined-by: []
  derives-from:
    - "https://github.com/scarecrow173/audio-lens/issues/142"
  derived-by: []
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# AudioLens 独立 PR/Push CI: 公開/非公開 Git 依存対応と回帰 Gate

## What Should Be Built

`.github/workflows/ci.yml` を `workflow_dispatch` 専用の手動検証
workflow から、`pull_request` / main `push` / 手動再実行で起動する
AudioLens 独立の継続検証 CI へ拡張する。Ruviform 側の
cross-consumer candidate Gate(ruviform #294)を代替せず、AL#127 の
Consumer 回帰を AudioLens 自身の PR/Push で継続検証する。

含めるもの:

- `pull_request`(main 向け)・`push`(main)・`workflow_dispatch`
  トリガ。最小権限 `contents: read`、同一 ref の重複実行は PR で
  cancel・main では履歴保全、OS ごとの独立した job 結果
- Ruviform Git 依存のアクセス経路を 2 モードで解決:
  public 時は匿名 HTTPS(最小権限 = 秘密を持たない)、private 復帰
  時は read-only Deploy Key。アクセス不能は緑扱いせず明示
  fail / not-run とする
- CI 実行に必要な mise ツールへの scope(`--locked` 再現性を維持)。
  開発ツール全量(`apm`/`pipx:serena-agent`)は CI から除外する
- lane 分離: Rust fmt/check/lint/test(Golden・実メディア LFS
  fixture を含む)、CLI build + headless 依存境界、`al-gui`
  `webview` feature、`ui/` npm typecheck/test +
  `sync-ui-deps --check`、実 Window/OS 試験の not-run 明示
- `cargo fetch --locked` 後に Ruviform Git 依存の resolved SHA
  単一性と UI vendor `REV` 一致を検証する。manifest `rev` 列挙
  方式と lockfile 主体方式(#140)の両方で動くこと
- lane/OS/feature/resolved SHA/結果(Pass/Fail/NotRun/Unsupported)
  を記録する固定レポート形式と、失敗時の Consumer/Ruviform Owner
  切り分け指針

## Why Now

現行 `ci.yml` は dispatch 専用で通常の PR/Push を検証しない。
また `RUVIFORM_DEPLOY_KEY` 必須の private 専用経路のみ残るが、
現在両 repo は public であり、credential 未設定だと Git 依存を
取得できない構造になっている。#131/#132 で `ui/` TypeScript
連携と vendored 同期が入ったが、CI には sync-check/tests の
lane が無い。#140 の lockfile 主体化と同時進行のため、依存
検証は両方式で成立する設計が必要。

## Requirements

1. PR/Push/Manual の 3 トリガで CI が起動し、対象 branch/commit・
   OS・feature・依存 resolved SHA・失敗ケースが追跡できる
2. 匿名 HTTPS(public)と read-only Deploy Key(private)の両経路を
   持ち、鍵をログへ出さない。不正な fallback(鍵なし SSH 試行等)を
   行わず、到達不能は lane を fail または明示 not-run とする
3. `mise install --locked` は CI が使うツールへ scope し、lockfile
   の platform entry 欠落があるツールは gap を記録する
4. 実行不能な環境・未整備の Gate(実 Window GUI 等)は Pass と
   偽らず NotRun/Unsupported として記録する
5. 既存の `mise run test` / `check` / `lint` 等、ローカル開発の
   task 意味を変更しない

## Non-Goals

- Ruviform candidate SHA の cross-consumer Gate 実装
  (ruviform #294 が Owner。本 CI は pin 済み resolved SHA を検証)
- 実 Window/OS GUI 自動試験(ruviform #312/#313 接続の
  opt-in/専用 Runner で別 Gate)
- `al-cli` Export 3件失敗の個別修正(#141 が Owner)
- manifest `rev` 撤去そのもの(#140 が Owner。両方式で動く
  検証のみ提供)

## Acceptance Criteria

- [ ] PR/Push/Manual トリガで CI が起動し、対象 branch/commit・
      OS・feature・依存 resolved SHA・失敗ケースを追跡できる
- [ ] 3 OS の可能な lane の結果が記録され、not-run / ゼロ件を
      pass と偽らない
- [ ] public 匿名アクセス・private 最小権限アクセス・認証失敗が
      検証され、secret を表示しない
- [ ] Rust + CLI/headless + UI TypeScript + Golden/Export の
      必要な Gate を含み、実機 GUI 未検証を明示する
- [ ] Export 失敗の個別 Issue(#141)解消と Ruviform
      cross-consumer 再実測(#294)を参照できる
- [ ] 設定変更が既存の `mise run test` 等の意味を破壊しない

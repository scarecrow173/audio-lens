---
id: "ADR-034yXLLpyO9jy0n7L5z50Y"
type: "adr"
status: "accepted"
title: "要件・設計の正本を docs/ とし issues は草案とする"
created: "2026-10-02"
updated: "2026-10-02"
owners: ["scarecrow173"]
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
metadata:
  adr:
    decision-makers: ["scarecrow173"]
    consulted: []
    informed: []
---

# 要件・設計の正本を docs/ とし issues は草案とする

## Context and Problem Statement

現状、AudioLens の要件と設計は GitHub issues(#1 全体スコープ、
#2〜#14 各 stage)にのみ存在する。正本がリポジトリ外にあると、
変更履歴がコードと分断され、doc-driven-dev の artifact graph
(spec / design / ADR / task の関連付け・検証)にも載せられない。

## Considered Options

* GitHub issues を正本として使い続ける
* `docs/` に canonical 文書を置き、issues は草案置き場兼
  work tracking とする
* issues と docs の両方を正本として同期運用する

## Decision Outcome

Chosen option: "`docs/` に canonical 文書を置き、issues は草案置き場兼
work tracking とする"。正本をリポジトリ内の Markdown に置くことで、
コードと同じ version 管理・レビュー導線に乗り、doc-driven-dev の
graph / audit の対象にできるため。issues と docs の二重正本は
drift の温床になるため採用しない。

## Implementation Plan

* Affected paths: `docs/specs/`、`docs/designs/`、`docs/adr/`、
  `docs/tasks/` 等(scaffold 済み)。issues の内容は対応する
  canonical 文書へ整備・移行する
* Constraints: `docs/` に正本がある領域は docs を優先し、issue
  との差異は docs 側を正とする。新たに確定した事項は issue
  への追記ではなく `docs/` の文書として記録する。issues 自体は
  削除せず work tracking として残す

## Verification

* [x] `docs/specs/overall-architecture.md` に issue #1 の内容が
      転記されている
* [x] `docs/` 各ディレクトリの index(README.md)が scaffold で
      生成済み

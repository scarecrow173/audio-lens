---
id: "ADR-034yXLLQXeff2g3MpiLULN"
type: "adr"
status: "accepted"
title: "言語 stack 未決定のため最小 toolchain のみ pin する"
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
  related:
    - "docs-issues.md"
  refines: []
  refined-by: []
  derives-from: []
  derived-by: []
  verifies: []
  verified-by: []
  references:
    - "../../mise.toml"
    - "../../apm.yml"
  defers: []
  deferred-by: []
metadata:
  adr:
    decision-makers: ["scarecrow173"]
    consulted: []
    informed: []
---

# 言語 stack 未決定のため最小 toolchain のみ pin する

## Context and Problem Statement

AudioLens の実装言語 stack は未決定(数値解析・media decode・
columnar storage の要件を踏まえて今後選定する)。一方で、
`apm install`・serena MCP・doc-driven-dev スクリプトの実行には
Python 系 runtime が必要であり、fresh checkout を開発可能にする
setup 契約は先に必要だった。

## Considered Options

* node / pnpm / python / uv 等の polyglot toolchain を予め pin
  する(参考リポジトリ cheaper・code-knowledge と同構成)
* ツール実行に必要な最小構成のみ pin し、言語 toolchain は
  決定次第追加する
* toolchain を置かず global 環境に依存する

## Decision Outcome

Chosen option: "ツール実行に必要な最小構成のみ pin し、言語
toolchain は決定次第追加する"。未決定の stack を先回りして
pin すると lock の意味が薄れ、YAGNI に反するため。ただし
`apm` / `serena` の実行基盤(python 3.12・uv 0.11・
pipx:apm-cli 0.28.0・pipx:serena-agent 1.7.0)は setup 契約の
ために pin する。`pipx:apm-cli` は 0.30.0 が Windows で
path dep 解決を壊すため、cheaper と同じく 0.28.0 に固定する。

## Implementation Plan

* Affected paths: `mise.toml`、`mise.lock`、`apm.yml`、
  `apm.lock.yaml`
* Constraints: 言語 stack 決定時に `mise.toml` の `[tools]` と
  検証 task(lint / typecheck / test 等)を追加し、本 ADR を
  更新または supersede する。`mise.lock` は `mise lock` で
  再生成して commit 対象に含める

## Verification

* [x] `mise install` で 4 ツールが解決される(python 3.12.15 /
      uv 0.11.33 / apm-cli 0.28.0 / serena-agent 1.7.0)
* [x] `apm install` で 31 APM 依存 + 2 MCP server(serena,
      mise)が install され `apm.lock.yaml` が生成される
* [x] `mise.lock` が生成され 4 ツールのみを含む

---
id: "SPEC-034Rp2PraZGXQhVUwhn5KL"
type: "spec"
status: "approved"
title: "エクスポートパッケージのテキスト成果物可搬投影"
created: "2026-11-03"
updated: "2026-11-04"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/audio-lens/issues/141"
  changes:
    added: []
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements: []
  implemented-by: []
  depends-on:
    - "analysis-export.md"
    - "analysis-dataset.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "../tasks/export-package.md"
  refines:
    - "analysis-export.md"
  refined-by: []
  derives-from: []
  derived-by: []
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# エクスポートパッケージのテキスト成果物可搬投影

## What Should Be Built

エクスポートパッケージ(`export/<name>/`)内の**すべての**テキスト
成果物に対し、ホスト固有情報(source path・job dir・任意の絶対
パス)を流出させない可搬投影を定義・実装する。

現行実装は `analysis/manifest.json` の `inputs[].source_path` と
`config` のみを縮約し、`analysis/inputs/declared.json`・
`analysis/inputs/<id>.metadata.json`・将来追加される JSON 成果物は
生のコピーのまま絶対パスを保持していた。Ubuntu では末尾の安全性
Gate が漏洩を検出して Export が fail-closed で失敗し、Windows
では JSON エスケープされた `\\` 区切り表現が Gate をすり抜けて
非可搬パッケージが静かに公開されていた。

## Why Now

Ruviform 横断 consumer-verify(run #41604498852)の AudioLens
lane で `crates/al-cli/tests/cli.rs` の 3 テストが失敗する P0
実害として検出された(issue #141)。`analysis-export.md` が要求する
「パッケージ内パスは再配置可能」という契約が、データセット内の
テキスト成果物全体に対して成立していなかった。

## Users and Value

- バグレポート / 横断 consumer 利用者: パッケージを別ホスト・
  別 OS へ移してもプライベートなホストパスが混入しない
- AudioLens CI: Linux lane で fail-closed 失敗、Windows lane で
  静かな漏洩、という OS 依存の挙動差を解消する

## Scope

### In Scope

- コピー済み `analysis/` 配下の全 `.json` 成果物への
  スキーマ耐性のある可搬投影(絶対パス文字列→ファイル名)
- Portability gate の検査表現を `/` 正規化・素の `\`・JSON
  エスケープ `\\` の全 spelling に拡張
- Export 途中失敗時の部分成果物と完成品の分離(staging publish)
- POSIX / Windows drive / UNC / Unicode / 相対パス / 同一
  basename 別 input の試験

### Out of Scope

- 元 Dataset / Job manifest / Source Provenance の変更
  (projection は Export コピーにのみ適用)
- `.json` 以外のテキスト成果物の内容変換(構造化投影は JSON のみ。
  非 JSON テキストは gate の fail-closed 対象)
- Ruviform 共通 IO 基盤への製品固有 metadata 投影の移管
  (ruviform#282、AudioLens #139 の将来移行は待たない)
- バイナリ成果物(WAV・chunk `.bin`)の走査・変換

## Requirements

- 元メディア名・input ID・digest(`source_sha256` / `source_hash`)・
  role・時刻/ストリーム metadata・Validity を投影後も保持する
- JSON 値のみを変換し、構造・キー・数値・真偽値を破壊しない。
  変更の無いファイルは byte を変えない
- 投影対象は「絶対パスらしい文字列値」であって任意テキストの
  文字列置換ではない(POSIX root・drive-letter・UNC を全ホスト
  規約で判定)
- サニタイズ不能(`.json` が parse 不能、非 JSON テキストに
  private path 残存)は fail-closed し、元データを破壊せず
  対象ファイルを報告する
- 失敗した Export は部分成果物を `export/<name>/` に残さず、
  成功済みパッケージを上書きで破壊しない
- Gate の private path 集合(declared input の絶対パス + job dir)
  を緩めず、全 separator/escape spelling で検査する

## Acceptance Criteria

- [ ] issue #141 で失敗した 3 テストが Ubuntu CI profile で pass
      する(本 worker は Windows のため、同一修正で CLI 15 件と
      al-dataset 45 件の全緑 + 実 Export 成果物の漏洩スキャンで
      代替検証し、Ubuntu 実測は CI 実行担当へ記録する)
- [ ] Windows でも export 成果物の全テキストに絶対パス spelling
      が残存しない(エスケープ差を含む)
- [ ] 元 Dataset の provenance・digest・stream metadata が保持
      され、Export パッケージが単体で再読込可能
- [ ] 解析 golden・数値精度・CLI/headless・失敗時部分状態が
      回帰しない

## Deferred Design Concerns

- `.json` 以外の非公開テキスト形式(`.txt`/`.html` レポート)を
  データセットに追加する場合の投影方式 | 現行は gate で
  fail-closed | 非 JSON テキスト成果物の追加時 | 新形式を追加する
  実装者が投影を忘れて漏洩を公開するリスク

## Sources and Relations

- Issue #141(受入の正本)、`analysis-export.md` の可搬性要件、
  `docs/tasks/export-package.md` の既存「絶対パス除去」作業の
  対象外だった成果物をカバーする拡張

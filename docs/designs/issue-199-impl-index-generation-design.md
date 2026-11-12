---
id: "DESIGN-034WsMaALpLsJSgSFhJLyp"
type: "design"
status: "approved"
title: "Issue #199 impl index 生成対応: front matter 是正と managed marker 設計"
created: "2026-11-12"
updated: "2026-11-12"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/audio-lens/issues/199"
  changes:
    added:
      - type: "doc"
        path: "docs/specs/issue-199-impl-index-generation.md"
      - type: "doc"
        path: "docs/designs/issue-199-impl-index-generation-design.md"
      - type: "doc"
        path: "docs/plans/issue-199-impl-index-generation-plan.md"
      - type: "doc"
        path: "docs/tasks/issue-199-impl-index-generation.md"
      - type: "doc"
        path: "docs/test-specs/issue-199-impl-index-generation.md"
      - type: "doc"
        path: "docs/impl/ir/issue-199-impl-index-generation.md"
    modified:
      - type: "doc"
        path: "docs/impl/ir/impact-based-ci-macos-separation-al-149.md"
      - type: "doc"
        path: "docs/impl/ir/issue-101-ruviform-numeric-migration.md"
      - type: "doc"
        path: "docs/impl/ir/issue-102-domain-separation.md"
      - type: "doc"
        path: "docs/impl/ir/issue-103-job-dataset-provenance-regression.md"
      - type: "doc"
        path: "docs/impl/ir/issue-105-gui-app.md"
      - type: "doc"
        path: "docs/impl/ir/issue-106-viz-removal.md"
      - type: "doc"
        path: "docs/impl/ir/issue-117-ruviform-contract-api.md"
      - type: "doc"
        path: "docs/impl/ir/native-gui-drawn-parity-run-mount.md"
      - type: "doc"
        path: "docs/impl/ir/native-gui-job-receipt-consumption.md"
      - type: "doc"
        path: "docs/impl/ir/native-gui-job-run-mount.md"
      - type: "doc"
        path: "docs/impl/ir/native-gui-product-screen-mount.md"
      - type: "doc"
        path: "docs/impl/ir/native-gui-spectrogram-analysis-projection.md"
      - type: "doc"
        path: "docs/impl/ir/native-gui-tabs-selection.md"
      - type: "doc"
        path: "docs/impl/ir/native-gui-waveform-seek-screen.md"
      - type: "doc"
        path: "docs/impl/ir/perf-reeval-2026-10.md"
      - type: "doc"
        path: "docs/impl/ir/ruviform-lockfile-source-of-truth.md"
      - type: "doc"
        path: "docs/impl/ir/ruviform-logging-span-events-compat.md"
      - type: "doc"
        path: "docs/impl/ir/spectrogram-tab-hang.md"
    deleted: []
    renamed: []
    moved: []
    generated:
      - type: "doc"
        path: "docs/impl/ir/README.md"
      - type: "doc"
        path: "docs/impl/exp/README.md"
      - type: "doc"
        path: "docs/specs/README.md"
      - type: "doc"
        path: "docs/designs/README.md"
      - type: "doc"
        path: "docs/plans/README.md"
      - type: "doc"
        path: "docs/tasks/README.md"
      - type: "doc"
        path: "docs/test-specs/README.md"
  implements:
    - "../specs/issue-199-impl-index-generation.md"
  implemented-by:
    - "../plans/issue-199-impl-index-generation-plan.md"
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "https://github.com/scarecrow173/ruviform/issues/586"
    - "https://github.com/scarecrow173/synvelis/issues/365"
  refines: []
  refined-by: []
  derives-from:
    - "../specs/issue-199-impl-index-generation.md"
  derived-by: []
  verifies: []
  verified-by:
    - "../test-specs/issue-199-impl-index-generation.md"
  references: []
  defers: []
  deferred-by: []
---

# Issue #199 impl index 生成対応: front matter 是正と managed marker 設計

## Context

`doc-maintenance` の `index` 動詞は `docs/impl/ir`・`docs/impl/exp`
について `impl-doc` の `sync_impl_index` safe engine に委譲する。
この engine は fail-closed であり、(a) index が
`<!-- doc-suite:generated-index -->` marker を持つ managed 形式で
あること、(b) 全 source 文書が canonical dir の schema を満たす
こと、(c) source hash・file list・index bytes が plan 時と apply
時で一致すること、を要求する。marker がない hand-curated index は
`promote_index`(byte 一致する旧 scaffold のみ対象)でも force 経路
でも救済できないため、marker 追加が唯一の移行経路である。

調査の結果、`index plan --dir docs/impl/ir` の `rejected` の原因は
renderer ではなく source 側にあった: 18 ファイルが schema 不適合
(`metadata` 欠落、`relations.changes` が文字列配列、front matter
全体の欠落、不正 status)。`docs/impl/exp` は source(jsonl file
list)が健全で、index の marker 欠落のみが原因だった。

## Scope

### In Scope

- `docs/impl/ir` 18 ファイルの front matter 形式修正。
- `docs/impl/ir/README.md`・`docs/impl/exp/README.md` の managed
  index 化(marker + `Directory:` 行 + 生成テーブル)。
- 正規 plan→apply→plan(in-sync)手順と引数なし apply の検証。

### Out of Scope

- 本文・ファイル名・id の変更、audit で可視化された本文セクション
  不足の補完(別 Issue)、他 dir の index 手編集、製品コード変更。

## Components and Boundaries

### 1. 実装記録 front matter schema(正本: `audit_impl_record.js`)

- 必須: `id`(非空), `type: "impl"`, `status` ∈
  {draft, in-progress, completed, blocked, abandoned, superseded},
  `title`, `created`/`updated`(YYYY-MM-DD), `owners[]`,
  `relations`, `metadata.experiments.{adopted,rejected}[]`。
- `relations.changes` は `added/modified/deleted/renamed/moved/
  generated` 各キーに `{type: string(min 1), ...}` object の配列。
  文字列配列は不可。
- その他の relation フィールドは string 配列。ローカル path は
  存在確認される(broken-relation-link は warning)。

### 2. front matter 修正の分類と方針

調査で違反は 4 類型に分類された(延べ 18 ファイル、一部重複あり)。
すべて機械的・意味保存の修正:

- **metadata.experiments 欠落**(7 ファイル): 既存 `relations` は
  保持し、`metadata.experiments.{adopted,rejected}: []` を追加。
  採用/棄却実験の記述が本文にないため空配列。
- **changes 文字列配列**(2 ファイル): 各文字列を path とみなし
  `{type, path}` object へ変換。type は path の拡張子・性質から
  機械的に判定(`.rs`→code、`.md`→doc、`Cargo.toml`/`Cargo.lock`/
  `mise.toml`→manifest/lockfile、`.yml`→code 等)。既存の正しい
  IR の type 語彙に合わせる。
- **front matter 不完全・全欠落**(9 ファイル): native-gui 系 2
  ファイルは既存 front matter を補完し、issue-101/102/103/105/
  106/117・perf-reeval の 7 ファイルは front matter が皆無だった
  ため H1・対象 Issue・日付から `id`(新規採番 `IMPL-…`)、
  `type: "impl"`、`status: "completed"`(完了済みの歴史的記録)、
  `title`(H1 由来)、`created`/`updated`、`owners: []`、
  `relations.source`(Issue URL)、`implements`(対応 task doc が
  存在する場合のみ)、残り relation 空配列、`metadata.experiments`
  空で生成。本文は一切変更しない。
- **不正 status**(spectrogram-tab-hang): 本文が完了済み作業を
  記述しているため `completed` に正規化。

### 3. managed index 化

- `docs/impl/ir/README.md`: 旧来の手書き `Task | Record` 表は
  managed index に置き換える。preamble(title + 説明文)は保持し、
  marker 以降を `Directory:` + 4 列テーブル(ID | Title | Status |
  File)+ `## Settled <!-- doc-suite:settled-index -->` 構成へ。
  旧表の task→file 対応情報は各文書の `implements` relation と
  新 index の File リンクで代替可能なため複製しない。
- `docs/impl/exp/README.md`: title + `Directory:` + 1 列 File 表の
  既存構造に marker を挿入し、jsonl file list を決定的順で再生成。
- 両者とも `index apply` が source から再生成するため、手書き表の
  内容は最終的に生成物で置き換わる。

### 4. index 形式(IR / EXP)

- IR: active(status が completed/abandoned/superseded 以外)と
  settled の 2 tier。filename 順で決定的。4 列
  `ID | Title | Status | File`。
- EXP: `.jsonl` file list を filename 順の 1 列 `File` 表。
  front matter 不要。

## Data and Control Flow

- `index plan`: source 文書走査 → front matter parse + schema 検査
  → 期待 index 行生成 → 現 index との差分を planHash 付きで報告。
- `index apply --expect <planHash>`: plan を再計算し hash・source
  pin・index bytes を照合してから atomic write。不一致は拒否。
- `index plan`(再): `in-sync` なら完了。以降の文書追加/status 変更は
  同手順で再生成する運用に載る。

## Non-Functional Constraints

- 意味保存: front matter 修正は schema 適合のための形式修正に限り、
  本文・id・ファイル名を変更しない。status/title/created/updated
  等の値は本文記述と git 履歴から保守的に推定し、判断は PR 本文に
  明記する。
- fail-closed: safe engine の plan/apply 検証を迂回しない
  (promote/force 不使用)。
- 既存の手書き preamble は merge で保持される(preamble/tail は
  managed region 外として残る)。

## Risks and Trade-offs

- front matter 修正で audit の missing-section 検査が活性化し、
  error 件数が増える → coordinator 判断で「新規 error なし」と
  解釈し、内訳(invalid-front-matter 減少 vs missing-section
  可視化増加)を PR 本文に記録。本文補完は別 Issue。
- 新規採番 ID の一意性 → 既存全 id との衝突を確認した上で
  ランダム採番し、doc-status audit で invalid-id-prefix 等が
  増えないことを確認する。
- 旧 `Task | Record` 表の喪失 → relation/新 index で代替可能、
  情報損失なしと判断(spec の Deferred Design Concerns に記録)。

## Deferred Design Concerns

- 既存 IR 本文の必須セクション補完 | 別 Issue(coordinator 起票)
  | audit error 解消時 | 現状 error が残る
- `relations.changes` の type 語彙の正規化 | 現状は path からの
  機械的推定 | schema に type enum が導入された場合 | 語彙の揺れが
  残る可能性

## Verification Notes

- [ ] `doc_maintenance.js index plan --dir docs/impl/ir --json` が
      `rejected` でなく `sync`/`in-sync` を返す。
- [ ] `index apply --dir docs/impl/ir --expect <hash>` 後の
      `index plan` が `in-sync`。
- [ ] `docs/impl/exp` も同様に `in-sync`。
- [ ] 引数なし `index apply` が exit 0。
- [ ] `audit_impl_record.js --json`・`audit_experiment_log.js
      --json` の error 内訳をベースラインと比較し、新規 error が
      ないこと。
- [ ] `audit_docs.js --type all --json` で新規 doc 由来の error が
      増えていないこと。

---
id: "KNOW-NrL9tC9HxfoCXX4fs4vbNh"
type: "knowledge"
status: "confirmed"
lifecycle: "current"
knowledge_kind: "architecture"
title: "CI アーキテクチャ: 独立 CI・fail-closed 概要・Ruviform アクセス"
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

# CI アーキテクチャ: 独立 CI・fail-closed 概要・Ruviform アクセス

AudioLens の CI は GitHub Actions ベースで、AL#142(独立 CI 化)、AL#141(Ruviform 移行時の private アクセス)、AL#143(#142 merge 後の修正)を経て現在の構成になった。本書は CI の構造的設計、fail-closed 概要判定、Ruviform アクセスモードの設計をまとめる。

## 独立 CI 構成(AL#142)

- 対象 event は `pull_request`/`push` on `main` と `workflow_dispatch`。**paths filter を付けない** —— prek 衛生チェックは全ファイルを対象にする必要があるため、ドキュメントのみの変更でも CI が走る。
- `check-ci`/`lint-ci`/`test-ci`/`build-ci` は `--profile ci`+`CARGO_INCREMENTAL=0` で `target/ci/` 分離。`fmt`/`check`/`lint`/`test` の 4 lane + headless 依存検査 + UI lane(setup-ui→sync-ui-deps→check-ui→test-ui)+ CLI build。
- workflow 実行は dispatch 前提だが、PR 前の検証は actionlint+step 静的確認+ローカル再現で代替する(workflow_dispatch は default branch merge 後のみ)。

## 概要判定の fail-closed 設計

summary job は「未実行・キャンセル・スキップされた lane」を成功とみなさない —— `not_run`/`skipped`/`cancelled` は failure として扱う fail-closed 判定。これは「CI lane が条件不成立で暗黙スキップされると全 green に見える」問題への対処で、観測不能は Pass でなく `NotRun`/`Fail` に分類する原則の CI 版。

## Ruviform アクセス(AL#141)

- private Git 依存の CI アクセスは preflight job で「利用可能なアクセスモード」を解決し、**モード未解決は認証 step で明示失敗**(fail-closed)。`RUVIFORM_DEPLOY_KEY` 未設定時はデフォルト GITHUB_TOKEN で private repo へ届かず、認証 step で明示失敗する設計。
- Deploy Key 経路は read-only Deploy Key + `url.insteadOf` + GitHub 公開 host key の known_hosts 照合 + `git ls-remote` probe + `cargo fetch --locked`。
- `net.git-fetch-with-cli = true` + `GIT_LFS_SKIP_SMUDGE=1` で private HTTPS fetch を system git(GCM / CI Deploy Key)へ委譲 —— Windows(GCM)と CI(Deploy Key)で共通に動く構成。

## Ruviform lockfile 正本化(AL#140)

- resolved revision 正本は `Cargo.lock` の `source` fragment(`git+<url>#<sha>`)へ移行。`Cargo.toml` の `rev` 列挙は撤去(14 件の ruviform git 依存から `rev` 除去、URL と feature は保持)。
- `tools/ruviform-lock-rev.mjs`: lockfile `[[package]].source` を構造的に解釈する単一取得・検証契約。URL/host 正規化、`#<sha>`(40 桁 hex)抽出、同一 URL 由来の全 ruviform crate が単一 SHA であることの fail-closed 検証(ゼロ件・分裂・未知形式・混在 URL は全て失敗)。
- `tools/update-ruviform-rev.mjs`: candidate SHA を `cargo update --precise` で lockfile へ明示適用。workspace で `--precise` 実行(cargo が git source 単位で原子更新、旧 SHA への更新は lockfile 無変更で失敗)。
- `check-ruviform-lock` mise task で lockfile 整合を検証、UI lane で依存解決 revision と `ui/vendor/ruviform/REV` の一致を CI 検証。

## 未整備の CI 領域

- 実 workflow 実行・Deploy Key 経路・3 OS matrix の実機検証は当時未実施(dispatch 不可・secret 未設定環境)。
- `check-ui-driver-boundary`・`test-ui-parity` は mise.toml に存在しない task として NotRun のまま残った。

## 設計上の教訓

- 「paths filter を付けない」は衛生チェックを全域に走らせるための意図的選択 —— 軽量化のために paths filter を付けると prek が見逃す変更が発生する。
- アクセスモードは「デフォルトで動くか」を賭けず、preflight で明示解決+fail-closed。
- lockfile 正本化では契約ツールと manifest 変更を同一コミットにする(sync ツールが正本遷移の隙間で失敗するため)。
- 「manifest rev 必須」記述を設計・ガイド・コメントから一括洗い出す必要がある(表現ドリフトの掃除)。

関連 Issue/PR: AL#140、AL#141、AL#142、AL#143、AL#99、#127、#131、#137、RF#219、RF#294。

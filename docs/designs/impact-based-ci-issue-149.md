---
id: "DESIGN-034RNl7ph9ojvXsoInYjLx"
type: "design"
status: "approved"
title: "Issue #149 変更影響ベース CI — AudioLens 分類規約・期待集合 Fail-Closed Gate・Shadow Mode 移行"
created: "2026-11-04"
updated: "2026-11-04"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/audio-lens/issues/149"
  changes:
    added:
      - type: "file"
        path: "tools/classify-impact.mjs"
    modified:
      - type: "file"
        path: ".github/workflows/ci.yml"
      - type: "file"
        path: "docs/designs/audiolens-independent-ci.md"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements: []
  implemented-by: []
  depends-on:
    - "../designs/audiolens-independent-ci.md"
    - "../designs/sccache-compile-cache-issue-148.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "https://github.com/scarecrow173/ruviform/issues/333"
    - "https://github.com/scarecrow173/ruviform/issues/327"
    - "https://github.com/scarecrow173/ruviform/issues/326"
    - "https://github.com/scarecrow173/audio-lens/issues/142"
    - "https://github.com/scarecrow173/audio-lens/issues/147"
    - "https://github.com/scarecrow173/audio-lens/issues/148"
  refines:
    - "../designs/audiolens-independent-ci.md"
  refined-by: []
  derives-from: []
  derived-by: []
  verifies: []
  verified-by: []
  references:
    - "https://github.com/scarecrow173/ruviform/issues/333"
  defers: []
  deferred-by: []
---

# Issue #149 変更影響ベース CI — AudioLens 分類規約・期待集合 Fail-Closed Gate・Shadow Mode 移行

## Context

AudioLens 独立 CI(#142)は PR / main push / dispatch で
`verify`・`ui-verify`・`webview-verify`(各 3 OS)を毎回全実行する。
変更と無関係な lane の実行は待ち時間と Runner 消費を増やすが、
現行 `summary` gate は「全 lane が走って success だった」ことしか
検査できず、job を単純に `if:` で skip すると GitHub の skipped =
success 扱いによって実行欠落が緑に見えうる。

Ruviform [#333](https://github.com/scarecrow173/ruviform/issues/333)
(CI-OPT-06)が汎用 Impact 分類契約の Owner として並行実装中。
本設計はその AudioLens 側適用で、Issue 本文の契約仕様
(期待集合・Pass/Fail/NotRun/Unsupported/NotApplicable 区別・
fail-closed 集約・Shadow Mode)に忠実に実装する。
#333 の汎用分類器・契約文書は本実装時点で ruviform main 未着のため、
差異・整合の確認結果を PR 証拠として記録する(#333 §8 は製品固有
ルールを各 Consumer が所有すると規定しており、本表は AudioLens が
Owner となる)。

## Scope

### In Scope

- `tools/classify-impact.mjs`: PR diff → AudioLens 変更分類 →
  lane 期待集合(`required`/`not_applicable`)を出力する分類器。
  本設計 §2 の分類表を実装する唯一の正本
- `impact` job: PR file list 取得(`gh api`)+ 分類実行 +
  証拠テーブル出力。分類失敗・分類不能・非 PR event は Full CI
- `hygiene` job: `prek run --all-files` を verify から分離し常時
  実行化(audiolens-independent-ci §1 の不変条件を skip 可能化後も
  維持する)
- `verify`/`ui-verify`/`webview-verify` の selective `if:` と
  `needs: [ruviform-access, impact]`
- `summary` job の期待集合 fail-closed 再検査
  (required→success 必須、NotApplicable→skipped または実行済み
  のみ許容、分類出力欠落・impact 失敗は fail)
- Shadow Mode → 選択実行の移行機構
  (repo variable `CI_IMPACT_SELECTIVE=1`)、Shadow 偽陰性注記、
  Before/After 計測と Rollback

### Out of Scope

- OS leg 単位の選択実行(matrix leg 分割)。分類表は lane×3OS を
  1 単位として扱う。細粒度化は Shadow 実績後の Deferred
- 実 Window/OS GUI 自動試験(gui-real は従来どおり not_run 記録。
  Browser/headless の成功を実機 PASS とみなさない契約は不変)
- `Cargo.lock` 以外の依存 freeze/更新方針、Ruviform candidate SHA
  検証(ruviform #219/#294 の責務。本 CI は pin 済み resolved SHA
  の Consumer 回帰 gate)
- Ruleset required context の変更(下記 §5 で「変更しない」と
  定義する)

## Components and Boundaries

### 1. 実行モデル

```
ruviform-access ─┐                        (常時)
impact ──────────┤   分類→期待集合出力      (常時)
hygiene ─────────┤   prek --all-files     (常時)
  ├─ verify (3 OS)        if: impact.required or 分類失敗時
  ├─ ui-verify (3 OS)     同上
  ├─ webview-verify (3 OS)同上
  ├─ gui-real             not_run 記録(常時)
  └─ summary              期待集合 fail-closed 検査(常時, if: always())
```

- `pull_request`: `impact` が `gh api .../pulls/N/files` で diff を
  取得し分類。`CI_IMPACT_SELECTIVE=1` のみ NotApplicable lane を
  job skip。無設定(Shadow Mode)では分類を記録して全 lane 実行
- `push: main`・`workflow_dispatch`: `impact` が mode=full を出力、
  全 lane required = 従来どおり Full CI(契約上維持)
- lane `if:` は
  `!cancelled() && needs.ruviform-access.result == 'success' &&
   (needs.impact.result != 'success' || needs.impact.outputs.<lane> == 'required')`。
  分類自体の失敗・欠落は「分類不能 = 安全側 Full CI」として lane
  を実行させる(impact job 失敗時も lane が走る)。cancel は伝播を
  妨げないよう `!cancelled()` を併記する
- `ruviform-access` 失敗時は依存解決不能のため lane は skip
  (summary が access 失敗で fail するので gate 結果は不変)

### 2. AudioLens 変更分類表(唯一の正本は `tools/classify-impact.mjs`)

規則は上から順に first-match。分類不能・表に無い path は
`unclassified` = 全 lane required(契約の安全側)。

| class | path 規則 | verify | ui-verify | webview-verify | 根拠 |
|---|---|---|---|---|---|
| ci | `.github/**`、`.pre-commit-config.yaml` | R | R | R | CI 定義自身を全 lane で再検証 |
| cargo-lock | `Cargo.toml`/`Cargo.lock`(root・crates/* 含む) | R | R | R | ruviform resolved SHA・dep cone を変えうる。Consumer 回帰契約で全 lane 必須 |
| toolchain | `mise.toml`/`mise.lock`/`.cargo/**` | R | R | R | 共有 toolchain → Full CI |
| ui-vendor | `tools/sync-ruviform-ui.mjs` | – | R | – | vendor 再生成は ui-verify のみが消費 |
| docs | `docs/**`、`*.md`、`LICENSE`/`NOTICE` | – | – | – | 実行成果物へ影響なし(hygiene が検査) |
| ci-scripts | `tools/**`(上記以外) | R | R | R | CI/検証スクリプト・未使用ツールも安全側 |
| ts | `ui/**`(vendor 生成物含む) | – | R | – | cargo workspace 外。webview は runtime 利用のみで自動検証未接続 |
| lfs-golden | `.gitattributes`、`assets/`・`samples/`・`dictionaries/`・`baseline/`、他 LFS 拡張子 | R | – | – | Golden/LFS fixture は workspace test が消費。al-gui は crate 内 embed asset のみで共有 fixture を読まない |
| cli | `crates/al-cli/**` | R | – | – | Export 含む。al-gui 依存 cone 外 |
| observability | `crates/al-observability/**` | R | – | – | al-cli 経由のみ。al-gui cone 外 |
| gui | `crates/al-gui/**` | R | – | R | headless(workspace test)+ webview feature |
| rust-core | `crates/al-core/**` | R | – | R | 全 crate の基盤。al-gui cone 内 |
| service | `crates/al-job/**`、`crates/al-service/**` | R | – | R | al-gui cone 内(Export の service 側実装を含む) |
| analysis | `crates/al-analysis/**`、`crates/al-locate/**` | R | – | R | al-gui cone 内 |
| source | `crates/al-media/**` | R | – | R | source ingest。al-gui cone 内 |
| dataset | `crates/al-dataset/**` | R | – | R | al-gui cone 内 |
| testkit | `crates/al-testkit/**` | R | – | R | dev-dep で al-gui test cone 内 |
| docs-config | `.agents/` `.claude/` `.codex/` `.cursor/` `.windsurf/` `.serena/` `.sdp/` `.vscode/`、`.mcp.json` `apm.yml` `apm.lock.yaml` `AGENTS.md` `.gitignore` `.editorconfig` `.worktreeinclude` `biome.jsonc` | – | – | – | agent/repo 設定。実行成果物へ影響なし(hygiene が検査) |
| unclassified | 上記以外の全 path | R | R | R | 分類不能 → 安全側 Full CI |

依存伝播(al-gui 依存 cone に基づく webview-verify の
required/not_applicable):

```
al-core ── al-job ── al-media ── al-dataset ── al-locate ── al-analysis
                                                              │
                                    al-service ◄──────────────┘
                                       │
            ┌──────────────────────────┤
            │                          │
        al-cli (+observability)     al-gui (+ruviform-ui/viz/webview)
            │                          │
        verify のみ               verify + webview-verify
```

- al-gui は al-cli・al-observability に依存しないため、cli/
  observability 変更は webview-verify NotApplicable(根拠付き)
- webview feature 内分離(feature cfg は lib.rs 内に混在)は path
  分解不能のため GUI と WebView を同一 `gui` class として扱う
- `Cargo.toml`/`Cargo.lock` 変更は dep cone 自体を変えうるため
  crate 所属に関わらず全 lane required

Issue 本文の分類名との対応: Rust Core=rust-core、Analysis=analysis、
Source=source、Dataset=dataset、CLI=cli、Export=cli/service(実装が
al-cli `export` + al-service に分散)、GUI=gui、TS=ts、
UI vendor=ui-vendor、WebView=gui(feature 内分離不可のため統合)、
Cargo.lock=cargo-lock、CI=ci、LFS Golden=lfs-golden。

### 3. `hygiene` job(prek 分離)

audiolens-independent-ci §1 が「paths filter を置かない」根拠とした
不変条件 — **doc 変更を含む全ファイルを `prek run --all-files`
が検査する** — は、verify が skip 可能になると崩れる。
よって prek を verify から分離し常時実行 lane とする:

- `hygiene`(ubuntu): checkout(LFS 不要)→ scoped mise
  (`rust prek uv biome actionlint`)→ `prek run --all-files`
- expected set で常に required。hook 群(trailing-whitespace /
  EOF / yaml / toml / private key / windows names / biome /
  cargo fmt / actionlint)は全変更に対して意味を持つ
- verify から prek step と `prek uv actionlint` の tool install を
  除去(`biome` は `lint-ci` が使うため残留)。prek 検査の権威は
  hygiene に一本化(DRY)

### 4. `summary` 期待集合 gate(fail-closed)

required context `summary` は名称・常時実行(`if: always()`)を
維持し、検査を期待集合ベースへ拡張する:

- **常時 required**: `ruviform-access`・`hygiene`・`gui-real`
  (gui-real は not_run 証拠を出すことが期待動作。job 自体の
  skipped/failure は証拠欠落として fail)
- **分類依存 lane**: `verify`/`ui-verify`/`webview-verify` は
  `impact` の `<lane>` output と実 result を照合
  - `required` → `success` のみ許容。failure/cancelled/
    skipped/NotRun/Unsupported は fail(Pass 偽装禁止)
  - `not_applicable` → `skipped`(正当 NotApplicable、根拠は
    impact 証跡)または実行済み `success` を記録。実行済みの
    failure/cancelled は fail
  - 期待値の欠落・不正値(impact 失敗・output 欠落)→ fail-closed
- **Shadow 偽陰性注記**: classified=not_applicable の lane が実
  実行で failure なら `::warning::SHADOW FALSE-NEGATIVE` を出す
  (gate は lane 失敗で既に落ちる。分類表の見直し対象として明示)
- 集約表は lane×(classified/expected/result/reason)に拡張し、
  期待集合・実結果・対象外理由を PR 上で監査可能にする(#333
  契約 §4・§5)

### 5. Ruleset 移行の安全性(#147 との整合)

- required context は `summary` のみ(main-protect ruleset、
  2026-11-04 時点で確認)。名称・job の存在・`if: always()` を
  不変とし、**ruleset 変更不要 → 無保護期間ゼロ**
- lane job(verify 等)は required context ではないため、
  skip されても GitHub 上は `summary` のみが gate 判定に効く。
  「skipped が success 扱い」問題は summary の期待集合照合で
  構造的に排除される
- `impact`/`hygiene` は新規 check 名だが required context では
  ないため ruleset 更新は不要。summary が両者を期待集合に含めて
  検査する(impact 失敗・hygiene 失敗は fail)

### 6. Shadow Mode → 選択実行の移行

- `CI_IMPACT_SELECTIVE` が選択実行の制御スイッチ(kill switch と
  同じ `CI_*` variable convention)。二層で解決される:
  - **repo variable** `CI_IMPACT_SELECTIVE`: ops 側の優先
    override。設定されていれば workflow env 既定値を上回る
  - **workflow env**(`ci.yml` トップレベル `env:`): repo
    variable 未設定時の既定値。#152 導入時は事実上 `0`(Shadow)、
    #153 で `"1"` に引き上げて選択実行を既定化した
- **Shadow(=0)**: 分類は実行・記録されるが effective は全 lane
  required。分類表と Full CI 実結果を PR 上で突合し、
  「NotApplicable と分類された lane が実際は失敗する」偽陰性が
  無いことを確認する期間。偽陰性検知は §4 の warning が担う。
  有効化後も repo variable `CI_IMPACT_SELECTIVE=0` で任意時点に
  Shadow 相当へ復帰できるため監視経路は残る
- **有効化(#153 で実施)**: 当初は Shadow で偽陰性なし(目安:
  複数 PR・各主要 class を網羅)を確認後、Settings → Variables に
  `CI_IMPACT_SELECTIVE=1` を設定する ops 有効化を予定していた。
  macOS runner 逼迫による required lane の queue→cancel 実害を受け
  owner 判断で前倒しし、PR で有効化できる workflow env 経路を採用
  (有効化 PR は `.github/` = ci class で必然的に Full CI 検証
  され、`git revert` で恒久復帰できる)。以後の PR run から
  NotApplicable lane が skip される
- **Rollback**:
  - 即時(merge 不要): repo variable `CI_IMPACT_SELECTIVE=0` を
    設定 → 次回 run から Shadow 相当(分類記録のみ全 lane 実行)
    に復帰
  - 恒久: workflow env の `CI_IMPACT_SELECTIVE` 行を削除
    (`git revert`)→ 既定値が Shadow に戻る
- 有効化 PR 自身は ci/ci-scripts/docs class を含むため Shadow
  でも selective でも全 lane required = Full CI として走る
  (分類表の自己防御性: 分類器・workflow 変更は常に全検証)。
  実 lane skip(selective の実動作)は merge 後の後続 PR で確認する

### 7. 分類器の自己防御

- `tools/classify-impact.mjs`・`.github/**` の変更はそれぞれ
  ci-scripts/ci class → 全 lane required。分類規約の改変は必ず
  Full CI で検証される
- `gh api` 失敗・空 fetch → `--fallback` → mode=full。分類不能
  ファイル → unclassified → 全 lane required。二重の安全側倒れ
- 空 diff(0 ファイル)は正しく「全 lane NotApplicable」(変更が
  無いため検証するものが無い。hygiene/gui-real/summary は常時
  実行)

## 計測契約

- Before: run 41827679237(main push、sccache warm)で
  verify 713s・ui-verify 150s・webview-verify 609s・その他 ~9s、
  総 Runner ~1481s・wall ~327s(実測)
- After(Shadow): 本 PR run で impact + hygiene の追加 cost と
  各 lane の非回帰を記録(prek は verify→hygiene へ移動で
  +1 leg・verify から -1 step)
- After(選択実行有効化後): 代表 class の PR で lane skip による
  Runner 時間削減を ruviform `ci/metrics/collect-ci-metrics.mjs`
  同一手順で測定し IR/exp に追記
- 見込み(分類表からの推定): docs-only ≈ Runner -87%、
  cli/observability-only ≈ -40%、ui-only ≈ -77%、rust 共有 crate は
  ±0%(全 required)

## 失敗モードと無効化手順

| 失敗モード | 検知 | 無効化/復旧 |
|---|---|---|
| 分類規約の偽陰性(N/A が実は失敗) | Shadow warning・Full CI 失敗 | repo variable `CI_IMPACT_SELECTIVE=0` → 即時 Full CI(Shadow 相当)。env 行削除で恒久復帰。分類表を修正して再有効化 |
| 分類 job 失敗(api 障害等) | impact result≠success、summary fail | lane は fallback で全実行。rerun/恒久なら repo variable `=0` のまま運用可 |
| 選択実行の誤動作 | summary 期待集合表 | repo variable `CI_IMPACT_SELECTIVE=0` で即時 Full CI。env 行削除/`git revert` で完全除去 |
| hygiene lane 失敗 | summary fail(常時 required) | prek 違反を修正。分類とは独立 |

## Deferred Design Concerns

- OS leg 単位の選択実行(matrix leg 分割、ruviform #333 の lane/OS
  粒度に追従するか)— Shadow 実績と #333 着後に再評価
- `unclassified` の細分化(新規 top-level dir を都度 class 化)
- `gui-real` の opt-in lane 化(ruviform #312/#313 接続時。本設計
  では従来どおり not_run 証拠を維持)
- ruviform #333 の汎用契約文書・共通部品が main 着した際の整合
  (本表は AL 所有の製品固有規約。共通機構が提供されれば
  classify-impact.mjs を差し替え可能な入力/出力形を維持)

## Verification Notes

- [x] `mise exec -- actionlint .github/workflows/ci.yml`
- [x] `mise exec -- prek run --all-files`
- [x] `tools/classify-impact.mjs` を合成 diff で検証
      (cli/docs→V、al-core→VW、ui-only→U、Cargo.lock→全、
      未知 path→全、apm-only→N/A、shadow/full/fallback)
- [x] `summary` Enforce ロジックを stub 値で検証
      (required-skipped=fail・N/A-skipped=pass・
      N/A-failure=fail・impact 失敗=fail・偽陰性 warning)
- [ ] 本 PR の CI run で全 job green + impact 証跡出力を確認
- [x] 選択実行の有効化 — #153 で workflow env 既定値を `"1"` に変更
      (Shadow 初期実績を受けた owner 判断による前倒し。
      repo variable `=0` の即時復帰を維持し、偽陰性監視は継続。
      selective 実動作の確認は merge 後の後続 PR)

---
id: "DESIGN-T1zXGDg8nn70x2f6GwIeNc"
type: "design"
status: "approved"
title: "AudioLens 独立 CI: 3 OS lane・依存 dual-mode アクセス・証拠レポート設計"
created: "2026-11-04"
updated: "2026-11-05"
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
  implements:
    - "../specs/audiolens-independent-ci.md"
  implemented-by:
    - "../plans/implement-audiolens-independent-ci.md"
  depends-on:
    - "../designs/ruviform-dependency-ci.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "https://github.com/scarecrow173/audio-lens/issues/140"
    - "https://github.com/scarecrow173/audio-lens/issues/141"
    - "https://github.com/scarecrow173/ruviform/issues/294"
  refines:
    - "../designs/ruviform-dependency-ci.md"
  refined-by: []
  derives-from:
    - "../specs/audiolens-independent-ci.md"
    - "../designs/ruviform-dependency-ci.md"
  derived-by: []
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# AudioLens 独立 CI: 3 OS lane・依存 dual-mode アクセス・証拠レポート設計

## Context

`docs/designs/ruviform-dependency-ci.md` §10 が定義した `ci.yml` は
`workflow_dispatch` 限定・`RUVIFORM_DEPLOY_KEY` 必須の private
専用経路だった。同設計の Deferred「`pull_request`/`push` 自動
CI トリガ」は「credential 設定完了後に判断」とされていたが、
現在両 repo は public であり、匿名 HTTPS で Git 依存を取得
できる。#131/#132 以降は `ui/` TypeScript headless 統合層
(vendored ruviform `ui/{core,components,binding,viz}`)が存在し、
CI への明示 lane が未整備。本設計は旧 §10 を更新し、Deferred の
自動トリガ項目を解消する。

Ruviform 側 `consumer-verify.yml`(ruviform #294, PR #273/#274)が
確立した access mode 解決(deploy_key / anonymous / not_run)、
証拠テーブル、scoped mise install の convention を踏襲し、
重複 Runner を作らない。

## Scope

### In Scope

- `.github/workflows/ci.yml` の trigger 拡張(PR/push/dispatch)と
  lane 再構成
- Ruviform Git 依存アクセスの dual-mode 化(anonymous / deploy_key)
- mise ツールの CI scope 化と `mise.lock` platform entry 補完
- `tools/check-ruviform-rev.mjs`(resolved SHA 単一性 + UI vendor
  REV 整合ガード)と対応 mise task
- lane/OS/feature 単位の証拠レポート形式と Owner 切り分け指針
- `docs/guides/ruviform-private-access-ci.md` の dual-mode 追従

### Out of Scope

- 実 Window/OS GUI 自動試験(Ruviform #312/#313 が前提。本 CI
  では not-run を明示する placeholder のみ)
- `al-cli` Export 失敗の修正(#141)
- `Cargo.toml` manifest `rev` 撤去(#140)
- 新規 third-party action の導入(§セキュリティ参照)
- ブラウザ/IPC/Visual Golden Harness の AL 側 fixture 実装
  (Ruviform #310/#311/#314 が利用可能部品を提供次第、別 Issue で
  段階導入)

## Components and Boundaries

### 1. トリガと実行方針

```yaml
on:
  pull_request:
    branches: [main]
  push:
    branches: [main]
  workflow_dispatch:

concurrency:
  group: ${{ github.workflow }}-${{ github.ref }}
  cancel-in-progress: ${{ github.ref != 'refs/heads/main' }}
```

- PR: 同一 ref への追加 push で stale run を cancel(最新 commit
  のみ検証)。main push は cancel しない(merge ごとの検証履歴を
  保全する)
- paths filter は設けない。prek hygiene gate が doc 変更を含む
  全ファイルを検査するため、doc-only PR を skip すると既存違反が
  潜伏し後続 PR を阻害する。変更対象ごとの lane 分岐は job 構成で
  実現する。Issue #149 で `impact` 分類 + selective `if:` 導入後も、
  hygiene(prek)は常時実行の独立 lane として維持し、分類依存 lane
  の skip は summary の期待集合照合が fail-closed で管理する
  (詳細は impact-based-ci-issue-149.md)
- `permissions: contents: read` のみ。secret 参照は Deploy Key の
  みで job env 経由・ファイル化して渡し、ログへ出さない

### 2. Ruviform 依存アクセス(`ruviform-access` job)

`secrets` context は `if:` で使えないため、consumer-verify と同じ
preflight パターンで access mode を output として解決する:

| mode | 条件 | 経路 |
| --- | --- | --- |
| `deploy_key` | `RUVIFORM_DEPLOY_KEY` secret 設定済み | SSH + known_hosts 照合 + `url.insteadOf` 書換(旧 ci.yml/ガイドの手順を継承) |
| `anonymous` | secret 未設定かつ `git ls-remote https://github.com/scarecrow173/ruviform.git HEAD` が成功 | 認証なし HTTPS(最小権限) |
| `unavailable` | 両方不可 | 下流 job の gate step が `::error::` で明示失敗 |

- 鍵なしでの SSH 試行や、anonymous 失敗時の credential 推測など
  不正 fallback は行わない。probe 失敗と manifest/lock 失敗は
  probe step で切り分ける
- Deploy Key 手順(鍵ファイル mode 600、`api.github.com/meta` の
  SSH host key 照合、ruviform URL のみの insteadOf、終了時
  cleanup)は旧 ci.yml とガイドの構成を維持する
- fork PR では GitHub が secrets を供給しないため mode は
  anonymous へ落ちる。repo が private 復帰した場合は fork PR は
  unavailable → 明示失敗(検証不能を緑にしない)

### 3. mise ツール scope

workflow env に `MISE_ENABLE_TOOLS` を設定し、CI が使うツールのみ
activate する(`apm`・`pipx:serena-agent` を CI の `mise exec` /
`mise run` から除外)。install も `mise install --locked <tools>` で
scope する:

- `rust`(components rustfmt/clippy、1.99.0 pin): cargo 系全 lane
- `git-lfs`: fixture 実体復元(`git lfs pull` / checkout lfs)
- `node`: `tools/*.mjs`・`ui/` npm lane
- `prek`: hygiene gate(`prek run --all-files`)
- `uv`: prek の python 言語 hook env 構築(pre-commit-hooks repo)
- `biome`: `lint-ci` の `biome check`
- `actionlint`: prek の workflow 静的検証 hook

`mise.lock` の `actionlint` に platform entry が無く `--locked` が
全 platform で失敗していたため、本変更で `mise lock` により
7 platform の entry を補完する(checksum + GitHub release URL +
provenance)。`pipx:serena-agent` は backend 上 platform entry を
生成できず gap が残るが、CI scope 外のため別 Issue 扱いとし本
設計では記録に留める。

### 4. lane 構成(job 分割)

```
ruviform-access (ubuntu)            # access mode 解決(常時)
impact (ubuntu)                     # Issue #149: PR diff → 分類→期待集合(常時)
hygiene (ubuntu)                    # Issue #149: prek --all-files(常時)
  ├─ verify (ubuntu/windows)        # Rust 本体 + Golden + CLI(selective)
  ├─ ui-verify (ubuntu/windows)     # ui/ TS + vendor 整合(selective)
  ├─ webview-verify (ubuntu/windows) # al-gui webview feature(selective)
  ├─ gui-real (ubuntu)              # 実 GUI: not-run 記録 placeholder(常時)
  └─ summary (ubuntu)               # 証拠集約 + 期待集合 fail-closed(常時)

verify-macos / ui-verify-macos / webview-verify-macos (macos)
                                    # 非ブロッキング macOS 観察 leg
  └─ macos-summary (ubuntu)         # macOS lane 別 Pass/Fail/NotRun 記録
```

Issue #149 以降、`verify`/`ui-verify`/`webview-verify` は PR で
`impact` 分類が required のときのみ実行され得る(Shadow Mode 中は
全 lane 実行)。選択条件と分類表は
[impact-based-ci-issue-149](./impact-based-ci-issue-149.md) を参照。

**macOS leg の物理分離(Issue #149 2026-11-05 追加合意)**:
macOS runner 逼迫で Required `summary` が待機しないよう、各 lane
の macOS leg は `*-macos` job へ分離され `summary` の needs・判定
対象から外れる。PR/main では lane と同じ applicability で継続実行
され、結果は非ブロッキング check `macos-summary` が lane 別に
Pass/Fail/NotRun/Unsupported/Pending で記録する。step 本体は
`.github/actions/<lane>` composite action が必須 legs と共有する
単一定義。macOS legs を Ruleset Required context に登録しない。
macOS 対応を表明するリリース/互換受入では候補 SHA の macOS 実測を
別途必須とする(§4 末尾・guides/release-macos-verification.md)。

**verify**(ubuntu/windows matrix + `verify-macos`、同一定義は
`.github/actions/verify`): checkout(`lfs: true`)→ scoped mise install →
`verify-toolchain` → dep access 設定 + probe →
`cargo fetch --locked` → `check-ruviform-rev` →
`fmt` → `check-ci` → `lint-ci` →
`test-ci`(Golden/f64/Raw timing/Stream provenance/Stage resume/
Job/Dataset/Export の全回帰を含む、LFS fixture 使用)→
`cargo build -p al-cli --profile ci` → `check-cli-headless`。
(`prek run --all-files` は Issue #149 で `hygiene` job へ移管)

**ui-verify**(ubuntu/windows + `ui-verify-macos`、同一定義は
`.github/actions/ui-verify`): checkout(LFS 不要)→ `mise install --locked
node` → dep access 設定 + probe → `mise run sync-ui-deps`
(vendor を pin SHA で materialize)→ `mise run setup-ui`
(`npm ci`)→ `check-ruviform-rev --require-vendor`(vendor `REV`
== lockfile resolved SHA)→ `mise run check-ui`(sync `--check` +
tsc)→ `mise run test-ui`(happy-dom suite)。Ruviform #310/#311/
#314 の利用可能部品が増えた場合の UI integration smoke の受け皿を
兼ねる lane で、現行は AudioLens 側 headless 層が smoke 本体。

**webview-verify**(ubuntu/windows + `webview-verify-macos`、
同一定義は `.github/actions/webview-verify`): Rust lane のみ。ubuntu のみ
`libwebkit2gtk-4.1-dev libgtk-3-dev libsoup-3.0-dev
libjavascriptcoregtk-4.1-dev` を apt install(ruviform `ui-verify`
と同一構成)→ `check-ui-webview` / `lint-ui-webview` /
`test-ui-webview`。実窓起動は含まない(headless 境界内の
feature surface 検証)。

**gui-real**: 常時「not run」を記録する placeholder job。
Ruviform #312(test host)/#313(OS driver)が未整備のため、実
Window/OS 自動試験は本 run の検証範囲外であることを証拠として
残す。`status=not_run` を output し、summary が pass と誤認しない
よう明示する。#312/#313 接続時に opt-in lane へ置換する拡張点。

**macos-summary**: `needs: [impact, verify-macos, ui-verify-macos,
webview-verify-macos]` + `if: always()`(Issue #149 追加合意)。
macOS legs の結果を lane 別に集約する非ブロッキング check —
`summary` の needs に含まれないため成否・待機が merge gate を
阻害しない。leg が queue 滞留の間は check 自身が Pending のまま
残り(未実行を NotApplicable に変換しない)、実行済みの
failure/cancelled・`observe` 期待の NotRun は check を失敗にする
(Fail を隠さない)。

**summary**: `needs: [ruviform-access, impact, hygiene, verify,
ui-verify, webview-verify, gui-real]` + `if: always()`。job ごとの
`status`(pass/fail/not_run)・access mode・resolved SHA・OS・
feature を集約した証拠テーブルを `$GITHUB_STEP_SUMMARY` に出力し、
失敗の Owner 切り分け指針を表示する(§5)。全 lane not-run や
cancelled は run を失敗にし、unknown を pass に見せない。
Issue #149 以降は verify/ui-verify/webview-verify について
`impact` の期待値(required/not_applicable)と実 result を
fail-closed で照合する(required の skipped/failure/cancelled は
fail、根拠付き NotApplicable の skipped のみ許容)。
2026-11-05 合意以降、各 lane job は ubuntu/windows legs のみの
matrix — macOS legs(`*-macos` job)は needs に含まれず、
summary の緑は macOS PASS を意味しない。

### 5. 証拠レポート形式と Owner 切り分け

各 job の末尾(`if: always()`)に evidence step を置き、固定列の
テーブル行を出力する(consumer-verify と同型):

```
| lane | OS | feature | ruviform resolved SHA | 結果 | 備考 |
```

結果語彙: `pass` / `fail` / `not run` / `unsupported`。
job 名に OS・feature が入り、run 自体が branch/commit を保持する
ため、行は lane・結果・resolved SHA・失敗 step を記録する。

Owner 切り分け(summary job が常時表示):

- Rust check/lint/test 失敗 → Consumer(AudioLens)側の回帰として
  まず疑う。Ruviform pin 更新直後のみ失敗するなら Ruviform 側を
  疑い、cross-consumer 再実測は ruviform #294 に報告する
- `al-cli` Export 系の失敗 → 既知 Issue #141 を参照(本 CI は
  修正を担わない)
- dep access / fetch 失敗 → 依存取得経路の問題(repo visibility /
  Deploy Key 登録)としてガイドへ誘導
- ui-verify 失敗で REV 不一致 → `sync-ui-deps` 未実行または pin
  とのずれ(Consumer 側操作ミス)

### 6. Ruviform resolved SHA ガード(`tools/check-ruviform-rev.mjs`)

`cargo fetch --locked` 後に実行する Node スクリプト(既存
`tools/*.mjs` と同じ OS 非依存方針)。検証対象:

1. `Cargo.lock` の `scarecrow173/ruviform.git` source 全件が同一
   40 桁 SHA(`#<sha>` fragment)へ解決している。0 件・複数 SHA・
   形式不明は fail-closed
2. `Cargo.toml` の ruviform git 依存に manifest `rev` が存在する
   場合: 全 entry で同一 rev かつ lockfile SHA と一致。`rev` が
   全く無い場合は lockfile 主体モード(#140)として受理。一部だけ
   `rev` がある混在状態はエラー
3. `ui/vendor/ruviform/REV` が存在すれば lockfile SHA と一致。
   `--require-vendor` 指定時は REV 不在もエラー(ui-verify lane が
   sync 後に使用)

成功時は resolved SHA を標準出力と `$GITHUB_OUTPUT` の
`resolved_sha` へ出し、証拠行が採用する。`mise run
check-ruviform-rev` task を新設し、CI とローカルで同一入口にする。

### 7. 既存 task との非互換

- 新規追加は `check-ruviform-rev` task と `tools/*.mjs` のみ。
  既存 task(`test`/`check-ci`/`lint-ci`/`check-ui` 等)の意味は
  変更しない
- `#140` 進行中の `sync-ruviform-ui.mjs` 変更とはファイルを分け、
  manifest rev 有り/無しの両状態で本ガードが動く設計とする

## Data and Control Flow

```text
pull_request / push(main) / workflow_dispatch
  -> ruviform-access: mode = deploy_key | anonymous | unavailable
  -> impact: PR diff を分類 -> 期待集合(required/not_applicable)
     非 PR・分類不能・fetch 失敗 -> mode=full(全 required)
  -> hygiene: prek run --all-files(常時)
  -> verify / ui-verify / webview-verify (ubuntu/windows, 並列,
     PR かつ分類 required のときのみ実行。Shadow Mode は全実行)
  -> verify-macos / ui-verify-macos / webview-verify-macos
     (分類 observe のとき実行。summary 非依存)
  -> macos-summary: macOS lane 別 Pass/Fail/NotRun 記録(非ブロッキング)
       unavailable -> gate step ::error:: (明示 fail)
       deploy_key  -> SSH credential + probe
       anonymous   -> HTTPS probe
       -> cargo fetch --locked -> check-ruviform-rev
       -> lane steps (mise run ...)
       -> evidence step (if: always()) -> GITHUB_STEP_SUMMARY
  -> gui-real: not_run を記録
  -> summary: lane 表 + Owner 切り分け + 期待集合照合で fail
```

## Non-Functional Constraints

- 秘密情報: Deploy Key は env 経由でファイル化のみ。stdout/artifact
  へ出さない。third-party action に secret を渡さない
  (使用 action は `actions/checkout`・`actions/cache`・
  `jdx/mise-action` のみ、旧設計 §10 の方針を継承)
- 再現性: `mise install --locked` + `cargo fetch --locked`。
  mise.lock の platform entry gap は本変更で actionlint 分を補完、
  残 gap(serena-agent)は記録
- 緑の偽装禁止: not-run/ゼロ件を pass にしない。全 lane not-run
  または job cancelled は summary で fail

## Risks and Trade-offs

- `actionlint` の lockfile 補完は `mise lock` の標準出力で、
  upstream release の改竄時に checksum が追従する点は
  `--locked` 本来の挙動と同一
- fork PR が private repo で fail するのは意図した fail-closed。
  運用上の摩擦は public 維持で回避される
- webview-verify は wry/webkit2gtk ビルドで重いが、macOS leg を
  含め独立に結果を残す価値に見合う。apt 依存は ruviform 側と同一構成で
  上流実績あり
- macOS leg の分離は Required gate を 2OS に限定する代わりに、
  macOS の formal 互換判定をリリース Gate の実測(runbook)へ
  移す。PR 上で macOS の regressive 失敗は macos-summary check の
  失敗として可視だが merge は阻害しない — 見逃しリスクを受容
  しつつ runner 逼迫耐性を得るトレードオフ(2026-11-05 合意)
- doc-only PR も全 lane が走る(paths filter 不採用の対価)。
  concurrency cancel で実害は限定的。Issue #149 で hygiene を
  独立常時 lane 化し、分類依存 lane は期待集合ベースで選択実行
  可能となった(Shadow 検証後に有効化)

## Deferred Design Concerns

- 実 Window/OS GUI 試験 | Ruviform #312/#313 未整備 | 同 Issue 解決時 |
  gui-real placeholder を opt-in lane へ置換
- Browser Harness(#310)/IPC conformance(#311)/Visual Golden(#314)
  の AL fixture 導入 | 上流未提供 | Ruviform 側提供次第・別 Issue |
  ui-verify lane を拡張
- `pipx:serena-agent` の lockfile platform entry 欠落 | pipx backend
  が entry を生成しない | CI で必要になった時点(現状 CI scope 外)
  | `--locked` 全量 install は依然不可。別 Issue で記録
- `cargo vendor`/offline build | private access 前提で十分(旧設計
  から継続) | 配布・オフライン要件の発生時 | 同上

## Verification Notes

- [ ] `actionlint` が本 workflow を静的検証で通す
      (prek hook + ローカル実施)
- [ ] `node tools/check-ruviform-rev.mjs` が現在の manifest+lock
      で pass し、REV 一致を `--require-vendor` で確認する
- [ ] スクリプトの fail 経路(lock 0 件・SHA 分裂・manifest rev
      不一致・REV 不一致)が非ゼロ exit になることを unit 相当で
      確認する
- [ ] scoped tool list で `prek run --all-files` が通る
      (python hook の env 構築に uv が効くこと)
- [ ] PR 上の実際の CI 実行は merge 後の初回 PR/run で確認する
      (本 PR 上では e2e 検証不可。旧設計 §10 と同じ制約)

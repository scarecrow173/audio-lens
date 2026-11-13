---
id: "DESIGN-034WQJLvsRIXxWUeAMY3BF"
type: "design"
status: "approved"
title: "Issue #206 OS-L1 GPUI Desktop CI 統合: gpui-verify lane 設計"
created: "2026-11-13"
updated: "2026-11-13"
owners:
  - "scarecrow173"
relations:
  source:
    - "https://github.com/scarecrow173/audio-lens/issues/206"
  changes:
    added:
      - type: "file"
        path: ".github/actions/gpui-verify/action.yml"
    modified:
      - type: "file"
        path: ".github/workflows/ci.yml"
      - type: "file"
        path: ".github/actions/verify/action.yml"
      - type: "file"
        path: "tools/check-ci-workflow.mjs"
      - type: "file"
        path: "mise.toml"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "../specs/ci-os-l1-gpui-desktop.md"
  implemented-by:
    - "../plans/implement-ci-os-l1-gpui-desktop.md"
  depends-on:
    - "../designs/full-ci-ci-summary-issue-158.md"
    - "../designs/sccache-compile-cache-issue-148.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "../specs/ci-os-l1-gpui-desktop.md"
    - "../test-specs/ci-os-l1-gpui-desktop.md"
    - "../tasks/ci-os-l1-gpui-desktop.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/specs/ci-os-l1-gpui-desktop.md"
    - "docs/designs/full-ci-ci-summary-issue-158.md"
  derived-by: []
  verifies: []
  verified-by: []
  references:
    - "POLICIES.md"
  defers: []
  deferred-by: []
---

# Issue #206 OS-L1 GPUI Desktop CI 統合: gpui-verify lane 設計

## Context

`ci.yml` は workflow_dispatch 専用の Full CI で、lane は
`verify` / `ui-verify` / `webview-verify` の 3 系統 ×
(ubuntu+windows matrix / macOS leg)で構成され、`ci-summary`
が全 job を fail-closed 集約、`summary` が Ruleset Required
context 互換の macOS 非含有 gate として残る
(`docs/designs/full-ci-ci-summary-issue-158.md` が正本)。

`al-gui` の `gpui`/`gpui-test` feature(Issue #166 AL-G1〜G4)
と `al-desktop` bin は存在するが、CI lane がなく OS-L1
(Native Build/Link + headless test)の 3 OS 証拠がない。
`mise.toml` の `check-ui-gpui`/`lint-ui-gpui`/`test-ui-gpui`
は定義済みで CI 未接続。Ruviform は同じ GPUI adapter に対し
`ui-gpui-verify` lane を稼働済み(resolved SHA `fd1d4d5` の
ci.yml)で、Linux build deps・sccache・headless `gpui-test`
構成の先行実装がある。

## Scope

### In Scope

- 新 lane `gpui-verify`(ubuntu+windows matrix)と
  `gpui-verify-macos` の 2 job、および共有 composite
  action `.github/actions/gpui-verify/action.yml`。
- `ci.yml` の `ci-summary`/`summary` needs・証跡表・
  fail-closed 集約・Owner 切り分け・top comment の更新。
- `tools/check-ci-workflow.mjs` の構造契約更新
  (EXPECTED_JOBS・LANES・CISUMMARY_NEEDS・SUMMARY_NEEDS)。
- `mise.toml` への `build-ui-gpui` task 追加
  (`al-desktop` bin の Native Build/Link 入口)。
- `verify` lane からの `check-full` step 撤去
  (`lint-full` の clippy が同等の型診断を包含するため。
  計測証拠は下記「check-full/lint-full 重複の整理」)。

### Out of Scope

- `crates/` 実装、`gpui`/`gpui-test` feature 定義、test 内容。
- 実 OS Window の GUI-L2 検証、Xvfb offscreen pixel lane
  (Ruviform `ui-gpui-offscreen-verify` 相当は AudioLens では
  Deferred)。
- 旧 `webview` lane / `webview-verify` の変更・撤去(#189)。
- `daily-ci.yml`・tracker 系 workflow・`mise.ci.toml`・
  `tools/check-ci-tracker-workflows.mjs` 等。

## Components and Boundaries

- `.github/actions/gpui-verify/action.yml`(新規): lane 本体の
  単一定義。`webview-verify/action.yml` と同型 — access gate
  → mise → OS 依存 → cargo cache → sccache → ruviform access
  解決/probe → `cargo fetch --locked` → `check-ruviform-rev`
  → GPUI 検証 step 群 → 証跡記録。inputs は `os` /
  `access_mode` / `deploy_key` / `sccache_disable`、output は
  `resolved_sha`(guard step 由来)。
- `ci.yml`: `gpui-verify`(matrix.os = ubuntu-latest,
  windows-latest)と `gpui-verify-macos`(macos-latest)を追加。
  needs/if は他 lane と同一 `[ruviform-access, expected-sha]`
  + 両 success 条件。`ci-summary` needs に両 job を追加し
  証跡表・fail-closed pair・resolved SHA fallback 連鎖・
  Owner 切り分けを更新。`summary`(Required 互換・macOS 非含有
  scope)には `gpui-verify` のみ追加し、`gpui-verify-macos` は
  `ci-summary` のみが集約する(#157 の性質維持)。
- `tools/check-ci-workflow.mjs`: job 集合を 14 job に更新し、
  LANES・CISUMMARY_NEEDS・SUMMARY_NEEDS を新 lane 構成へ
  合わせる。契約コメントも同期。
- `mise.toml`: `[tasks.build-ui-gpui]` を追加
  (`cargo build -p al-gui --features gpui --bin al-desktop`)。
  既存 `check-ui-gpui`/`lint-ui-gpui`/`test-ui-gpui` と同じ
  feature lane 規約(--locked/--profile なし、dev profile)。
- `.github/actions/verify/action.yml`: `Type-check workspace
  (mise run check-full)` step を撤去し、証跡表から check-full
  行を除去(型診断は `lint-full` の
  `cargo clippy --workspace --all-targets` が包含)。

## Data and Control Flow

1. `expected-sha`/`ruviform-access` が成功した run のみ
   `gpui-verify*` が起動(lane `if:` 条件、他 lane と同一)。
2. action 内で ruviform access mode を gate し、deploy_key
   または anonymous で `cargo fetch --locked` →
   `check-ruviform-rev` が resolved SHA を出力。
3. `check-ui-gpui`(cargo check --all-targets)→
   `lint-ui-gpui`(clippy -D warnings)→ `test-ui-gpui`
   (cargo test、gpui-test feature の headless parity/
   scenario test)→ `build-ui-gpui`(al-desktop bin build)
   を順に実行。いずれか失敗で job 失敗 → `ci-summary` が
   fail-closed で run を失敗にする。
4. 証跡は各 action の `Record lane evidence` が
   `$GITHUB_STEP_SUMMARY` に lane×OS×feature×SHA×結果の
   表を出力し、`ci-summary`/`summary` が集約する。

## GPUI lane の OS 別構成

- **ubuntu-latest**: Ruviform `ui-gpui-verify` と同一の apt
  群を導入 — `pkg-config libxcb1-dev libxkbcommon-dev
  libxkbcommon-x11-dev libx11-dev libxi-dev libxtst-dev
  libwayland-dev wayland-protocols libfontconfig1-dev
  libfreetype-dev libvulkan-dev libegl-dev libgl1-mesa-dev
  mesa-vulkan-drivers libvulkan1`。`gpui-test` の headless
  render は software Vulkan(lavapipe)を使うため runtime
  deps も必須(GPU/実 Window は使わない)。
- **windows-latest**: 追加 deps なし。MSVC toolchain で
  Native Link を検証。gpui build.rs の HLSL shader compile
  は `debug_assertions` 下(dev/ci profile)では skip される
  ため fxc.exe は不要。
- **macos-latest**: 追加 deps なし。build.rs が dispatch
  binding(bindgen)・cbindgen・`xcrun metallib` を実行する
  ため Xcode CLT が必要だが macos-latest 標準で充足。
  `gpui-test` は TestAppContext の headless platform で
  動作し実 Window は開かない。

## check-full/lint-full 重複の整理(証拠)

`verify` lane は `check-full`(`cargo check --workspace
--all-targets --profile ci`)と `lint-full`(`biome check` +
`cargo clippy --workspace --all-targets --profile ci --
-D warnings`)を連続実行していた。clippy-driver は rustc
frontend を包含するため、`cargo clippy --all-targets` は
`cargo check --all-targets` と同一の型診断を全 target に
対して実行した上で clippy lint を追加する — check-full の
撤去で失われる診断はない。

計測(run 43348460663、verify (ubuntu-latest) leg、
SHA d6bf722 — task 名は当時 check-ci/lint-ci/test-ci、
構成は同一):

| step | duration |
|---|---|
| fmt | 0.8s |
| check-ci(cargo check) | 13.1s |
| lint-ci(biome+clippy) | 6.8s |
| test-ci(cargo test) | 89.3s |

check→lint で workspace member の check/clippy は別
fingerprint のため二重 compile であり、dependency の
check artifact は共有される。check-full 撤去後は
lint-full が dependency check を自前で行うが、sccache
(GHA backend)が compile を吸収し、節約分は workspace
member の `cargo check` pass 分(lane あたり数秒〜十数秒、
6 verify leg で累積)。`test-full` が全 target を codegen
込みで compile するため build 成功の下限保証も残る。

判断: `verify` lane から `check-full` step を撤去する。
`mise run check-full` task 自体と `verify-full` bundle は
不変(ローカル全体検証の入口として維持)。

## Non-Functional Constraints

- fail-closed: 新 lane の失敗/skip/cancel は `ci-summary`
  で run 失敗。`summary` は macOS 非含有 scope を維持。
- headless 誠実性: `gpui-test` は OS surface を stub する
  test platform であり、実 Window 操作の Pass を主張
  しない。証跡表・`gui-real` not_run 記録は既存のまま。
- 既存 lane・証跡(Golden/LFS/Export/CLI、headless TS、
  webview、Ruviform SHA)は不変。
- CI 時間: sccache・cargo cache を既存 lane と同構成で
  適用し、`check-full` 撤去で verify leg を短縮。

## Risks and Trade-offs

- **macOS での gpui build/test 未実績**: Ruviform は
  macOS leg を持たない。macos-latest での初回実走で
  build 失敗の可能性 → 手動 Full CI で実測し、失敗時は
  原因を切り分けて報告(本 lane は required のため
  fail-closed で顕在化するのが正しい挙動)。
- **Linux deps の過不足**: Ruviform と同一 gpui 0.2.2・
  同一 apt 群を採用。audio-lens 側依存差分は
  `ruviform-ui-gpui` feature 経由のみで追加 native deps
  なし。
- **check-full 撤去の診断欠落**: clippy が型診断を包含
  するため理論上なし。万一 clippy 特有の見逃しがあっても
  `test-full` の compile が下限を保証。
- **job 増加による CI 時間/コスト**: GPUI dependency 群は
  重いが feature lane のため verify lane と cache
  namespace を分離(`-cargo-ui-gpui-`)し、sccache で
  compile を共有可能にする。
- **libc pin (実装時に発見・解消済み)**: `zed-async-tar`
  経由の `xattr 0.2.3` が `libc::ENOATTR` を要求するが、
  libc 0.2.191 は Linux (`unix/linux_like/`) から同定数を
  撤去済みで Ubuntu leg の `check-ui-gpui` が E0425 で
  失敗した (run 43421012773)。Ruviform の lock は
  `libc 0.2.189` で同一 xattr/zed-async-tar/gpui 構成が
  稼働しているため、`cargo update -p libc --precise
  0.2.189` で Cargo.lock のみを揃えた (Cargo.toml 変更
  なし)。`x86_64-unknown-linux-gnu` target への
  `cargo check` で xattr が解決することを確認
  (ring/libz-sys 等の C cross toolchain 不在は Windows
  host 固有の環境制約であり本件と無関係)。恒久対応は
  上流 (zed-async-tar → xattr 1.x 系) の更新待ち。

## Deferred Design Concerns

- 実 OS Window / offscreen pixel の自動検証: AudioLens の
  GUI-L2 要件が発生した時点で別 Issue(Ruviform
  `ui-gpui-offscreen-verify` が先行事例)。
- `summary` job の撤去: `ci-summary` の Required 化後の
  最終段階(#158 design の延期項目)であり本件の対象外。

## Verification Notes

- [x] `mise exec -- actionlint` が ci.yml を受理する。
- [x] `mise run check-ci-workflow` が更新後の構造契約で
  exit 0。
- [x] 手動 Full CI(`gh workflow run ci.yml --ref <branch>
  -f expected_sha=<sha>`)で 3 OS の gpui-verify 系 job が
  success となり、`ci-summary`/`summary` が pass する。
  実測: run 43422571711 (SHA 7e5412d) — gpui-verify
  ubuntu 10m04s / windows 10m14s / macos 9m42s 全て
  success、ci-summary/summary も success。

## 追補（2026-11-14）：LintのOS間重複を解消

上記の`check-full`/Clippy重複最適化は同一OS・同一対象についての判断。
現在のCIでは静的Lint・整形（`biome check`、`cargo clippy`、`cargo fmt -- --check`）をLinuxに集約し、
Windows/macOSでは`check-full`によるネイティブ型チェックと`build-full`によるBuild/Link、
`test-full`およびCLIのNative Build/Linkを維持する。
`gpui-verify`・`webview-verify`のFeature専用ClippyもLinuxのみとし、
各OSのFeature専用check/testと`al-desktop`のBuild/Linkは変更しない。
各Composite Actionの結果表はLinux専用の未実行を明示しており、OS-L1の実行結果には転用しない。

元の計測・判断を削除せず、現行CIとの相違のみを記録する。

---
id: "DESIGN-034P5eOr1bEaMgiImL3DHd"
type: "design"
status: "approved"
title: "Ruviform Git 依存・SHA 固定・Private repo CI アクセス設計"
created: "2026-10-31"
updated: "2026-10-31"
owners: ["scarecrow173"]
relations:
  source:
    - "https://github.com/scarecrow173/audio-lens/issues/99"
    - "https://github.com/scarecrow173/audio-lens/issues/95"
  changes:
    added:
      - type: "doc"
        path: "docs/guides/ruviform-private-access-ci.md"
      - type: "doc"
        path: "docs/tasks/ruviform-dependency-ci.md"
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "../specs/ruviform-foundation-migration.md"
  implemented-by: []
  depends-on:
    - "../adr/ruviform-foundation-migration.md"
    - "../designs/ruviform-migration-responsibility-map.md"
    - "../plans/ruviform-foundation-migration.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related: []
  refines: []
  refined-by: []
  derives-from:
    - "../specs/ruviform-foundation-migration.md"
    - "../adr/ruviform-foundation-migration.md"
    - "../designs/ruviform-migration-responsibility-map.md"
  derived-by: []
  verifies: []
  verified-by: []
  references:
    - "https://github.com/scarecrow173/ruviform/issues/150"
    - "https://github.com/scarecrow173/ruviform/issues/151"
    - "https://github.com/scarecrow173/ruviform/issues/152"
    - "https://github.com/scarecrow173/ruviform/issues/153"
    - "https://github.com/scarecrow173/ruviform/issues/154"
    - "https://github.com/scarecrow173/ruviform/issues/155"
    - "https://github.com/scarecrow173/ruviform/issues/156"
    - "https://github.com/scarecrow173/ruviform/issues/157"
    - "https://github.com/scarecrow173/ruviform/pull/158"
  defers: []
  deferred-by: []
---

# Ruviform Git 依存・SHA 固定・Private repo CI アクセス設計

## Context

Issue #99。移行追跡 Issue #95 と移行計画
`../plans/ruviform-foundation-migration.md` の段1ゲートとして、
Ruviform の各 crate を **検証済みの同一 Git commit** から再現可能に
参照するための依存・toolchain・CI 認証の設計を固定する。

なお `depends-on`/`derives-from`/`implements` の参照先は AL#97 の
成果物であり、本設計時点では未マージのブランチ
(`al97-migration-adr` worktree)に存在する。relation は #97 の
ドキュメントが main に入った後の正本パスを指す。

前提事実(本設計時点で `gh api`/`git ls-remote`/各 manifest を
直接確認して検証済み):

- Ruviform は `scarecrow173/ruviform` Private リポジトリ。
  default branch `main`、本設計で固定する commit は
  `73190d7d26b3197b1da36749798b5952499d1b70`(2026-10-30、
  PR #158 merge 後の main HEAD)。
- Ruviform workspace は `publish = false`・`license = "MIT"`(暫定)・
  `rust-version = "1.89"`・edition 2024。mise pin は rust 1.99.0。
  対象 8 crate の manifest を同 SHA で確認済みであり、
  `ruviform-media-io` は `ruviform-io-symphonia` を依存に持たない
  (backend は別 crate として分離されている)。submodule は無し。
- AudioLens 側は PR #108(48803eb)で rust 1.99.0 に揃え済み。
  本 Issue 起票時点の「AL 1.98.1 / RV 1.99.0」差異は解消済みだが、
  解決した toolchain が常に選ばれることを明示検証する機構は無い。
- AudioLens には GitHub Actions CI が未導入(`.github/workflows/` 未追跡)。
  Ruviform 側は `ci.yml`(workflow_dispatch 限定)を持つ。
- `al-media` は symphonia `0.5.5` を直接依存に持つ。
  `ruviform-io-symphonia` は symphonia `0.6`(default features)。
- `al-cli` は `al-viz` に依存していたが、#106 で `al-viz` 撤去・
  `StageKind::Export` の al-dataset 再配線が完了し、この依存と
  静的 `report.html`/`report.data.js` 埋め込みは解消済み。
- AudioLens 側も Git LFS を使用(`*.mp4`/`*.png`/`*.jpg`)。
  `al-media` のテストは LFS 管理の MP4 fixture を直接読むため、
  CI checkout で LFS 実体の復元が必須。
- Ruviform 側も LFS 使用(`*.wav`/`*.flac` 等の音声 fixture)。
  cargo の git checkout が ruviform の LFS 実体を引かないよう
  `GIT_LFS_SKIP_SMUDGE=1` を cargo 経路に設定する(依存ビルドに
  fixture は不要)。
- 開発者ローカルでは Git Credential Manager が GitHub HTTPS 認証を
  保持しており、system git で private repo の fetch が成功する
  (`git ls-remote` 確認済み)。cargo 組み込みの libgit2 転送は
  credential helper を使わないため、private HTTPS Git 依存には
  `net.git-fetch-with-cli` が必要。
- mise の inline `run` task は Windows では `cmd /c` で実行される。
  `grep` 等の POSIX テキスト処理に依存する task は Windows で
  動かないため、新設する検証 task は node(既に toolchain に
  `node = "26.12.1"` として pin 済み)で実装する。

## Scope

### In Scope

- 必要な Ruviform crate の選択と、同一 commit SHA への `rev` 固定、
  Cargo.lock による再現性の確立。
- cargo の private Git 取得設定(`.cargo/config.toml`)。
- mise task の CI 語彙整備(check / *-ci / verify-toolchain /
  check-cli-headless / verify-ci)と `[profile.ci]`。
- CI workflow(`.github/workflows/ci.yml`)と private repo アクセスの
  最小権限 credential 設計・手順書、actionlint による workflow 検証。
- リリース/署名/ライセンスの依存解決方針の記録(publish・license・
  Symphonia 版差・vendor 不採用)。
- GUI/CLI 依存分離ポリシー・記録済み例外・機械的 guard。
- 基礎型の import smoke-test。

### Out of Scope

- 解析・メディア I/O の実装差し替え(AL#100/#101/#102 が担当)。
- `ruviform-io-symphonia` の導入(R#154 backend 未確定。
  Symphonia 0.5/0.6 混在を避けるため本 Issue では追加しない)。
- GUI アプリ本体(AL#105)と Ruviform GUI 系 crate(#156/#157)の導入。
- `al-cli` → `al-viz` 依存の撤去(`StageKind::Export` の再配線を
  含む #106 のスコープ)。
- `cargo vendor` による ruviform ソースの AL リポジトリ内複製。
- pull_request / push 自動トリガの有効化判断(credential 存在後の
  別途決定事項。後述)。
- `al-service`/`al-gui` の新設(#105 の詳細設計)。

## Components and Boundaries

### 1. 依存する Ruviform crate の選定

責務マップの切替対象行に対応し、後続 Issue が消費する crate のみを
選択する。`ruviform-io-symphonia` 等の backend・plugin・engine 系、
GUI 系(#156/#157 未設計)、`ruviform-rt`(ruviform-exec の推移依存
としてのみ入る)は直接依存に加えない。

| AL 側 crate(依存の置き場所) | 追加する Ruviform crate | 消費する Issue | 依存種別 |
| --- | --- | --- | --- |
| `al-media` | `ruviform-media-core`・`ruviform-audio-core`・`ruviform-video-core`・`ruviform-media-io` | AL#100(メディア I/O 切替) | `[dependencies]`(切替境界の本番依存) |
| `al-locate` | `ruviform-dsp`・`ruviform-math` | AL#101(相関/FFT 等カーネル呼出切替) | `[dependencies]`(同上) |
| `al-analysis` | `ruviform-audio-analysis`・`ruviform-dsp` | AL#101(STFT・帯域系切替) | `[dependencies]`(同上) |
| `al-job` | `ruviform-exec` | AL#54(OfflineDriver 適合性の**評価**) | `[dev-dependencies]`(採否未定の評価範囲のため shipped 依存にしない) |

- `al-cli`・`al-core`・`al-dataset`・`al-testkit`・
  `al-observability` には Ruviform 直接依存を追加しない。
- `#100`/`#101` 向けの dep は将来の消費場所に先行配置する段階導入
  (本 Issue では smoke-test で import のみ検証)。`ruviform-exec`
  は #54 が採否を決める評価対象のため dev-dep に留め、採用決定時に
  `[dependencies]` へ昇格させる。
- 固定 SHA 時点で確認済みの smoke-test 対象 API:
  `ruviform_media_core::{MediaTime, TimeBase, TimeBase::COMMON_90K,
  StreamDescriptor, StreamId, StreamKind}`、
  `ruviform_audio_core::{AudioBuffer, BufferSpec, ChannelLayout,
  SampleRate}`、
  `ruviform_video_core::{VideoFrame, PixelFormat}`、
  `ruviform_media_io::{CodecId, CodecFormat, CodecDescriptor,
  Capabilities, Packet}`、
  `ruviform_math::dsp::SimdToken::detect()`、
  `ruviform_dsp::{Window, BiquadDesign, Fft}`、
  `ruviform_audio_analysis::LevelMeter::new(channels)`、
  `ruviform_exec::{OfflineConfig, CancellationToken, Progress}`。
  各 crate は `version()` も公開している。

### 2. SHA 固定と再現性

- 全 Ruviform 依存はルート `Cargo.toml` の `[workspace.dependencies]`
  に集約し、`git = "https://github.com/scarecrow173/ruviform.git"` +
  `rev = "73190d7d26b3197b1da36749798b5952499d1b70"` で参照する。
  各 crate の `rev` は全て同一値とし、更新時は一括で揃える
  (entry 先頭のコメントで規約を明記)。
- member crate 側は `<name>.workspace = true` で消費するため、
  feature/SHA が crate 間で分岐しない。
- Cargo.lock は commit 対象(現行どおり)。git 依存は
  `source = "git+...?rev=<sha>#<sha>"` で記録され、`--locked` で
  lockfile に記録された同一 commit が取得・検証される(取得には
  ネットワークと private repo 認証が必要。`--locked` は取得自体を
  オフライン化しない)。
- SHA の更新は移行の節目で意図的に行い、本設計・変更履歴で
  追跡可能にする(dependabot 等による自動 bump は行わない)。
- pin した SHA は ruviform `main` 上で到達可能に保つ運用を
  手順書に明記する(force-push/rebase で pin が unreachable に
  なった場合は rev の一括更新で対応する)。
- `cargo vendor` は採用しない。ruviform ソースを AL リポジトリへ
  複製すると二重管理となり、MIT ライセンス表記の保持・更新追従の
  責務も発生する。再現性は lockfile + rev 固定 + private access
  credential で担保する。

### 3. cargo の private Git 取得

`.cargo/config.toml`(commit 対象、秘密情報を含まない):

```toml
[net]
git-fetch-with-cli = true

[env]
# cargo の git checkout が依存 repo の LFS fixture を引かないように
# する。ビルドに必要なのは crate ソースのみ。
GIT_LFS_SKIP_SMUDGE = "1"
```

- cargo の Git 取得を system git に委譲し、ローカルでは Git
  Credential Manager、CI では後述の credential 設定がそのまま
  効く。libgit2 転送では private HTTPS repo を認証できないため必須。
- ローカル開発者は repo へのアクセス権を既に持つため追加設定不要。

### 4. CI アクセス設計(最小権限 read-only)

**採用: Ruviform 側の read-only Deploy Key**(SSH、単一 repo・読取
のみ・取り消し容易・発行が API で完結する)。

- 手順(運用者が実施。詳細は `docs/guides/ruviform-private-access-ci.md`):
  1. `ssh-keygen -t ed25519 -C "audio-lens-ci-ruviform-read"` で
     専用鍵ペアを生成(passphrase なし。CI 専用鍵であり他用途と
     分離する。comment には用途名のみを入れ、repo URL は含めない)。
  2. 公開鍵を `scarecrow173/ruviform` の Deploy keys に **read-only
     明示**で登録:`gh api repos/scarecrow173/ruviform/keys
     -f title="audio-lens-ci-read" -f key="<pubkey>" -F read_only=true`。
     登録後に `gh api repos/scarecrow173/ruviform/keys` で
     `read_only: true` を確認する。
  3. 秘密鍵を `scarecrow173/audio-lens` の Actions secret
     `RUVIFORM_DEPLOY_KEY` に登録(`gh secret set`)。
  4. CI workflow 側は third-party action を使わず、鍵ファイル +
     `GIT_SSH_COMMAND` で完結させる(秘密情報を外部 action に
     渡さない):
     - secret を `RUNNER_TEMP` 配下に mode 600 の鍵ファイルとして
       展開する(env 経由のみ。内容を echo/log しない)。
     - GitHub 公開の host key(`gh api /meta` の `ssh_keys`、
       認証不要で取得可)を `known_hosts` に書き出し、
       `StrictHostKeyChecking=yes` で検証する(TOFU ではなく
       公開値と照合)。
     - `GIT_SSH_COMMAND="ssh -i <key> -o IdentitiesOnly=yes
       -o UserKnownHostsFile=<known_hosts>
       -o StrictHostKeyChecking=yes"` を `GITHUB_ENV` に設定。
     - `git config --global
       url."git@github.com:scarecrow173/ruviform.git".insteadOf
       "https://github.com/scarecrow173/ruviform.git"` で
       Cargo.toml の HTTPS URL のみを SSH に書換える。
  5. 認証設定直後に `git ls-remote
     git@github.com:scarecrow173/ruviform.git HEAD` の probe step を
     置き、fetch 失敗時の原因切り分けを容易にする。
- 秘密鍵はコード・ログ・コミットに出さない。workflow 内では
  `secrets.*` 経由のみで、echo/dump/artifact 化しない。
- 代替案(不採用だが記録):
  - **fine-grained PAT**(Contents: Read、ruviform のみ): HTTPS で
    そのまま使え SSH 設定が不要。発行は Web UI 手動で自動化
    しにくく、有効期限管理が必要。Deploy Key が使えない環境向けの
    代替として手順書に併記する。
  - **GitHub App + actions/create-github-app-token**: 短命 token で
    監査性は最も高いが、App 作成・インストールの初期コストと
    third-party action の追加が大きい。将来の監査要件次第で移行可能。
  - **third-party ssh-agent action**(webfactory/ssh-agent 等):
    鍵を外部 action に渡す供給網上の信頼辺が増えるため採用しない。
    Windows runner の msys/Win32-OpenSSH agent 混在問題も回避できる。
- `secrets.GITHUB_TOKEN` は発行 repo(audio-lens)にのみスコープ
  されるため ruviform を読めず、要件を満たさない。個人の OAuth
  token は repo を跨ぐ広いスコープを持つため利用しない。

### 5. toolchain の明示検証

- `mise.toml` の `rust = "1.99.0"`(components: rustfmt, clippy)を
  正とする。Ruviform の pin と同一であり、`rust-toolchain.toml` は
  新設しない(mise が唯一の toolchain 定義であり、二重定義を避ける)。
- `[tasks.verify-toolchain]` を新設し、`tools/verify-toolchain.mjs`
  (node スクリプト、3 OS 共通)が `rustc`/`cargo`/`rustfmt`/
  `cargo-clippy` の解決バージョンが `mise current rust` の pin と
  一致することを検証する。バージョンをソースへ二重記述しない
  (pin の正本は `mise.toml`/`mise.lock` のみ)。
- `mise run`/`mise exec` 配下では mise の tool dir が PATH 先頭に
  来るため本チェックは主に (a) component 欠落、(b) `mise exec` を
  介さずに cargo を呼ぶ CI step/ローカル手順の検出、(c) 将来の
  toolchain override 機構の混入検知 を担う。CI では
  `mise install --locked` 直後に実行する。
- CI 内の全 cargo 呼出は `mise run <task>` または
  `mise exec -- cargo ...` 経由とし、runner 同梱の別 toolchain が
  使われないことを保証する。
- Ruviform 側 rust-version(MSRV 1.89)< AL pin 1.99.0 であり、
  依存解決上の矛盾は無い。**AL 側は rust-version(MSRV)を宣言
  しない**ことを本設計で決定する: pre-release で公開もせず、
  実効 toolchain は mise pin が担う。未検証の MSRV 宣言は嘘に
  なるため、MSRV matrix が必要になった時点で再評価する
  (Deferred 参照)。

### 6. mise task 定義と CI profile

- `[workspace.package]` 直下の `[profile.ci]`(ruviform と同一
  convention)を `Cargo.toml` に追加:`inherits = "dev"` +
  `debug = "line-tables-only"` + `incremental = false` +
  `[profile.ci.package."*"] debug = "none"`。CI 成果物は
  `target/ci/` に分離される。
- 新設 task(Windows では `cmd /c` 実行のため、全て単一
  コマンド呼出として記述できる形にする):
  - `check` = `cargo check --workspace --all-targets`(新設。
    `check/test/lint/build` の契約を揃える)
  - `check-ci`/`lint-ci`/`test-ci`/`build-ci` = 各対応 task と
    同一の検査を `--profile ci` + `CARGO_INCREMENTAL=0` で実行
    (lint-ci は lint と同様 `biome check` + clippy の 2 コマンド)
  - `verify-toolchain` = `node tools/verify-toolchain.mjs`
  - `check-cli-headless` = `node tools/check-headless-deps.mjs`
  - `verify-ci` = `fmt` → `check-ci` → `lint-ci` → `test-ci` の
    順次実行(ローカルで CI と同等の検証を回す入口)
- 既存 `fmt`(`cargo fmt --all -- --check`)は既存 task doc が
  check 動作を契約として参照しているため semantic を変えず、
  `fmt-check` は新設しない。CI は `mise run fmt` を呼ぶ。

### 7. GUI/CLI 依存分離

- ポリシー:`al-cli`・`al-core`・`al-math`・`al-job`・`al-dataset`・
  `al-media`・`al-locate`・`al-analysis`・`al-testkit`・
  `al-observability` は GUI/WebView/GPU/ウィンドウ系 framework
  crate を(normal・build 依存として)持たない。将来の Ruviform
  GUI backend(#156/#157 系)やフロントエンド資産は `al-gui`(#105)
  側の optional deps/別 binary に隔離する。
- **解消済みの旧例外**: `al-cli` はかつて `al-viz` に依存し、
  `al-viz` は静的 `report.html`/`report.data.js` をバイナリに
  埋め込んでいた。#106 の `StageKind::Export` 再配線(`al-viz`
  撤去・export を al-dataset 実装へ)でこの例外は解消済み。
- 機械的 guard:`tools/check-headless-deps.mjs` が対象 crate ごとに
  `cargo tree -p <crate> --edges normal,build --prefix none` を実行し、
  出力の crate **名完全一致**で GUI/WebView/GPU 系外部 crate
  denylist(`wry`/`tao`/`winit`/`wgpu`/`wgpui`/`tauri`/`tauri-*`/
  `gtk`/`gtk4`/`gdk*`/`qt`/`eframe`/`egui`/`iced`/`slint`/`fltk`/
  `druid`/`webview2-com`/`webkit2gtk` 等)と照合する。
  `windows-sys`/`windows-targets` 等の OS API crate(memmap2・
  audio_thread_priority 由来)は denylist に含めず誤検出しない。
  substring ではなく完全一致で判定する。
- 3 OS の CI matrix で各 OS 固有の cfg 依存も網羅する
  (target-gated な GUI dep は当該 OS の lane で検出される)。

### 8. リリース/署名/ライセンスの依存解決

- Ruviform は全 crate `publish = false`・version `0.1.0`(仮置き)・
  license `MIT`(暫定)。crates.io 非公開であり Git 依存が唯一の
  導入経路。
- AudioLens 側 `workspace.package` に `publish = false` を追加し、
  **かつ全 member crate の `[package]` に `publish.workspace = true`
  を追加する**(`workspace.package` だけでは member に伝播しない)。
  「git 依存を含むため crates.io 公開不可」という状態を暗黙の
  まま残さず、Ruviform と同じ暫定扱いに揃える。
- license 整合: ruviform `MIT` の依存を `MIT OR Apache-2.0` の
  プロジェクトが Git 参照で利用するのは両立可能。vendor しない
  ため AL repo 内に複製・同梱するライセンス文書は発生しない。
  暫定 license の確定は Ruviform 側の課題として追跡する。
- Symphonia 版差: AL は `symphonia 0.5.5`、ruviform-io-symphonia
  は `0.6`。`ruviform-media-io` は io-symphonia を依存に持たない
  ことを manifest で確認済みのため、本 Issue では lockfile 上の
  混在は発生しない。#100/#154 で backend 採用時に版を一本化する
  方針(責務マップの decoder 差分バケット規約に従う)を記録する。
  検証として `cargo tree` で symphonia が単一 version のみである
  ことを確認する。
- 署名: パッケージ配布・リリース署名は現時点で対象外(両 repo
  pre-release、バイナリ配布なし)。

### 9. 実行時 platform feature の再現性

- `ruviform-math` の `fearless_simd` は `force_support_fallback`
  feature 付きで、`SimdToken::detect()` が実行時 CPU 検出 +
  フォールバックを行う。ビルド時の target feature 指定に依存
  せず、CI runner/開発機で同一バイナリ挙動となる。
- `ruviform-exec` の `audio_thread_priority` は OS 別 backend を
  持つ(locked 0.38.0 で確認)。dev-dep 配置のため shipped
  バイナリには入らないが、CI matrix の 3 OS で compile/test を
  検証する。失敗時は ruviform-exec 依存自体を #54 へ defer する。
- smoke-test は基礎型の構築・`version()` の非空のみを確認し、
  SIMD/デバイス依存の挙動は扱わない。

### 10. CI workflow(`.github/workflows/ci.yml`)

- Ruviform `ci.yml` と同一 convention で新設。トリガは当面
  **`workflow_dispatch` 限定**(Ruviform と同一方針: merge 後に
  Actions tab から手動検証する運用)。Deploy Key が未設定の状態で
  自動 gate 化すると cargo の manifest 解決自体が全件失敗する
  ため、credential 設定完了後に `pull_request` 等への拡張を別途
  判断する(Deferred 参照)。
- dispatch 可能になるのは default branch merge 後のため、
  本 PR 上では ci.yml 自体の e2e 検証ができない。軽減策として
  (a) `actionlint` を mise tool に追加し prek の local hook で
  workflow ファイルの静的検証を commit 時と CI で実施する、
  (b) 認証・probe 手順は手順書でローカル検証可能な形を記録する。
- Job 構成(matrix: `ubuntu-latest`/`windows-latest`/
  `macos-latest`、fail-fast 無し、`CARGO_INCREMENTAL=0`):
  1. `actions/checkout`(`lfs: true` — AL 側 fixture の LFS 実体を
     復元する)
  2. `jdx/mise-action` + `mise install --locked`
  3. `mise run verify-toolchain`
  4. private dep 認証設定(§4 の鍵ファイル + known_hosts +
     GIT_SSH_COMMAND + insteadOf)と `git ls-remote` probe
  5. `mise exec -- cargo fetch --locked`
  6. `mise exec -- prek run --all-files`
  7. `mise run fmt` / `check-ci` / `lint-ci` / `test-ci`
  8. `cargo build -p al-cli --profile ci` を `mise exec` 経由で
     実行する headless artifact ビルド step(GUI 側 lane は
     GUI 依存が存在しないため未設置。§7 guard で逆流を検知)
  9. `mise run check-cli-headless`
- 利用 action は GitHub 公式 `actions/checkout`/`actions/cache` と
  `jdx/mise-action` のみで、ruviform と同じ semver tag pin 規約に
  従う(秘密情報を扱う third-party action は使わない)。

## Data and Control Flow

依存解決(ローカル/CI 共通):

```text
Cargo.toml [workspace.dependencies] (git + rev 固定)
  -> cargo fetch (net.git-fetch-with-cli → system git、
     GIT_LFS_SKIP_SMUDGE=1)
       ローカル: Git Credential Manager (GitHub HTTPS)
       CI:       Deploy Key + url.insteadOf 書換え + known_hosts 照合
  -> Cargo.lock ("git+...?rev=<sha>#<sha>")
  -> build / check / lint / test (mise task 経由)
```

CI(手動検証フロー):

```text
workflow_dispatch
  -> checkout(lfs) -> mise install --locked -> verify-toolchain
  -> private-dep 認証設定(Deploy Key + known_hosts + insteadOf)
  -> git ls-remote probe -> cargo fetch --locked(mise exec)
  -> prek -> fmt -> check-ci -> lint-ci -> test-ci
  -> al-cli headless build -> check-cli-headless
```

## Non-Functional Constraints

- 認証秘密・token・秘密鍵をリポジトリのコード・コミット・ログに
  含めない(workflow は `secrets.*` 参照のみ、key は `RUNNER_TEMP`
  の実ファイルとしてのみ存在させログ出力しない)。
- credential は最小権限(ruviform 単一 repo・read-only)で、
  `secrets.GITHUB_TOKEN` や個人 token を流用しない。
- Cargo.lock・mise.lock は commit 対象のまま維持する。
- `mise run check/test/lint/build/fmt` の既存 task 契約を壊さない
  (追加のみ、既存 task の semantic は不変)。
- 新設 task は `cmd /c` で完結する形(POSIX 専用のテキスト処理を
  inline run に書かない)。
- Pre-release 規約: 互換層・deprecation を設けず、依存構成の変更は
  既存成果物の再生成で扱う。

## Risks and Trade-offs

- **同一 SHA の複数 entry 管理**: Cargo.toml に変数共有は無いため
  `rev` は 8 entry に重複する。全 entry 同一値の規約をコメントで
  明記し、更新は一括置換・lockfile で検証する。機械的検証は
  `cargo metadata`/`cargo tree` で source が単一 git+rev である
  ことを確認可能。
- **pin SHA の陳腐化と到達可能性**: Ruviform #150〜#157 の API
  追加で古い pin が必要機能を欠く。SHA 更新は各切替 Issue の判断で
  一括更新する規約とする。また pin 対象 SHA が ruviform 側の
  force-push で unreachable になった場合は fetch 不能になるため、
  手順書に「pin 済み commit を main 上で到達可能に保つ」運用を
  明記する。
- **Deploy Key 発行には ruviform 側 admin 権限が必要**: 同一 owner
  のため手順化可能だが、credential 未設定の間は CI の cargo fetch
  が失敗する。設計・手順書を先行し、設定完了まで dispatch 検証は
  保留できることを acceptance と両立させる。
- **`audio_thread_priority` のプラットフォーム依存**: ruviform-exec
  を dev-dep に留めたため shipped には入らないが、test 対象として
  3 OS で compile を検証する。失敗時は ruviform-exec 依存自体を
  #54 へ defer する。
- **AL repo が将来 public 化した場合**: private Git 依存は外部
  contributor が fetch できない。Pre-release の間は問題にならず、
  公開判断時に Ruviform 公開・vendoring・crates.io 移行のいずれかを
  再設計する(Deferred)。
- **workflow_dispatch 限定の CI は merge 前に workflow 自体を
  e2e 検証できない**: actionlint の静的検証と手順書のローカル
  検証手順で軽減し、初回 dispatch は手順書の checklist に従う。
  `pull_request` 有効化は credential 設定完了を前提に別途判断する。

## Deferred Design Concerns

- `ruviform-io-symphonia` 導入と Symphonia 版一本化 | backend
  選定が #154 で確定していない | AL#100 の切替設計時 | 両版並存で
  decoder 出力差の帰属が曖昧になる。
- Ruviform GUI 系 crate(#156/#157)の optional 導入・`al-gui`/
  `al-service` 構成 | crate 未設計・未実装 | AL#105 詳細設計 |
  headless への GUI 依存逆流(§7 guard で検知は可能)。
- `al-cli` → `al-viz` 依存・静的 report アセットの撤去 | 解決済み
  (AL#106 で `al-viz` 撤去・export を al-dataset 実装へ再配線) |
  AL#106 | 完了。
- `cargo vendor`/offline build | private access 前提で十分 | 配布・
  オフライン要件の発生時 | private repo 障害時の build 不能。
- `pull_request`/`push` 自動 CI トリガ | credential 未設定だと全
  PR が manifest 解決で失敗する | Deploy Key 設定完了後 | PR 上の
  自動 gate が無い期間は手動 dispatch で代替。
- fine-grained PAT/GitHub App への credential 移行 | Deploy Key で
  現要件を満たす | 監査・期限管理要件の変化時 | 長期の key 管理負債。
- AL の MSRV(`rust-version`)宣言 | pre-release で未検証の floor
  宣言は嘘になる | MSRV matrix の必要性が生じた時点 | 宣言無しは
  現状維持で矛盾しない(実効 floor は mise pin)。
- Ruviform 暫定 license(`MIT` 仮置き)の確定 | Ruviform 側の
  配布方針未確定 | Ruviform 側 licensing 決定時 | AL 側の同梱・
  配布判断に影響。

## Verification Notes

- [ ] 全 Ruviform dep が同一 `rev` で `[workspace.dependencies]` に
      集約され、`Cargo.lock` に `git+...?rev=<sha>#<sha>` が記録される。
- [ ] `cargo fetch --locked` が lockfile に記録された同一 commit を
      取得できる(ローカルで実測)。
- [ ] `cargo metadata`/`cargo tree` で Ruviform source が単一
      git+rev に統一され、symphonia が 0.5.5 のみであることを確認。
- [ ] `mise run verify-toolchain` が rustc/cargo/rustfmt/clippy の
      pin 解決を検証する。
- [ ] `mise run check/lint/test/build/fmt` が全て成功する(新設
      `check` を含む)。
- [ ] 各 smoke-test が対象 crate の基礎型 import を検証し、
      `mise run test` に含めて実行される。
- [ ] `mise run check-cli-headless` が対象 headless crate 群の
      GUI framework 依存非存在を検証する。
- [ ] ci.yml が actionlint で検証される(prek hook)。
- [ ] 手順書の credential 設定手順が secret を露出しない構成を持ち、
      `read_only=true`・host key 照合・probe を含む。

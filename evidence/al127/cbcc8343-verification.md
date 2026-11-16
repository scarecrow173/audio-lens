# AL#127 rev-watch 検証台帳: ruviform pin `eafe0a08` → `cbcc8343`

実行環境: Windows / Orca worktree
`G:\orca\workspaces\audio-lens\al127-cbcc8343` /
rustc 1.99.1（mise 解決 toolchain、`verify-toolchain` で一致確認）/
Node 26.12.1 / mise 2026.11.2 系。

- 対象 revision（ruviform resolved SHA）:
  `cbcc83437616fdd7418da37183c6be66b6cfe225`
  （`git ls-remote https://github.com/scarecrow173/ruviform.git
  refs/heads/main` と local clone `G:/repositry/audio-products/
  ruviform` の `origin/main` HEAD で一致確認。Spec 起稿時点と
  同一 SHA で upstream の進行なし）
- 旧 pin: `eafe0a08b71bd9a74566ae6678c3e0683fce1421`（PR #216）
- 差分: `eafe0a08..cbcc8343` = 6 commit・97 file・
  +15,611/−581（crates/ 9 crate、docs/・evidence/・root
  `Cargo.lock`/`mise.toml`。`.github/`・ruviform 側 `ui/` TS
  変更なし）
- `Cargo.lock` 差分: ruviform 26 package の `source` fragment
  （SHA）26 行のみ。version 変動・新規 package 追加・依存 edge
  変動なし（in-tree crate の `Cargo.toml` に変更がないため）。
- 依存木内の変更 crate（`de3ed4e8` = `scarecrow173/ruviform#630`
  `#676` test_instrument 由来の 3 crate）:
  - `ruviform-plugin-core`: `wire.rs` に shm slot 用固定長
    record codec `encode_event_record`/`decode_event_record` を
    additive 追加（既存 `encode_event`/`decode_event`/定数は
    不変）。
  - `ruviform-plugin-host`: `l1/data.rs` の内部 callsite を
    `encode_event` → `encode_event_record` へ更新（1 行、
    公開 API 不変）。
  - `ruviform-audio-test`: `backend/stream.rs` で非公開 fn
    `deliver_input_errors` を抽出する内部リファクタ
    （`pub(crate)`、公開契約不変）。
- 依存木外の変更 crate: `ruviform-audio-device`（RF#629 `#673`
  WASAPI endpoint probe/virtual_endpoint test）、
  `ruviform-plugin-clap`/`plugin-export`/`plugin-dock-test`/
  `plugin-test-clap`/`plugin-test-vst3`（RF#630 `#676`
  test_instrument、RF#631 `#674` Test Lab GUI、RF#532 `#677`
  DComp pixel regression）。AL lockfile 26 package に非含有。

## 実行した検証（全て正規 mise task、exit code 実測）

| コマンド | exit | 結果 |
| --- | --- | --- |
| `node tools/update-ruviform-rev.mjs cbcc83437616fdd7418da37183c6be66b6cfe225` | 0 | 26 package が `cbcc8343` へ原子更新 |
| `mise run setup-ui`（`npm ci --prefix ui`） | 0 | fresh worktree の ui 依存を復元 |
| `mise run sync-ui-deps` | 0 | `ui/vendor/ruviform` 再生成、`REV` = `cbcc8343...` |
| `mise exec -- cargo fetch --locked` | 0 | locked fetch 成功（補助証拠） |
| `mise run check-ruviform-lock` | 0 | 26 package 単一 SHA（fail-closed） |
| `mise run check-ruviform-rev` | 0 | REV = lockfile SHA、lockfile-primary |
| `mise run check-ui` / `mise run test-ui` | 0 | typecheck + node:test 113 件 fail 0 |
| `mise run fmt` | 0 | fmt 差分なし |
| `mise run check al-analysis` | 0 | `cargo check --locked --all-targets -p al-analysis` Pass |
| `mise run lint al-analysis` | 0 | clippy `-D warnings` Pass |
| `mise run test al-analysis` | 0 | 全 suite fail 0（`tests/rf628_oracle.rs` 7 tests・`rf628_long_scenario_non_regression` 約 132 秒実走を含む） |
| `mise run check-cli-headless` | 0 | headless 10 crate に GUI 依存なし |
| `mise run verify-toolchain` | 0 | rustc/cargo 1.99.1 一致 |
| `mise run check-full`（補助証拠） | 0 | workspace check Pass（al-gui/al-cli 等を含む） |

## upstream main CI 証拠（`cbcc8343` 対象、`gh run list --commit` 実測）

| workflow | run | 結果 |
| --- | --- | --- |
| `consumer-verify`（workflow_dispatch） | 43783883894 | **success** — preflight / verify-synvelis / verify-audiolens / summary 全 job success（2026-11-16T01:02:30Z） |
| `daily-ci`（schedule） | 43783844680 | **success** — decide / decide-consumer-verify（dispatcher のみ、2026-11-16T01:02:04Z） |
| `ci`（workflow_dispatch） | 43783872777 | **failure** — `verify (ubuntu-latest)`・`l1-real-verify (ubuntu-latest)`: `ruviform-plugin-test-vst3` clippy `-D warnings`（unnecessary `u32 as u32` cast ×5）。`verify-macos`・`rt-verify`: `ruviform-midi-device` test `output_ports` compile error（`MidiGroup`/`MidiChannel`/`NoteKey`/`MidiValue`/`MidiKind` 未解決）。`ci-summary` failure（集約）。windows lane・msrv・ui/observability/io/compression/icc/bench は success |

切り分け: 失敗 crate はいずれも AL lockfile 26 package に非含有
（依存木外）。`ruviform-plugin-test-vst3` の clippy 失敗は
`de3ed4e8` の instrument/events 系コードに起因する可能性が高い。
`ruviform-midi-device` は本差分で未変更（`bff3603b` = PR #663 以降、
pin `eafe0a08` に既に含有）であり既存問題の可能性。`scarecrow173/
ruviform#625`（CI failure tracker、Open）へ状況コメントを追加。

## NotRun / Unsupported（POLICIES.md §3 語彙）

- 実 OS GUI・実 Pixel・IME・DPI: **NotRun**（実ウィンドウ操作・
  実機 pixel 証拠を要する lane は本実行では未実施）
- macOS / Linux lane: **Unsupported**（実行環境は Windows のみ。
  統合検証は merge 後対象 SHA の CI 証拠が正本）
- cross-consumer（Synvelis 側）非回帰: **NotRun**（Coordinator
  側責務。upstream `consumer-verify` run 43783883894 は
  verify-synvelis/verify-audiolens 共に success であり upstream
  側証拠として記録）
- `gui-real` CI job: `not_run` placeholder（既存仕様どおり）
- ruviform 側 manual/opt-in lane（`test-l1-real-plugin`・
  `test-ui-plugin-dock`・`test-ui-real-scenario`・windowed
  `uitest`・Test Lab GUI lane 等）: **NotRun**（実 desktop
  session / 実 Plugin 前提の ruviform 側 lane であり AL の検証
  lane ではない）
- `lint-full`/`test-full`/`build-full`・`check-ui-gpui`/
  `lint-ui-gpui`/`test-ui-gpui`/`build-ui-gpui`・`check-ui-webview`/
  `lint-ui-webview`/`test-ui-webview`・`check-ci-workflow`/
  `check-ci-tracker-workflows`: **NotRun**（本 slice の正規検証
  コマンド対象外。補助証拠として `check-full` のみ実施）

## 記録のみの警告（main 既存・抑制なし）

- mbx cache の absolute-path mapping / build-script UTF-8 warning
  （Git checkout 由来）

## 採用/保留判断

- pin 追従として **採用**。全正規検証 lane が対象 revision で
  Pass、破壊的 API/Feature/OS 変更・AL 側恒久 workaround 要求は
  検出されなかった。upstream `ci` の failure は依存木外 crate
  に限定され、`consumer-verify`（verify-audiolens 含む）は
  success。
- upstream 新 API/feature（`encode_event_record`/
  `decode_event_record` record codec・plugin test_instrument
  fixture・audio-device endpoint probe・Test Lab GUI・DComp pixel
  regression）は opt-in/test-only/依存木外であり AL 側の採用
  gate・需要が未定義のため本 slice では **採用しない**。
  record codec は AL が plugin wire codec を直接消費しないため
  現状不要（採用判断は別 slice）。
- rollback: `Cargo.lock` を `eafe0a08b71bd9a74566ae6678c3e0683fce1421`
  へ戻す revert + `mise run sync-ui-deps` 再実行で復帰可能
  （製品コード差分なし）。

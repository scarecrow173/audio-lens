# Issue #255 — verify-audio-lens skill + `al-ctl` live proof

実行環境 (slice 2): Windows 11 (x86_64) / Orca worktree
`G:\orca\workspaces\audio-lens\al255-skill-slice2` /
rustc 1.99.1(環境導入、repo は rust を pin しない)/
mise 管理 tool: prek 0.5.7, biome 2.6.2, node 26.12.1, git-lfs 3.8.0。
`MISE_ENV` 未設定 = ローカル標準 profile(dev/test)。
**対話 desktop session あり**(console session active) —
real-window drive は実走可能で、9 case 全て `pass` を記録した。

slice 1 実走環境(当時の記録): 同 OS・別 worktree、対話 session
なし — 実 window drive は `NotRun`/`environment_blocked` で記録。

ブランチ: `feature/255-verify-skill-drive`(基点 `dd7e537` =
slice-1 merge = origin/main tip)。

## スコープ

slice 2(本記録の主対象): `features/` 4 節正規化(GUI surfaces +
scenario catalog)、`al-ctl drive`/`evidence` 追加、SKILL.md の
Drive/Evidence/Helpers 実値化、live proof 拡張。GUI lane は
`al-ctl drive` が `cargo` を子プロセスとして spawn した receipt
で記録 — fake/gpui lane は全 case Pass、real-os lane は
blocked-probe(fail-closed 記録)と **live run(実 window、全 case
Pass)** の両方を記録した。

slice 1(下部に保存): CLI + headless 範囲 — `al-ctl doctor`/
`launch`・`al run` 最小 job・read-only 副作用観測・cleanup。

## ファイル

slice 2:

| ファイル | 内容 |
| --- | --- |
| `drive-list.json` | `al-ctl drive --list --json` — pin 済み 9 case の catalog |
| `drive-fake.json` | `al-ctl drive --driver fake --json` receipt(spawned pid・child exit・captured stdout/stderr = `cargo test -p al-gui --test scenario` の結果) |
| `drive-gpui.json` | `al-ctl drive --driver gpui --json` receipt(`--features gpui-test --test scenario_gpui`) |
| `drive-real-os-blocked.json` | `al-ctl drive --driver real-os --blocked-probe` receipt + `--out` の record 数 |
| `drive-real-os.json` | `al-ctl drive --driver real-os` live run receipt(実 window) |
| `gui-real-blocked.jsonl` | lane 自身の JSONL — run header + 9 case 全て `not_run`/`environment_blocked`、`environment_kind: deterministic-backend` |
| `gui-real-live.jsonl` | lane 自身の JSONL — run header + 9 case 全て `pass`、`environment_kind: real-window`(正本: lane の `--out`、本 repo の `evidence/al219/` 出力先は別 lane 規約のため scratch に出力してから複写) |
| `evidence-scaffold.json` | `al-ctl evidence scaffold --issue 255 --json` — 既存 ledger で `exists`(非破壊)を確認 |
| `actions.jsonl` | action → 観測結果の 1 行台帳(slice-2 分は `al-ctl evidence append` 自身で追記) |

slice 1(保存): `doctor-before.json`・`doctor-after.json`・
`launch-run.json`・`launch-inspect.json`・`launch-dry-run.json`・
`gitignore-checks.txt`。

## 実走(slice 2 — actions + resulting state)

GUI lane は `al-ctl drive` が解決した argv を `cargo` 子プロセスと
して spawn(全 pid を receipt に記録・wait 済み)。

| 手順 | コマンド | exit | 観測 |
| --- | --- | --- | --- |
| catalog | `al-ctl drive --list --json` | 0 | 9 case、各 drivers/required/filter を列挙 — `catalog_source` が正本(`al-gui-real-scenario.rs` CASES + `scenario_real.rs` pins)を指す |
| fake lane | `al-ctl drive --driver fake --json` | 0 | spawn pid 60896 → `cargo test -p al-gui --test scenario` child exit 0、5 tests 全て pass(desktop.jobs / source.seek / dataset.workspace 各 body) |
| gpui lane | `al-ctl drive --driver gpui --json` | 0 | spawn pid 49740 → `cargo test -p al-gui --features gpui-test --test scenario_gpui` child exit 0、5 tests 全て pass(fake/gpui intent parity 含む) |
| real-os blocked-probe | `al-ctl drive --driver real-os --blocked-probe --out <scratch> --json` | 0 | spawn pid 8172 → child exit 0、10 record(run header + 9 case)全て `not_run` + `environment_blocked`、`environment_kind: deterministic-backend` — window を開かない決定的 fail-closed |
| real-os live | `al-ctl drive --driver real-os --out <scratch> --json` | 0 | spawn pid 72512 → child exit 0、**9 case 全て `verdict: pass` + `environment_kind: real-window`** — 実 window mount で job 選択/seek commit/workspace tabs/実 OS input・pointer・keyboard/2 面 capture/close-veto を駆動、session は全て `closed` で後始末済み |
| evidence scaffold | `al-ctl evidence scaffold --issue 255 --json` | 0 | 既存 `actions.jsonl`/`verification.md` は `exists`(非破壊)。`--repo tmp/al255-scratch` では `created` を確認 |
| evidence append | `al-ctl evidence append --issue 255 ...` | 0 | 本台帳 slice-2 行(drive-list〜cleanup)を `append` 自身で追記 |
| cleanup | `rm -rf tmp/al255-proof2 tmp/al255-scratch` | 0 | scratch 削除済み。spawn した全 pid は wait 済み・残留なし。証拠ファイルは `evidence/al255/` に残存 |

slice 1 実走表(保存):

| 手順 | コマンド | exit | 観測 |
| --- | --- | --- | --- |
| build | `mise run build al-cli` | 0 | `al.exe` + `al-ctl.exe` 生成 |
| doctor(前) | `al-ctl doctor --repo . --json` | 0 | `ok: true` — git `c917431b`、samples 38 files・LFS pointer 0・dictionaries 6 entries・`al` probe exit 0 |
| drive | `al-ctl launch --json -- run --base-dir tmp/al255-proof/jobs --input capture_bug:...,source_asset:... --stages ingest,locate` | 0 | spawn pid 65816・child exit 0・`job_id=JOB-DN0TilJa5Cemqu957HSXBc` |
| 副作用観測(2 回目 read-only) | `al-ctl doctor --base-dir tmp/al255-proof/jobs --json` | 0 | `base_dir` check が `exists: true, job_dirs: 1` — run の副作用を read-only で確認 |
| manifest 観測 | `al-ctl launch --json -- inspect --base-dir ... --job-id JOB-...` | 0 | manifest: `ingest` completed(4 artifacts)・`locate` completed(7 artifacts = 埋込 source の alignment)、inputs は `source_sha256` pin |
| dry-run | `al-ctl launch --dry-run --json -- run ...` | 0 | `spawned: false` — 解決済み cmd のみ、指定 base-dir は生成されない |
| gitignore | `git check-ignore -v` / `git status` | — | `verify-audio-lens/**` は追跡対象、他 deploy 物は ignore 維持(補助証拠 — canonical = mise ではない) |
| cleanup | `rm -rf tmp/al255-proof` | 0 | 生成 wav・job tree・scratch を削除。起動した process は全て wait 済み(PID 記録)、残留なし。証拠ファイルは残存 |

## verdict(POLICIES §3 — 要求状態 × 実行結果)

slice 2:

| 項目 | 要求状態 | 結果 | 根拠 |
| --- | --- | --- | --- |
| `al-ctl drive` — catalog 列挙・fake/gpui/real-os lane を子プロセス spawn で駆動、usage error は actionable | Required | Pass | `drive-list.json`・`drive-fake.json`・`drive-gpui.json`・`drive-real-os{,-blocked}.json`・`tests/al_ctl.rs`(catalog drift・filter 実在・拒否経路 12 test) |
| `al-ctl evidence` — scaffold(非破壊)/append(1 行 JSON)・必須 field・`--issue` 数値検査・dry-run | Required | Pass | `evidence-scaffold.json`・`actions.jsonl` slice-2 行が `append` 実走の産物・`tests/al_ctl.rs` |
| headless 境界 — `al-ctl` は std + serde_json のまま(GUI crate は subprocess 境界の向こう) | Required | Pass | `mise run check-cli-headless` `no GUI framework deps in 10 headless crates` |
| 実 window drive(real-os lane) | Required(製品が real window を提供する面) | **Pass** | `gui-real-live.jsonl` — 9/9 case `pass` + `environment_kind: real-window`、blocked-probe `gui-real-blocked.jsonl` で fail-closed 形状も確認 |
| `features/` 全ユーザー可視機能の 4 節正規化 | Required | Pass | `cli-job-run` + `gui-desktop` + `gui-source-view` + `gui-dataset-workspace` + `gui-scenario-catalog` + README index |
| SKILL.md Drive/Evidence/Helpers が配線済み実値 | Required | Pass | `SKILL.md` — 本 proof のコマンドどおりの記述、残る "Not wired yet" は real-window Pass 証拠のみ(本 slice で解消) |

slice 1 verdict(保存):

| 項目 | 要求状態 | 結果 | 根拠 |
| --- | --- | --- | --- |
| `.gitignore` 再包含で `verify-audio-lens/` が追跡可能 | Required | Pass | `gitignore-checks.txt` |
| SKILL.md 6 節が配線済み実値 | Required | Pass | slice-1 proof が記述どおり実行できたことで実証 |
| `al-ctl doctor` read-only 報告 | Required | Pass | `doctor-before.json`/`doctor-after.json` |
| `al-ctl launch` 入口・`--dry-run`/`--json`/`--help` | Required | Pass | `launch-*.json`・`tests/al_ctl.rs` |
| live proof の証拠(actions + resulting state) | Required | Pass | slice-1 台帳一式 |
| 実 window drive(GUI-L2) | Deferred → Required(slice 2 で解消) | NotRun → **Pass**(slice 2) | slice 1 は非対話環境で `environment_blocked` が正しい記録だった。slice 2 で live run Pass |

## 正規検証 lane 結果(canonical = `mise run`、slice 2)

| lane | コマンド | exit | 結果 |
| --- | --- | --- | --- |
| fmt | `mise run fmt` | 0 | Pass — 差分なし |
| check | `mise run check al-cli` | 0 | Pass |
| lint | `mise run lint al-cli` | 0 | Pass — warning なし |
| test | `mise run test al-cli` | 0 | Pass — 48 tests(`al_ctl.rs` 20 = 新規 12 + 既存 8) |
| test al-gui | `mise run test al-gui` | 0 | Pass — 全 suite green |
| headless 境界 | `mise run check-cli-headless` | 0 | Pass — `no GUI framework deps in 10 headless crates` |
| gpui lane | `mise run test-ui-gpui` | 0 | Pass — `gpui-test` feature 全 test |
| blocked-probe 非回帰 | `mise run test-ui-gpui-real-headless` | 0 | Pass — 10 record 全て `not_run`/`environment_blocked`(fail-closed 維持) |
| lockfile 決定性 | `mise run check-ruviform-lock` | 0 | Pass — `e4c9f6f0` 26 packages、Cargo.lock 差分なし |
| pre-commit | `mise exec -- prek run --all-files` | 0 | Pass — 全 hook green |

slice 1 lane 結果(保存): fmt/check/lint/test al-cli・
check-cli-headless・check-ruviform-lock・
test-ui-gpui-real-headless・prek — 全て Pass。

注: `test-ui-gpui-real-headless` は規約どおり
`evidence/al219/gui-real-headless.jsonl` を repo_sha で上書きする。
本 slice の編集範囲外のため SHA churn 分は commit せず revert
した(lane の出力先設計どおり — 統合側の再実行で再生成される)。
live `test-ui-gpui-real`(mise 直呼び)は `evidence/al219/` を
出力先に持つため本 proof では使わず、同じ bin を spawn する
`al-ctl drive --driver real-os --out <scratch>` で代替した —
spawn した子は同一 lane 本体であり、証拠は等価。

## NotRun / 補足

- `mise run test-ui-gpui-real`(mise 直呼び): 本 proof では実走
  せず — `al-ctl drive --driver real-os` が同じ lane 本体を
  spawn し Pass を記録したため重複実走は不要。al219 の出力先
  churn 回避でもある。残る差分は「mise task 経由の receipt が
  `al-ctl drive` の receipt 形状ではない」だけで、lane 本体の
  合格実績はある。
- `al-ctl drive` の `--scenario` positional/`--scenario` flag・
  per-scenario filter・usage error 経路(real-os filter 不可・
  real-os 専用 scenario の fake 指定・未知 scenario・trailing
  `--`)は `tests/al_ctl.rs` で網羅 — live proof は lane 全体で
  実走した。
- `*.mp4` 圧縮 fixture の ingest: `samples/README.md` 規約どおり
  派生 AVI が必要。本 proof は GUI lane + 制御 CLI 面のみ。
- `al-ctl` の PATH 解決(source `path`)・`release` profile 解決は
  本 proof では未実走 — `tests/al_ctl.rs` の explicit/sibling
  経路で代替検証済み。

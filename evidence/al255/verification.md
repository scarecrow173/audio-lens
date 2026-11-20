# Issue #255 slice 1 — verify-audio-lens skill + `al-ctl` live proof

実行環境: Windows 11 (x86_64) / Orca worktree
`G:\orca\workspaces\audio-lens\al255-verify-skill-slice1` /
rustc 1.99.1(環境導入、repo は rust を pin しない)/
mise 管理 tool: prek 0.5.7, biome 2.6.2, node 26.12.1, git-lfs 3.8.0。
`MISE_ENV` 未設定 = ローカル標準 profile(dev/test)。
対話 desktop session なし(実 window drive は `NotRun` が正しい結果)。

ブランチ: `feature/255-verify-audio-lens-skill`(基点 `c917431b` =
origin/main)。live proof 実走時点の作業 tree は `.gitignore`・
`crates/al-cli`・`.agents/skills/verify-audio-lens/`・本 evidence
の差分を含む(doctor の `git.dirty: true` はこれを反映)。

## スコープ

本 slice は Issue #255 の第 1 段(骨組み + 最小面)。live proof の
対象は **CLI + headless 範囲**: `al-ctl doctor`・`al-ctl launch`
(dry-run 含む)・`al run` 最小 job・read-only 副作用観測・cleanup。
実 window(GUI-L2)drive は対象外 — 対話 session がない環境のため
`NotRun` + `environment_blocked` で記録する(Pass 偽装しない、
`test-ui-gpui-real-headless` の先例どおり)。

## ファイル

| ファイル | 内容 |
| --- | --- |
| `actions.jsonl` | action → 観測結果の 1 行台帳 |
| `doctor-before.json` | `al-ctl doctor --json`(drive 前の健全性) |
| `launch-run.json` | `al-ctl launch --json -- run ...` の receipt(spawned pid・exit・captured stdout/stderr) |
| `doctor-after.json` | `al-ctl doctor --base-dir <jobs>`(副作用の read-only 2 回目観測 — `job_dirs: 1`) |
| `launch-inspect.json` | `al-ctl launch --json -- inspect ...` の receipt(子 stdout = job manifest JSON) |
| `launch-dry-run.json` | `--dry-run` の解決のみ receipt(`spawned: false`) |
| `gitignore-checks.txt` | `.gitignore` 再包含の `git check-ignore -v` / `git status` 補助証拠 |

## 実走(actions + resulting state)

fixture 入力は node で生成した決定的 WAV(mono 48kHz PCM。
`src.wav` = 4800 サンプル 1kHz sine、`cap.wav` = 24000 サンプル
低振幅 + `src` を offset 9600 に埋込 — `tests/cli.rs`・
`scenario.rs` の fixture 規約と同型)。スクラッチは `tmp/`
(gitignore 済み)配下に置き、cleanup で削除済み。

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

| 項目 | 要求状態 | 結果 | 根拠 |
| --- | --- | --- | --- |
| `.gitignore` 再包含で `verify-audio-lens/` が追跡可能 | Required | Pass | `git check-ignore`/`git status` 補助証拠(`gitignore-checks.txt`) |
| SKILL.md 6 節(Launch/Doctor/Drive/Evidence/Cleanup/Helpers)が配線済み実値 | Required | Pass | `.agents/skills/verify-audio-lens/SKILL.md` — 本 proof が記述どおり実行できたことで実証 |
| `al-ctl doctor` が version・git SHA・samples+LFS・dictionaries・base_dir を read-only 報告 | Required | Pass | `doctor-before.json`/`doctor-after.json`(pid ・ spawn は `al --help` probe のみ、書込なし) |
| 制御 CLI が `--dry-run`・`--json`・`--help`・launch 入口を備え Drive 節から呼べる | Required | Pass | `launch-run.json`/`launch-inspect.json`/`launch-dry-run.json`・`tests/al_ctl.rs` 8 test |
| live proof の証拠(actions + resulting state) | Required | Pass | 本台帳一式 |
| 実 window drive(GUI-L2) | Deferred(opt-in interactive lane `test-ui-gpui-real` の範囲) | NotRun | 非対話環境 — `environment_blocked` が正直な記録。blocked-probe 非回帰は `mise run test-ui-gpui-real-headless` で確認 |
| features/ 全ユーザー可視機能の 4 節正規化 | NotApplicable(本 slice 範囲外、後続 slice) | NotRun | seed `cli-job-run.md` のみ本 slice で提供 |

## 正規検証 lane 結果(canonical = `mise run`)

| lane | コマンド | exit | 結果 |
| --- | --- | --- | --- |
| fmt | `mise run fmt` | 0 | Pass — 差分なし |
| check | `mise run check al-cli` | 0 | Pass |
| lint | `mise run lint al-cli` | 0 | Pass — warning なし |
| test | `mise run test al-cli` | 0 | Pass — 36 tests(新規 `al_ctl.rs` 8 + 既存 28) |
| headless 境界 | `mise run check-cli-headless` | 0 | Pass — `no GUI framework deps in 10 headless crates`(al-ctl は dep 追加なし) |
| lockfile 決定性 | `mise run check-ruviform-lock` | 0 | Pass — `e4c9f6f0` 26 packages、Cargo.lock 差分なし |
| blocked-probe 非回帰 | `mise run test-ui-gpui-real-headless` | 0 | Pass — 10 case 全て `not_run`/`environment_blocked`(fail-closed 維持、意味論変更なし) |
| pre-commit | `mise exec -- prek run --all-files` | 0 | Pass — 全 hook green |

注: `test-ui-gpui-real-headless` は規約どおり
`evidence/al219/gui-real-headless.jsonl` を repo_sha で上書きする。
本 slice の編集範囲外のため SHA churn 分は commit せず revert
した( lane の出力先設計どおり — 統合側の再実行で再生成される)。
lane 合格自体は本台帳に記録する。

## NotRun / 補足

- `mise run test-ui-gpui-real`(実 window): 対話 desktop session が
  ないため NotRun — `test-ui-gpui-real-headless` の fail-closed
  blocked-probe がその正直な記録(後続の lane 検証で確認)。
- `*.mp4` 圧縮 fixture の ingest: `samples/README.md` 規約どおり
  派生 AVI が必要。本 proof は WAV のみ(最小 job)。
- `al-ctl` の PATH 解決(source `path`)・`release` profile 解決は
  本 proof では未実走 — `tests/al_ctl.rs` の explicit/sibling
  経路で代替検証済み。
- `apm install` の `.agents/skills/` 再生成: 本 worktree で実走確認済み
  (2026-11-20、7→10 dependencies install)。`verify-audio-lens/` は
  package manifest 外の repo-owned dir として**温存された**
  (SKILL.md・features/ 全ファイル残存、`git status` は追跡差分ゼロ)。
  removal preview は package 由来 deploy 物のみを対象とする。

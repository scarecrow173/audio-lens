---
id: "DESIGN-56ekRrs3U0gtF3sylCGYeE"
type: "design"
status: "approved"
title: "Native GUI の Jobs 対話選択と Run 再開・取消の設計 (第7 slice)"
created: "2026-11-08"
updated: "2026-11-08"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/audio-lens/issues/166"
  changes:
    added: []
    modified:
      - type: "file"
        path: "docs/designs/native-gui-job-run-mount.md"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements: []
  implemented-by: []
  depends-on:
    - "native-gui-job-run-mount.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "../specs/native-gui-interactive-job-run.md"
    - "../test-specs/native-gui-interactive-job-run.md"
    - "https://github.com/scarecrow173/ruviform/issues/448"
    - "https://github.com/scarecrow173/ruviform/issues/456"
  refines:
    - "native-gui-job-run-mount.md"
  refined-by: []
  derives-from:
    - "../specs/native-gui-interactive-job-run.md"
  derived-by: []
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# Native GUI の Jobs 対話選択と Run 再開・取消の設計 (第7 slice)

## 概要

第6 slice の「job 選択は起動引数のみ・Run 操作は未接続」を解消する。ruviform#448（`a998101c`）の
`TableSpec.selection`・`ButtonSpec`・`ViewHost::select_row`/`invoke` を消費し、Jobs 表の行選択と
Run tab の Resume/Cancel を、既存の service 操作（`al.job.resume`）と runtime の操作（取消）へ接続する。

## 1. 依存の更新

`node tools/update-ruviform-rev.mjs a998101cc8707e160d74b05f31979d27af40142f` で `Cargo.lock` の 20 source を
更新し、`mise run sync-ui-deps` で `ui/vendor/ruviform` を同一 SHA に再生成する。`check-ruviform-lock` と
`check-ruviform-rev` は同じ SHA を返す。`Cargo.toml` は変更しない。

## 2. 契約の流れ

GPUI の view は Table 行の押下・Button の押下を `ViewHost` へ渡し、`CommandIntent::Action` を
`rvf.ui.command.v1` の Intent として window の Action channel に流す。

```text
行押下 ──► ViewHost::select_row ──► Intent(select, {row})    ──┐
Button ──► ViewHost::invoke      ──► Intent(resume / cancel)  ──┤
                                                               ▼
                       DesktopSession::handle(WindowLifecycle::Action)
                         ├─ cancel ──► runtime.cancel_window_jobs(window) ──► receipt(accepted | rejected)
                         └─ その他 ──► UiRuntime::dispatch_window_intent
                                         └─ AlGuiService::command
                                              ├─ select ──► JobService::list[row] ──► Result(applied, job_id)
                                              └─ resume ──► call_internal("al.job.resume") ──► CallOutcome::Job
                                                              └─ spawn_job ──► Result(accepted, {job,label})
                       receipt は window の inbox へ post され、ViewHost::apply_receipt で binding を進める
```

### 2.1 receipt の対応

| 操作 | 返す Result | ViewHost の結果 |
| --- | --- | --- |
| select（行 → job 解決に成功） | `ok`・`status: applied`・`job_id` | 選択を確定。session は `job_id` を採用し再射影する |
| select（行が範囲外） | `err`（invalid_params） | `rejected` → 直前の選択へ戻す。session の選択は変えない |
| resume（job を spawn） | `ok`（`job`・`label`、status 無し） | `accepted`。終端 receipt は届かない（ruviform#456） |
| resume（不正な params・job 不在） | `err` | `rejected` |
| cancel（window の実行中 job あり） | `ok`・`status: accepted` | `accepted`。終端は job event で反映する |
| cancel（対象無し） | `err`（unknown_job） | `rejected` |

select は service が一覧の行から job を引くため、行 → job の対応は service 層だけが持つ。
session は receipt の `job_id` を読んで「画面の job」を更新する。

### 2.2 cancel を session で処理する理由

取消には runtime の job 番号（`job.cancel` の `job`）が要るが、Button の params は静的で、service は
runtime の cancel token を持たない。`cancel_window_jobs(window)` は既存の runtime 操作で、window が所有する
job の取消を表す。単一 window の desktop では所有 job は高々 1 つのため、この粒度で十分とする。
取消は session（composition 直下の driver-neutral 層）で処理し、receipt は session が返す。

## 3. 再射影

画面の射影元は `(base_dir, selected_job)` だけで、`desktop_screen(base_dir, selected_job)` が決定的に作る。
session は次の二つで射影が古くなったことを記録する（`take_projection_dirty`）。

- 選択 job が変わった（select の receipt `applied` で `job_id` が異なる）
- job が終端に達した（Tick で `Event::Job` の `finished`/`failed`/`cancelled`）

composition root（`bin/al-desktop.rs`）は dirty のときだけ `desktop_screen` を作り、
`GpuiDriver::update_view(window, spec)` で window の ViewSpec を置き換える。`GpuiDriver` は `Clone` で、
`run` の handler はその clone を持つ。session 自体は `WindowDriver` の trait object しか見ないため、
`update_view`（GPUI 固有の inherent method）は呼べない。ここが composition root に再射影を置く理由である。

Table の `selected` は ViewHost が更新時に新しい行数で切り詰めるため、同じ job 一覧なら選択は保たれる。

## 4. 起動引数と初期選択

`al-desktop [<base-dir> [<job-id>]]` の `job-id` は `DesktopSession::set_selected_job` で初期の
`selected_job` になり、初期の `desktop_screen` に使う。Jobs 表の行の強調は初期化しない（§非スコープ）。

## 5. 責務と境界

| 層 | 役割 | 持たないもの |
| --- | --- | --- |
| `al-gui::command` | command 語彙（op 名・target・action）と Action の decode | GPUI 型・画面の組み立て |
| `al-gui::service` | `rvf.ui.command.v1` を `JobService`・`al.job.resume` へ写す | 再射影・cancel の runtime 操作 |
| `al-gui::view` | Jobs 表の selection・Run の Button の射影 | 選択の状態（ViewHost が持つ） |
| `al-gui::desktop` | 選択 job・dirty の追跡、cancel の処理、receipt | 画面の組み立て・GPUI |
| `bin/al-desktop.rs` | 引数解決・mount・`update_view` の適用 | 射影の判断 |

`command` 語彙は service（headless）と view（headless）と session の三者が使うため、service → view の
依存を作らないよう独立モジュールに置く。GPUI の `COMMAND_INTENT_OP` は gpui feature の内側にあるため、
headless 側は同じ値を `command` に持ち、gpui-test の parity で一致を検査する。

## 6. 代替案

| 案 | 内容 | 判断 |
| --- | --- | --- |
| A | Jobs 表の行選択を service ではなく session で解決する | 不採用。行 → job の規則を二箇所に持つ（DRY 違反） |
| B | 再射影を `WindowDriver` の trait に `update_view` として足す | 不採用。ruviform 契約の変更になり、本 slice の範囲を超える |
| C | 新規 run を `al.job.run` で Button から起動する | 不採用（本 slice）。入力 file の指定 UI が無く、`resolve_config` の再 pin が再現性を損なう |
| D | cancel を service op にし、runtime の job 番号を渡す | 不採用。Button の params は静的で job 番号を持てない |

## 7. リスク

- job 終端の receipt が無い（ruviform#456）。Resume の Button は `accepted` で止まり、進捗の表示は再射影頼みになる。
- 再射影で Tabs の `active` が Jobs に戻る可能性がある（spec の値に戻るため）。GPUI の tab 状態保持は本 slice では未検証。
- 行 index と一覧の順序は service の `list`（job id 昇順）に依存する。一覧が変わると行と job の対応がずれ得る。
  画面の Jobs 表は選択・終端でしか再射影しないため、外部プロセスが job を追加・削除した後の行選択は、
  表示と異なる job を選ぶ可能性がある（P2）。解消には行の job key を selection intent に載せる ruviform 側の契約が要る。
- 取消が既に受理されて終端待ちの job に再度 Cancel を押すと、`cancel_window_jobs` は新規に signal した数を返すため
  `rejected`（対象無し）になる（P3）。取消の完了は job 終端の再射影で示す。
- 同じ job の Resume を二重に押すと、2 つ目は runner の advisory lock で失敗する（`job is already running`）。
  manifest は壊れないが、受理ではなく job の失敗として現れる（P3）。
- 再射影の経路（`bin/al-desktop.rs` の `update_view`）は compile と lint のみで、自動テストは無い（P3）。
  session の dirty 通知と desktop_screen の射影は自動テストで検査している。

## 8. 既存設計の更新

`native-gui-job-run-mount.md` の §3（Job 選択の判断）と §6（Job 操作の範囲）は、RF#448 完了後の状態に合わせて
本 slice で更新する。§8 の ruviform#448 は closed、ruviform#456 を追記する。

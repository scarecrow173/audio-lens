---
id: "DESIGN-WRNx7ppADH8dxL2tG2HjFK"
type: "design"
status: "draft"
title: "Native GUI の Job/Run tab mount と Tile::columns 消費の設計 (第6 slice)"
created: "2026-11-08"
updated: "2026-11-08"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/audio-lens/issues/166"
  changes:
    added: []
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements: []
  implemented-by: []
  depends-on:
    - "native-gui-drawn-parity-run-mount.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "../specs/native-gui-job-run-mount.md"
    - "../test-specs/native-gui-job-run-mount.md"
    - "https://github.com/scarecrow173/ruviform/issues/443"
    - "https://github.com/scarecrow173/ruviform/issues/446"
  refines: []
  refined-by: []
  derives-from:
    - "../specs/native-gui-job-run-mount.md"
  derived-by: []
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# Native GUI の Job/Run tab mount と Tile::columns 消費の設計

## 概要

第 5 slice で未決だった「どの base_dir を画面の入力とするか」「Job 操作を画面でどう扱うか」を決め、
al-desktop の root 画面を Jobs・Run・Dataset の Tabs にする。あわせて、上流が公開した
`Tile::columns` を消費して `spectrogram_columns` の複製を除去する。

## 1. Ruviform rev の追従(Cargo.lock 正本)

`node tools/update-ruviform-rev.mjs d3be1ad9…` で `Cargo.lock` の 20 source のみを更新し、
`sync-ui-deps` で `ui/vendor` を同一 SHA に再生成する。`Cargo.toml` は変更しない
(`docs/designs/ruviform-lockfile-source-of-truth.md` §5)。

`d3be1ad9` は次を含む。

| 上流 | 内容 | 本 slice での利用 |
| --- | --- | --- |
| `ccee0cb1`(ruviform#445、RF#443 部分達成) | `Tile::columns`・`TileColumn`・`column_offsets`(`Spectrogram::tile` と共有する列規則) | `spectrogram_screen` の列へ写す |
| `d3be1ad9`(ruviform#447、RF#446) | 空 frame 範囲の `Spectrogram::tile` の panic 修正 | 空 dataset の Spectrogram tab が落ちない前提 |

## 2. RF443: `spectrogram_columns` の複製除去

前 slice の `view::screen::spectrogram_columns` は `Spectrogram::tile` の等分割規則を複製していた。
上流の `Tile::columns` は同じ規則を公開し、`count`(有限セル数)・`magnitudes`・クランプ済みの frame 区間を返す。
複製を消し、`tile.columns()` の各列を `SpectrogramColumn { start, end, count, magnitudes }` へ 1 対 1 で写す。

差分は空 frame 範囲の tile でのみ生じる。複製は `end = f0 + cf1` を `tile.frames.end` で切らず、
空 tile の列を `f0..f0+1` にしていた。上流は `frames.end` で切るため空区間になる。
`count` の意味は `SpectrogramColumn::count`(`0..=rows`、`0` = gap 列)と一致し、変更しない。

## 3. Job 選択の判断と上流 gap

`ViewSpec` の `Table`(`TableSpec`)は読み取り専用で、行選択の intent を持たない。`ControlSpec` 系
(`Fader`・`Seek`・`Knob`・`Toggle`)は連続値・二値の操作であり、job の選択肢には写らない。
ruviform の component 集合には Button 相当の command 起動 component も無い。

| 案 | 内容 | 判断 |
| --- | --- | --- |
| A | Jobs tab の行を選択し、Run・Dataset を切り替える | 不採用。行選択 intent が ViewSpec に無く、上流の契約追加が要る |
| B | 起動引数で job を指定する(`al-desktop <base-dir> <job-id>`) | **採用**。上流変更なしで選択を画面に反映できる |
| C | job ごとに tab を作る | 不採用。job 数に比例して tab が増え、上限を持てない |

案 A の上流契約(行選択・command 起動)は ruviform へ Issue として起票する(AudioLens 側の恒久回避策は作らない。
AGENTS.md「Ruviform とのマルチリポジトリ運用」)。Issue 番号は本書の §8 に記す。

## 4. base_dir と dataset root の渡し方

| 項目 | 採用 | 理由 |
| --- | --- | --- |
| base_dir の入力 | 第 1 引数。省略時は `GuiPreferences::base_dir` | 起動ごとに明示でき、preferences は既定値として残す |
| 解決の単一化 | `main` で一度解決し、`AlGuiService` と `desktop_screen` の両方へ同じ値を渡す | 同一概念の二重解決を避ける |
| job の置き場 | `JobService::new(base_dir)`(`list`・`inspect`・`open_dataset`) | 既存の job 操作の入口を使い、置き場の規則を複製しない |
| dataset root | `JobService::open_dataset(job_id)`(`<base>/<job>/analysis`) | dataset の場所は job の規則に従う |
| 旧 `<dataset-root> <input-id>` | 廃止 | 直接指定は job の規則と二重になる。pre-release のため互換層は作らない |

起動の引数は `[<base-dir> [<job-id>]]` の 0〜2 個とする。1 個の引数は常に `base-dir` として解釈し、
`job-id` 単独の指定の形は持たない(`job-id` だけでは base_dir の解決規則が定まらないため)。`job-id` は
`JobId::parse` で検証し、不正なら起動を拒否する(終了コード 2)。`job-id` の形をした 1 個の引数は
`base-dir` として解釈され、一覧が空になる(誤指定の検出は本 slice の対象外)。

## 5. 画面構成(UI 契約)

root は `desktop` の Tabs。tab の順序・key は固定し、既定の active は 0(Jobs)とする。

| tab key | 表示名 | 内容 | 入力 | 失敗時 |
| --- | --- | --- | --- | --- |
| `tab-jobs` | Jobs | 1 列 Table(job id) | `base_dir` の `JobService::list` | 理由 Label |
| `tab-run` | Run | 状態 Label と stage Table | 選択 job の `JobService::inspect` の manifest | 理由 Label(job 未選択・job 不在を含む) |
| `tab-dataset` | Dataset | Workspace(Source・Spectrogram・Analysis)の Tabs | 選択 job の `open_dataset` と先頭 input | 理由 Label(dataset 未作成を含む) |

- Run は `RunOutcome::from_manifest` で manifest の stage を typed に写す。`run/resume` の op 結果 JSON 形は
  consumer 側の再現に依存しないため、desktop は JSON を経由しない。
- Run の状態は永続 manifest の最終状態である。実行中の進捗は表示しない(§6)。
- Dataset の input は job の先頭 input(`manifest.inputs[0]`)とする。role による選択は、role が無い job で
  決められなくなるため採用しない。

## 6. Job 操作の範囲

本 slice で desktop から行うのは一覧・選択(起動引数)・結果表示のみとする。Run の起動・再開・取消は、
`ViewSpec` に command 起動の component が無いため行わない。`DesktopSession`(driver-neutral)の既存の
close・exit・job 待機の規則は変更しない。

## 7. 責務と境界

- `view::desktop`(新規)は driver-neutral。`JobService` と `view` の射影だけを使い、GPUI 型を持たない。
- `bin/al-desktop.rs` は引数解決・preferences・mount だけを行う。画面の組み立ては `view::desktop` に置く。
- Workspace の Tabs・理由 Label は `view::mod` の `tabs`・`unavailable` へ移し、Workspace と desktop で共有する
  (同じ表現を二箇所に持たない)。
- 数値の再計算はしない。Run・Dataset は既存の射影をそのまま使う。

## 8. 上流 Issue と参照

| 対象 | 内容 | 状態 |
| --- | --- | --- |
| ruviform#443 | Tile → SpectrogramColumn の列区間・count の上流契約 | 部分達成(`ccee0cb1`)。AudioLens の消費をコメントで追記済み |
| ruviform#446 | 空 frame 範囲の `Spectrogram::tile` panic | 修正済み(`d3be1ad9`) |
| ruviform#448(本 slice で起票) | 読み取り専用 Table の行選択と command 起動の ViewSpec 契約 | Open。AudioLens の Job 選択・Run 起動の前提 |

## 9. テストの方針

- Job の e2e は実 job(`JobService::run`、`ingest → cheap_analysis → dataset`)を使う。fixture は
  `tests/common::make_job` に 1 箇所で定義する。
- 射影の検証は Fake 層(`ViewHost` の mount と tab の構造)を基本とし、drawn parity は GPUI 層の観測のみを使う。
- RF443 の検証は `Tile::columns` の結果との一致と、空 tile の境界で行う。

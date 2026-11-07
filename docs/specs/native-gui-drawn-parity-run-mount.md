---
id: "SPEC-laQ2e5k5G9oPKq1z0KZmcF"
type: "spec"
status: "draft"
title: "Native GUI drawn parity の Spectrogram・Label・Table 拡張と Job/Run mount 判断 (Issue #166 第5 slice)"
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
    - "native-gui-spectrogram-analysis-projection.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "../designs/native-gui-drawn-parity-run-mount.md"
    - "../test-specs/native-gui-drawn-parity-run-mount.md"
    - "https://github.com/scarecrow173/ruviform/issues/436"
    - "https://github.com/scarecrow173/ruviform/issues/442"
    - "https://github.com/scarecrow173/ruviform/issues/443"
  refines: []
  refined-by: []
  derives-from:
    - "native-gui-spectrogram-analysis-projection.md"
  derived-by: []
  verifies: []
  verified-by:
    - "../test-specs/native-gui-drawn-parity-run-mount.md"
  references: []
  defers: []
  deferred-by: []
---

# Native GUI drawn parity の Spectrogram・Label・Table 拡張と Job/Run mount 判断

## What Should Be Built

Issue [#166](https://github.com/scarecrow173/audio-lens/issues/166) の AL-G3/AL-G4 残課題を、
第 5 slice として次の範囲で満たす。前 slice(第 4 slice)で NotRun とした項目のうち、
上流が提供した観測 seam で検証可能になったものを閉じる。

- **Ruviform rev の追従**: `627e4bc2` から `3121f8e6`(最新 main)へ `Cargo.lock` を更新する。
  このリビジョンは `DrawnPlan` の `Spectrogram`・`Label`・`Table` variant を含む
  (ruviform#442、PR #444 merge 済み)。`check-ruviform-lock` / `check-ruviform-rev` /
  `sync-ui-deps` で単一 SHA と vendor 整合を確認する。
- **drawn parity の拡張**: `tests/gpui_parity.rs` で `GpuiObserver::drawn` の
  `DrawnPlan::Spectrogram`(cell の位置・intensity)、`Label`(text)、`Table`(header と行、
  欠損セルの位置)を、mount した spec の payload から導いた Fake 層の期待値と照合する。
  期待値が空・欠損セルが無い場合に偽陽性で通らないよう、fixture 自体の性質も検証する。
- **Job/Run の射影 e2e 拡張**: 実 dataset の job(`JobService::run`)から得た一覧を `job_list`
  で Table へ写し、stage 結果を `run_screen` で Label・Table へ写して Fake 層へ mount する。
  Run 結果の形は `al.job.run` の stage 表示 JSON(`{stage,status,duration_ms,error}`)と同じ
  とし、実 manifest の stage 数・状態と一致することを確かめる。
- **Job/Run の al-desktop mount 判断**: 入力源の有無を確認し、接続可否と未達理由を明記する
  (後述「Job/Run mount の判断」)。

## Job/Run mount の判断

al-desktop の起動契約は `<dataset-root> <input-id>` のみで、Job 一覧の入力源
(`JobService::list` の base_dir)や Run の実行結果を画面へ渡す経路を持たない。
`preferences.base_dir` は `AlGuiService` の既定 base_dir にのみ使われ、desktop の
画面構成(Tabs: Source・Spectrogram・Analysis)には接続されていない。Run は
desktop から job を実行する操作が無いため、結果の入力源も無い。

よって本 slice では al-desktop へ Job/Run の tab を追加しない。Job 一覧の tab 追加は
「どの base_dir を画面の入力とするか」「Job 操作(実行・再開)を AL-G4 でどう扱うか」の
UI 契約判断を伴うため、coordinator の判断を待つ(未達)。射影と Fake 層 mount は e2e で
固定する。

## Non-goals(本 slice では扱わない)

- **al-desktop への Job/Run tab 追加**: 上記の判断。未達として PR と IR に明記する。
- **RF443 の上流契約への移行**: `Tile` → `SpectrogramColumn` の列区間・count の規則は
  ruviform#443 が OPEN のまま(上流 API 無し)。`view::screen::spectrogram_columns` の複製は
  維持し、PR 本文に明記する。
- **Spectrogram の theme token**: 色は既存の viz 色を使う(ruviform#434 の F3)。
- **実 OS window での表示確認・AL-G5(性能・visual regression)**: 対象外。
- 数値 oracle(f64・NaN・gap・PTS・timebase・Validity・Provenance)と Theme/Source の
  既存契約は変更しない。GPUI 型は `bin/al-desktop.rs` と gpui-test parity test の外に出さない。

## Acceptance

- `GpuiObserver::drawn` が返す Spectrogram cell は、mount した `SpectrogramSpec` の payload
  (gap 列 `count == 0`・非有限 magnitude・viewport 外の列を除く)から導いた期待値と
  位置・intensity が一致する。cell 数は期待値と一致し、fixture は gap と非有限セルを含む。
- `DrawnPlan::Label` の text は `LabelSpec::text` と一致し、`DrawnPlan::Table` の header は
  `TableSpec::columns`、行は `CellValue` の表示文字列と一致する。欠損セルは `None` の位置を保つ。
- 実 job の一覧は `job_list` で 1 行 1 job の Table になり、実 manifest の stage 数・状態から
  得た Run 画面が Fake 層に mount できる。
- Ruviform rev は最新 main `3121f8e6` で `check-ruviform-lock` / `check-ruviform-rev`
  (`--require-vendor`)/ `sync-ui-deps` が通る。
- GPUI 型は通常画面・ViewSpec・Domain・CLI に現れない(`check-cli-headless` が通る)。
- 既存数値 oracle が非回帰(`mise run test` 全件 pass)。
- Job/Run の al-desktop mount と RF443 の複製除去は未達として PR 本文と IR に理由を明記する。

---
id: "TSPEC-mEkVPMvQTo5Q4ov43W1ukN"
type: "test-spec"
status: "draft"
title: "Native GUI の Job/Run tab mount と Tile::columns 消費の検証仕様 (第6 slice)"
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
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "../specs/native-gui-job-run-mount.md"
    - "../designs/native-gui-job-run-mount.md"
    - "https://github.com/scarecrow173/ruviform/issues/443"
    - "https://github.com/scarecrow173/ruviform/issues/446"
  refines: []
  refined-by: []
  derives-from: []
  derived-by: []
  verifies:
    - "../specs/native-gui-job-run-mount.md"
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# Native GUI の Job/Run tab mount と Tile::columns 消費の検証仕様

## Feature: desktop の root Tabs と実 job の射影(AC1・AC2・AC3・AC4)

実 job(`make_job`: 1 秒の capture、`ingest → cheap_analysis → dataset`)を `base_dir` に置き、
`desktop_screen(Some(base), Some(job))` を Fake 層(`ViewHost`)へ mount する。

### Example: 3 tab が固定順で mount される

- root が Tabs(key `desktop`)で、tab key が `tab-jobs`・`tab-run`・`tab-dataset` の順。
- 各 tab key が `ViewHost::observe` で mounted になる。

### Example: Jobs tab は job を 1 行の Table で示す

- `jobs` Table の行は 1 件で、その列 0 が job id と一致する。

### Example: Run tab は manifest の stage を示す

- 状態 Label が `job <id>: completed 3/3, failed 0` と一致する。
- `stages` Table は 3 行で、各 status が `completed`。

### Example: Dataset tab は Workspace を示し、Spectrogram が実 tile を持つ

- `tab-dataset` の child は key `workspace` の Tabs。
- `tab-spectrogram` の Panel に、列を 1 つ以上持つ Spectrogram がある。

## Feature: 読めない tab の fallback(AC5)

### Example: base_dir 未設定では全 tab が理由を示す

- 3 tab それぞれの panel に `unavailable: not found: base_dir is not set` の Label がある。
- 画面は mount できる。

### Example: job 未選択では Jobs だけが job を示す

- Jobs tab は job を 1 行で示す。
- Run・Dataset は `unavailable: not found: no job selected` を示す。

### Example: 存在しない job は Run・Dataset だけが失敗する

- Jobs tab は既存の job を 1 行で示す。
- Run・Dataset は `unavailable: job <id> not found` を示す。
- 画面は mount できる。

## Feature: GPUI drawn parity(AC7)

### Example: desktop の active tab(Jobs)の Table が Fake の期待値と一致する

- `GpuiObserver::drawn` の描画 component key は `[jobs]` のみ(active tab の leaf だけが描かれる)。
- Table の header と行は mount した `TableSpec` の payload と一致する。

## Feature: RF443 の列消費(AC6)

### Example: 列は `Tile::columns` の frame 区間・count・magnitudes と一致する

- 10 frame・4 列の tile で、各列の `start..end`・`count`・`magnitudes`(bit 一致)が `tile.columns()` と一致する。

### Example: 空 frame 範囲の tile は空区間の列と `count == 0` を持つ

- `frames = 7..7`・3 列の tile で、全列の区間は `7..7`、`count` は 0。

### Example: 既存の列規則の例は変わらない(非回帰)

- 10 frame を 4 列へ分ける区間は `0..2, 2..5, 5..7, 7..10`。
- 2 frame を 4 列へ分ける区間は `0..1, 0..1, 1..2, 1..2`。
- gap 列(全 NaN)は `count == 0` で、magnitudes は NaN のまま。

## Feature: 上流の固定と既存契約の非回帰(AC8・AC9・AC10)

| 検証 | コマンド | 期待 |
| --- | --- | --- |
| lockfile の単一 SHA | `mise run check-ruviform-lock` | `d3be1ad9` の 20 package |
| manifest/vendor 整合 | `mise run check-ruviform-rev`・`mise run sync-ui-deps`・`mise run check-ui` | 通過 |
| GPUI 境界 | `mise run check-cli-headless` | 通過 |
| 既存数値 oracle | `mise run test`・`mise run test-ui-gpui` | 既存 test を含め全件 pass |
| lint・整形 | `mise run lint`・`mise run lint-ui-gpui`・`mise run fmt` | 警告ゼロ・差分なし |

## 未検証(NotRun)

- 実 OS window での Jobs・Run・Dataset の表示確認(AL-G5 の範囲)。
- Job 選択の対話操作・Run の起動・再開・取消(上流の契約が無い。設計書 §3・§6)。

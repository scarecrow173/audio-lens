---
id: "DESIGN-KmsgLgUSS9WY4blzhdDRqp"
type: "design"
status: "draft"
title: "Native GUI drawn parity の Spectrogram・Label・Table 拡張と Job/Run 射影の設計 (第5 slice)"
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
    - "../specs/native-gui-drawn-parity-run-mount.md"
    - "../test-specs/native-gui-drawn-parity-run-mount.md"
    - "https://github.com/scarecrow173/ruviform/issues/442"
    - "https://github.com/scarecrow173/ruviform/issues/443"
  refines: []
  refined-by: []
  derives-from:
    - "../specs/native-gui-drawn-parity-run-mount.md"
  derived-by: []
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# Native GUI drawn parity の Spectrogram・Label・Table 拡張と Job/Run 射影の設計

## 概要

第 4 slice(`native-gui-spectrogram-analysis-projection.md`)で NotRun だった drawn parity の
Spectrogram・Label・Table を、上流 `DrawnPlan` の新 variant で観測可能にして照合する。
Job/Run は入力源の有無で mount を判断し、本 slice は射影の e2e まで閉じる。

## 1. Ruviform rev の追従(Cargo.lock 正本)

Ruviform の resolved SHA の正本は `Cargo.lock`(lockfile-primary, `docs/designs/ruviform-lockfile-source-of-truth.md`)。
manifest には rev を書かない。`node tools/update-ruviform-rev.mjs 3121f8e6…` で 20 package の
source だけを更新し、`sync-ui-deps` で `ui/vendor` を同一 SHA に再生成する。`Cargo.toml` は変更しない。

`3121f8e6` は ruviform#442(PR #444 merge)の `DrawnPlan::{Spectrogram, Label, Table}` と、
#436 の `GpuiObserver::drawn` を含む。

## 2. drawn parity の期待値

期待値は Fake 層の規則から導く(`GpuiObserver` の実装を写さない独立した導出)。

| plan | 期待値の導出元 | 照合 |
| --- | --- | --- |
| Spectrogram | `SpectrogramSpec.columns`:`count != 0` の列・viewport 内の列。列 `c` の x は `viewport_x(start/end)`、行 `r` の y は `1 - (r+1)/rows` 〜 `1 - r/rows`、intensity は `(m - lo)/(hi - lo)` を `[0,1]` に clamp。非有限 `m` は除外 | cell 数・各 field を `1e-6` で照合 |
| Label | `LabelSpec::text` | 文字列一致 |
| Table | `TableSpec::columns` と `rows`(`CellValue` は `to_string`、欠損は `None`) | header と格子を一致 |

fixture は 8 frame × 4 bin の `VecSpectrum` から `Spectrogram::tile`(4 列)で作る。frame 2・3 の
全 bin を NaN にして gap 列(`count == 0`)を、frame 4・5 の bin 1 を NaN にして gap でない
非有限セルを作る。期待 cell 数が `rows × cols` より小さいこと、Table に欠損セルがあることを
テストで確かめ、照合が空振りしないようにする。

## 3. Job/Run の射影 e2e

実 dataset の job を `JobService::run` で作り、`JobRow` を `job_list` で Table にする。Run 結果は
al-gui の `stage_statuses`(非公開)と同じ `{stage,status,duration_ms,error}` 形で
`RunOutcome::from_json` へ渡す。この形は test 側で再現し、実 manifest の stage と照合する。
形の正本は al-gui service の `stage_statuses` にある。test 側の再現は形の変更を自動検出
しない既知の制約であり、形を変える場合は両方を同時に更新する。

## 4. al-desktop への Job/Run mount の判断

| 案 | 内容 | 判断 |
| --- | --- | --- |
| A | Job 一覧 tab を追加(`preferences.base_dir` の `JobService::list`) | 保留。画面の入力とする base_dir と Job 操作の扱い(AL-G4)が未決 |
| B | Run tab を追加(job 実行結果を表示) | 不可。desktop に job 実行操作と結果の入力経路が無い |
| C | 射影と Fake 層 mount の e2e まで(本 slice) | 採用 |

案 A は UI 契約の判断であり、coordinator の判断を待つ。案 C で射影の正しさは e2e で固定する。

## 5. RF443 と spectrogram_columns の複製

ruviform#443(Tile → SpectrogramColumn の列区間・count の上流契約)は OPEN で、上流に
公開規則が無い。`view::screen::spectrogram_columns` の `Spectrogram::tile` 等分割の複製は
維持し、上流 API が merge された時点で差し替える(本 slice では NotRun)。一時的な複製で
あることは `spectrogram_screen` の doc と PR 本文に明記する。

## 6. 責務と境界

- GPUI 型は `bin/al-desktop.rs` と gpui-test の parity test のみ。ViewSpec・Domain・CLI へは出さない。
- 数値の再計算はしない。期待値の導出は test 内の純関数に閉じる。
- 本 slice は非 test の production コードを変更しない(al-desktop・view の挙動は不変)。

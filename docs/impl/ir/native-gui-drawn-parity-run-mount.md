---
id: "IMPL-yAi6Kuqoh9s4HZcj0OFkFx"
type: "impl"
status: "completed"
title: "Native GUI drawn parity の Spectrogram・Label・Table 拡張と Job/Run 射影の実装記録(IR, 第5 slice)"
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
    - "../../tasks/native-gui-drawn-parity-run-mount.md"
  blocks: []
  supersedes: []
  superseded-by: []
  refines: []
  refined-by: []
  derives-from:
    - "../../plans/native-gui-drawn-parity-run-mount.md"
  derived-by: []
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# Native GUI drawn parity の Spectrogram・Label・Table 拡張と Job/Run 射影の実装記録(IR, 第5 slice)

## 実装内容

- **Ruviform rev の更新**: `node tools/update-ruviform-rev.mjs 3121f8e6…` で `Cargo.lock` の
  ruviform 20 source のみを `627e4bc2` から `3121f8e6` へ更新した(diff は source 行 20 組のみ、
  他の変更なし)。`Cargo.toml` は変更なし。`sync-ui-deps` で `ui/vendor` を同一 SHA に再生成した。
- **drawn parity の Spectrogram・Label・Table 照合**(`tests/gpui_parity.rs`): 8 frame × 4 bin の
  `VecSpectrum` から `Spectrogram::tile`(4 列)で作る fixture を使う。frame 2・3 を全 NaN(gap 列)、
  frame 4・5 の bin 1 を NaN(非有限セル)とした。期待 cell は spec payload の規則(`count != 0` の列、
  viewport 内、非有限 magnitude を除外、intensity は `magnitude_range` で正規化して clamp)だけから
  導出し、位置・intensity を `1e-6` で照合する。Label は text、Table は header と格子(欠損セル
  `None` の位置を保つ)を照合する。
- **Job/Run の射影 e2e**(`tests/view.rs::real_dataset_produces_view_models`): 実 job の `JobRow` を
  `job_list` で 1 行の Table にし、実 manifest の stage を `al.job.run` の stage 表示 JSON と同じ形で
  `RunOutcome` へ渡して `run_screen` を得る。状態テキスト `job <id>: completed N/N, failed 0` と
  stage 表(N 行、全行 completed)を照合し、`ViewHost` へ mount して `stages` key を観測する。
- **文書 chain**: spec・design・plan・test-spec・tasks を追加し、managed index(designs・plans・specs・
  tasks・test-specs)を `doc_maintenance.js apply` で再生成した。

## 判断と未達

- **al-desktop への Job/Run tab 追加は未達**: 入力源が無い(起動契約は `<dataset-root> <input-id>` のみ、
  Job 一覧の base_dir と Run 結果の経路が画面に未接続)。UI 契約の判断(base_dir の扱い、Job 操作の
  AL-G4 での扱い)を coordinator へ送る。射影と Fake 層 mount の e2e で代替検証した。
- **RF443 の上流契約への移行は未達(NotRun)**: ruviform#443 は OPEN で上流 API が無い。
  `view::screen::spectrogram_columns` の複製を維持した。
- **Spectrogram の theme token は未実施**(ruviform#434 F3)。
- **実 OS window の表示確認は NotRun**。
- **Job/Run e2e の Run 結果の形は test 側で再現している**: `stage_statuses`(al-gui service、非公開)と
  同じ形を test で組むため、形の変更は自動検出しない。形を変える場合は両方を同時に更新する。

## 検証結果(merge 前 HEAD、working tree で実行)

| コマンド | 結果 |
| --- | --- |
| `mise run fmt` | pass |
| `mise run check` | pass |
| `mise run lint`(biome + clippy `-D warnings`) | pass(biome の warning は `ui/` 配下の既存のもの) |
| `mise run test`(workspace) | pass(failed 0 の全 test binary。view 12 件、analysis_screen 5 件 ほか) |
| `mise run build` | pass |
| `mise run check-ui-gpui` / `lint-ui-gpui` | pass |
| `mise run test-ui-gpui` | pass(gpui_parity 8 件、うち drawn 照合 3 件を追加) |
| `mise run check-cli-headless` | pass |
| `mise run check-ruviform-lock` | pass(`3121f8e6`、20 packages) |
| `mise run check-ruviform-rev`(`--require-vendor` 相当) | pass(REV match) |
| `mise run sync-ui-deps` | pass(ui/vendor を 3121f8e6 で同期) |
| `mise run check-ui` / `test-ui` | pass(`mise run setup-ui` で `ui/node_modules` を復元後。node:test 112 件 pass・fail 0) |
| GPUI 境界 | `ruviform_ui_gpui` は `bin/al-desktop.rs` と gpui-test 限定の `tests/gpui_parity.rs` のみ |

- 数値 oracle(Waveform bit 同一・gap の NaN・count 0・Meter peak)は既存テストが非回帰で通る。
- `setup-ui` は worktree に `ui/node_modules` が無かったための復元で、依存定義は変更していない。

## 独立レビュー(review-protocol)の結果

- 独立した read-only サブエージェント(`subagent_explore`)が working tree の diff と upstream 3121f8e6 の
  契約(`spectrogram_cell_plan`・`table_plan`・`drawn_plans`・`CellValue` の Display)を照合した。
- 判定: **blocker なし**(`VERDICT: no-blocker`)。期待値の規則は上流と一致し、fixture が gap と非有限セル
  を含むことを assert で確かめている。Table の欠損セル位置、e2e の borrow と mount も問題なし。
- レビュアーは shell で `git diff` を実行できなかったため、Cargo.lock の無関係な変更の不在は親で確認した
  (`git diff Cargo.lock` は 20 組の source 行のみ)。

## 逸脱・残課題

- **Job/Run の al-desktop mount**: 上記のとおり未達。
- **RF443**: 複製を維持。ruviform#443 merge 後に `spectrogram_columns` を上流 API へ差し替える。
- **Spectrogram の読み出し上限**: 先頭 4096 STFT frame・128×512 セル(従来どおり)。
- **merge 後 HEAD の最終検証は NotRun**: マージは本ワーカーの範囲外。coordinator の merge 後に実施する。

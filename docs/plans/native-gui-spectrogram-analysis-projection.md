---
id: "PLAN-Jt6Pq2Zx8UvR4mYbN1kCfG"
type: "plan"
status: "draft"
title: "Native GUI Spectrogram・Job/Run/Analysis の ViewSpec 射影と drawn parity の実装計画"
created: "2026-11-08"
updated: "2026-11-08"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/audio-lens/issues/166"
  changes:
    added:
      - type: "file"
        path: "crates/al-gui/src/view/analysis_screen.rs"
      - type: "file"
        path: "crates/al-gui/src/view/run_screen.rs"
      - type: "file"
        path: "crates/al-gui/src/view/workspace.rs"
      - type: "file"
        path: "crates/al-gui/tests/spectrogram.rs"
      - type: "file"
        path: "crates/al-gui/tests/analysis_screen.rs"
      - type: "file"
        path: "docs/test-specs/native-gui-spectrogram-analysis-projection.md"
      - type: "file"
        path: "docs/tasks/native-gui-spectrogram-analysis-projection.md"
      - type: "file"
        path: "docs/impl/ir/native-gui-spectrogram-analysis-projection.md"
    modified:
      - type: "file"
        path: "Cargo.lock"
      - type: "file"
        path: "crates/al-gui/src/view/mod.rs"
      - type: "file"
        path: "crates/al-gui/src/view/screen.rs"
      - type: "file"
        path: "crates/al-gui/src/view/job.rs"
      - type: "file"
        path: "crates/al-gui/src/bin/al-desktop.rs"
      - type: "file"
        path: "crates/al-gui/tests/view.rs"
      - type: "file"
        path: "crates/al-gui/tests/gpui_parity.rs"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "../designs/native-gui-spectrogram-analysis-projection.md"
  implemented-by:
    - "../tasks/native-gui-spectrogram-analysis-projection.md"
  depends-on:
    - "../designs/native-gui-spectrogram-analysis-projection.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "https://github.com/scarecrow173/ruviform/issues/434"
    - "https://github.com/scarecrow173/ruviform/issues/435"
    - "https://github.com/scarecrow173/ruviform/issues/436"
    - "https://github.com/scarecrow173/ruviform/issues/442"
    - "https://github.com/scarecrow173/ruviform/issues/443"
  refines: []
  refined-by: []
  derives-from:
    - "../designs/native-gui-spectrogram-analysis-projection.md"
  derived-by: []
  verifies: []
  verified-by:
    - "../test-specs/native-gui-spectrogram-analysis-projection.md"
  references: []
  defers: []
  deferred-by: []
---

# Native GUI Spectrogram・Job/Run/Analysis の ViewSpec 射影と drawn parity の実装計画

## 前提

- audio-lens の基点は `origin/main` の `eb10b9d`(PR #170 merge 後)。新 branch は
  `scarecrow173/al166-screen-mount-s4`。
- Ruviform の基点は `627e4bc2`(#440 Label/Table・#441 Spectrogram・#439 drawn)。
  `node tools/update-ruviform-rev.mjs 627e4bc2…` で `Cargo.lock` の 20 package を更新する。
- 設計は `docs/designs/native-gui-spectrogram-analysis-projection.md` の D1〜D5 に従う。

## 作業分割

1. Ruviform rev 更新(`Cargo.lock` のみ)と `check-ruviform-lock` / `check-ruviform-rev` / `sync-ui-deps`。
2. Spectrogram 射影: `view::screen::spectrogram_screen`(Tile → SpectrogramSpec)。テストを先に書く。
3. Job/Run/Analysis 射影: `view::run_screen`・`view::analysis_screen`・`view::mod` の共通 helper。
4. Tabs 画面: `view::workspace::workspace_screen`、`al-desktop` の mount 差し替え、e2e 検証。
5. drawn parity: `tests/gpui_parity.rs` に Fake 期待値との照合を追加。
6. 上流ギャップの Issue 起票(RF#442・RF#443)と文書 chain。
7. 検証(mise 正規タスク)と review-protocol(独立レビュー)。
8. commit・PR(日本語、`Refs #166`)。merge 後 HEAD で最終検証。

## File map

| file | 変更 |
| --- | --- |
| `Cargo.lock` | ruviform 20 package の source を `627e4bc2` へ |
| `crates/al-gui/src/view/screen.rs` | `spectrogram_screen`・列写像(`spectrogram_columns`)を追加 |
| `crates/al-gui/src/view/mod.rs` | `tag`・`text_cell`・`num_cell`・`label`・`table`・`panel` の共通 helper |
| `crates/al-gui/src/view/job.rs` | private `tag` を共通 helper へ置換 |
| `crates/al-gui/src/view/run_screen.rs` | 新規。Run の状態 Label・stage 表・job 一覧 |
| `crates/al-gui/src/view/analysis_screen.rs` | 新規。events 節・alignment 節・Analysis 画面 |
| `crates/al-gui/src/view/workspace.rs` | 新規。dataset → Tabs(Source/Spectrogram/Analysis) |
| `crates/al-gui/src/bin/al-desktop.rs` | Source 単独 mount を `workspace_screen` の mount へ置換 |
| `crates/al-gui/tests/spectrogram.rs` | 新規。列写像・gap・区間規則・mount |
| `crates/al-gui/tests/analysis_screen.rs` | 新規。Run/Job/Analysis の Table・Label・欠損・上限・mount |
| `crates/al-gui/tests/view.rs` | e2e に workspace(3 tab)の mount を追加 |
| `crates/al-gui/tests/gpui_parity.rs` | drawn(Meter・TimeAxis・Waveform)の Fake 期待値照合を追加 |
| `docs/**` | 本 slice の文書 chain と README 索引 |

## 依存関係

- 1 → 2・3(並行可)→ 4 → 5。6 は実装結果を反映するため最後に確定する。
- 5 は Ruviform の `GpuiObserver::drawn`(rev `627e4bc2`)に依存する。
- 2 の区間規則は上流 `Spectrogram::tile` の実出力に対するテストで固定する。

## リスク

- **列区間の複製**: 上流の等分割規則と式がずれると、テストが検出できるのは規則を固定したケースだけ。
  割り切れない span・オフセット付き要求の両方をテストする(ruviform#443 で解消条件を記録)。
- **Tabs の非 active 子は mount されない**: 全画面を一度の mount で検証できない。tab 切替ごとの mount で検証する。
- **読み出しの重さ**: Spectrogram の読み出しは STFT frame 4096 までに制限する。`read_tile` は範囲全体を
  読むため、上限を上げると一時メモリが増える。
- **drawn の期待値**: 描画規則の写しが上流とずれると、parity が偽の不一致を出す。規則は contract の
  式をそのまま写し、空の期待値を通さないことで偽陽性を防ぐ。
- **events/candidates の不在**: 多くの dataset では events が無い。節単位の fallback で alignment 表は残す。

## ロールバック

- `Cargo.lock` を `git checkout -- Cargo.lock` で戻し、`mise run sync-ui-deps` で vendor を再生成する。
- 射影の追加は新規ファイルと `mod.rs`・`screen.rs`・`al-desktop.rs` の差分のみ。`al-desktop.rs` を
  `source_screen` の直接 mount へ戻せば従来の Source 画面に戻る。

## 検証マトリクス

| 検証 | 対象 | 手段 |
| --- | --- | --- |
| 形式 | 全体 | `mise run fmt` |
| 型・lint | 全体 | `mise run check` / `mise run lint` |
| 単体・統合 | al-gui 全体 | `mise run test`(workspace 全件) |
| 上流固定 | Cargo.lock | `mise run check-ruviform-lock` / `check-ruviform-rev` / `sync-ui-deps` |
| GPUI | gpui-test feature | `mise run check-ui-gpui` / `lint-ui-gpui` / `test-ui-gpui` |
| 境界 | headless 依存木 | `mise run check-cli-headless` |
| UI | TypeScript lane | `mise run check-ui` / `test-ui` |
| ビルド | 全体 | `mise run build` |

---
id: "IMPL-XreMEx2yJinBr1n3m0A06o"
type: "impl"
status: "completed"
title: "Native GUI Waveform/Seek 画面 ViewSpec 射影 実装記録(IR)"
created: "2026-11-08"
updated: "2026-11-08"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/audio-lens/issues/166"
  changes:
    added:
      - type: "code"
        path: "crates/al-gui/src/view/screen.rs"
      - type: "test"
        path: "crates/al-gui/tests/screen.rs"
    modified:
      - type: "code"
        path: "crates/al-gui/src/view/source.rs"
      - type: "code"
        path: "crates/al-gui/src/view/mod.rs"
      - type: "lockfile"
        path: "Cargo.lock"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/tasks/native-gui-waveform-seek-screen.md"
  implemented-by: []
  depends-on:
    - "docs/designs/native-gui-waveform-seek-screen.md"
    - "docs/plans/native-gui-waveform-seek-screen.md"
    - "docs/tasks/native-gui-waveform-seek-screen.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/specs/native-gui-waveform-seek-screen.md"
    - "docs/test-specs/native-gui-waveform-seek-screen.md"
  refines: []
  refined-by: []
  derives-from: []
  derived-by: []
  verifies: []
  verified-by: []
  references:
    - "https://github.com/scarecrow173/ruviform/commit/a57cd3d3bdae2b3dfde02bf4b496132bae7f3235"
    - "https://github.com/scarecrow173/ruviform/issues/421"
    - "https://github.com/scarecrow173/ruviform/issues/426"
  defers: []
  deferred-by: []
metadata:
  experiments:
    adopted: []
    rejected: []
---

# Native GUI Waveform/Seek 画面 ViewSpec 射影 実装記録(IR)

## 実装したこと

- ruviform resolved SHA を `e8adcc09` から `a57cd3d3`(main HEAD)へ
  `node tools/update-ruviform-rev.mjs` で更新。`Cargo.lock` のみ変更。
  副作用として `cargo-platform` 0.3.3→0.1.9、`cargo_metadata` 0.23.1→
  0.19.2 へ下がった(上流 ruviform-ui-test が Issue #411 の MSRV 1.89
  整合のため 0.19 系へ固定しているための収束。意図した変更)。
- `WaveformView::read_frames` を抽出し、`window` と viewport が同じ読み出し
  範囲を使うようにした(clamp 本体は非公開 `frame_range`。`window` の読み出し
  範囲は不変)。
- `view::screen::waveform_seek` を追加: 解決済み `ViewBin` を
  `WaveformBin` へ 1 対 1 写像し、source 全長の `transport.seek`
  Seek と Panel(`audio-view`)を組み立てる。
- `tests/screen.rs` に Fake 層(ViewHost headless)の 8 件を追加(PR 初版 6 件 + レビュー修正で 2 件)。

## 検証結果(この作業ツリーで実行)

| コマンド | 結果 |
| --- | --- |
| `mise run check-ruviform-lock` | pass(a57cd3d3) |
| `mise run check-ruviform-rev` | pass(lockfile-primary) |
| `mise run sync-ui-deps` | pass(ui/vendor 再生成) |
| `mise run check` | pass(更新直後) |
| `mise run fmt` | pass(`cargo fmt --all` 適用後) |
| `mise run lint` | pass(biome は既存 warning のみ、clippy `-D warnings` pass) |
| `mise run test` | pass(workspace 全件。ignored 2 は既存) |
| `mise run check-ui-gpui` | pass |
| `mise run lint-ui-gpui` | pass |
| `mise run test-ui-gpui` | pass |
| `mise run check-cli-headless` | pass(headless 10 crate に GUI 依存なし) |
| `mise run setup-ui` / `check-ui` / `test-ui` | pass(112 tests) |

## 逸脱・残課題

- GPUI driver 上の projection・intent parity は未実施(NotRun)。
  `ruviform-ui-gpui` の観測 seam が crate 内 `pub(crate)` のみのため、
  consumer test から Fake と同じ観測を取れない。上流 #421/#426 で扱う。
- `al-desktop` の window root は従来の fixture surface のまま。product
  screen を mount していない。
- TimeAxis / Meter / Spectrogram の ViewSpec 射影は未着手(#421 の
  範囲)。
- doc index の `rebuild-index` 提案は既存 hand-curated 行の churn を
  避けるため適用せず、本件の行のみ手で追記した。

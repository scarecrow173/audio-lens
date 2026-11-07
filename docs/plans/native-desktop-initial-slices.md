---
id: "PLAN-yVtTxHenQy8bBk2Sd2ZwdH"
type: "plan"
status: "approved"
title: "Native desktop 初期 slice AL-G1〜G3 実装計画"
created: "2026-11-07"
updated: "2026-11-07"
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
  implements:
    - "../designs/native-desktop-initial-slices.md"
    - "../specs/native-desktop-initial-slices.md"
  implemented-by: []
  depends-on:
    - "../specs/native-desktop-initial-slices.md"
    - "../designs/native-desktop-initial-slices.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "../test-specs/native-desktop-initial-slices.md"
    - "../tasks/native-desktop-initial-slices.md"
  refines: []
  refined-by: []
  derives-from:
    - "../designs/native-desktop-initial-slices.md"
  derived-by: []
  verifies: []
  verified-by:
    - "../test-specs/native-desktop-initial-slices.md"
  references: []
  defers: []
  deferred-by: []
---

# Native desktop 初期 slice AL-G1〜G3 実装計画

## File Map

新規:

| file | 責務 |
| --- | --- |
| `crates/al-gui/src/desktop.rs` | `DesktopSession`/`DesktopPolicy` — driver-neutral session |
| `crates/al-gui/src/theme.rs` | product DTCG document 埋め込み・`resolve(mode)` |
| `crates/al-gui/src/preferences.rs` | `GuiPreferences`/`ThemeMode`/`WindowPrefs` + load/save/path |
| `crates/al-gui/src/view/mod.rs` | view module 集約・`ViewError` |
| `crates/al-gui/src/view/job.rs` | `JobRow`/`InputRow`/`StageRow`/`JobDetail` |
| `crates/al-gui/src/view/analysis.rs` | `EventRow`/`AlignmentRow`/`FeatureTable` |
| `crates/al-gui/src/view/source.rs` | `WaveformView`/`SpectrumView`/`TimeMap`/`StreamTimeView` |
| `crates/al-gui/theme/base.tokens.json` | primitive 層(light/dark) |
| `crates/al-gui/theme/product.tokens.json` | product 層 semantic token |
| `crates/al-gui/src/bin/al-desktop.rs` | GPUI composition root(feature `gpui`) |
| `crates/al-gui/tests/desktop.rs` | FakeDriver lifecycle fixture |
| `crates/al-gui/tests/theme.rs` | theme resolve + preferences test |
| `crates/al-gui/tests/view.rs` | view model fixture(実 dataset 経由) |
| `docs/{discovery,specs,designs,plans,test-specs,tasks,impl/ir}/native-desktop-initial-slices.md` | doc graph |

変更:

| file | 変更 |
| --- | --- |
| `crates/al-gui/Cargo.toml` | 依存追加(ruviform-ui-theme/-gpui/-test/-media-core、al-core/al-locate/al-media、serde/serde_json/tempfile)、`gpui` feature、`[[bin]] al-desktop` |
| `crates/al-gui/src/lib.rs` | `desktop`/`theme`/`preferences`/`view` 公開 |
| `mise.toml` | `check-ui-gpui`/`lint-ui-gpui`/`test-ui-gpui` タスク追加 |
| `Cargo.lock` | 依存解決の自動更新 |
| `docs/*/README.md` | `node tools/gen-doc-index.mjs` 再生成 |

## Task Breakdown

### P0(必須)

1. doc graph(discovery→spec→design→plan→test-spec→task)作成 +
   `gen-doc-index` 再生成。
2. `al-gui` 依存・feature 配線(Cargo.toml)+ `lib.rs` 公開。
3. AL-G1: `desktop.rs` + FakeDriver fixture + `al-desktop` bin +
   gpui mise lane。
4. AL-G2: token 文書 + `theme.rs` + `preferences.rs` + 単体テスト。
5. AL-G3: `view/*` + 実 dataset fixture。
6. IR(`docs/impl/ir/native-desktop-initial-slices.md`)を実装後に確定。
7. 独立 review A/B → Validator → Judge、P0/P1 blocker 修正。
8. 検証: `fmt`/`check`/`lint`/`test`/`check-ui`/`test-ui`/
   `check-cli-headless`/`check-ui-gpui`/`lint-ui-gpui`/`test-ui-gpui`。
9. origin/main merge、union、再生成、最終 review/verify、
   日本語 Conventional Commit → push → PR。

### P1(重要だが別 slice 可)

- `resolve()` の alias/mode カバレッジ拡大(component 層)。
- `GuiPreferences` の永続化経路の GUI 側利用。

### P2(nice-to-have)

- `WindowPrefs` の position 保存。

## Verification Strategy

- 最低レイヤ: 単体テスト(preferences parse、TimeMap 算術、
  EventRow 変換)。
- 外部観測: FakeDriver での DesktopSession lifecycle、実 WAV/AVI
  fixture → job run → dataset → view model の e2e fixture、
  `al-desktop --help` 相当の compile 検証(mise gpui lane)。
- 比例原則: AL-G4/G5 の実 OS/visual/数値正式受入は NotRun と明記。

## Risks & Mitigations

- **gpui crate の Windows/Linux 依存が check を壊す** → feature
  `gpui` 内に閉じ込め、既定 check では解決されないことを
  `mise run check` で担保。
- **ruviform lockfile SHA の drift** → `check-ruviform-rev` で
  現 pin を検証。本件の変更で pin を書き換えない。
- **FakeDriver の semantic fixture との不整合** →
  `inject`/`pump`/`close` veto API のみ使い、内部実装を仮定しない。
- **dataset fixture コスト** → `short_sine`/`short_pair` 程度の
  小さい WAV + dataset stage のみ。

## Effort

実装 + doc + review/verify を 1 PR に集約。AL-G4/G5 は後続 Issue
(Issue #166 に残 gate として記録)。

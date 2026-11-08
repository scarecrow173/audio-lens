---
id: "DESIGN-034U1jgDOqHhX2wdbK7ElK"
type: "design"
status: "approved"
title: "ruviform pin 54439fdc 追従と Spectrogram intensity 実窓検証の設計"
created: "2026-11-08"
updated: "2026-11-08"
owners: []
relations:
  source: []
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
    - "docs/plans/ruviform-463-spectrogram-intensity.md"
    - "docs/test-specs/ruviform-463-spectrogram-intensity.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/specs/ruviform-463-spectrogram-intensity.md"
  derived-by: []
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# ruviform pin 54439fdc 追従と Spectrogram intensity 実窓検証の設計

## Context

AL#179 の実窓検証で Spectrogram canvas が `viz.waveform` 系 2 色に
飽和する症状が観測され、ruviform#463 として起票・ruviform 側で
修正(PR #466、merge commit `54439fdc`)された。consumer 側の
AudioLens は pin 追従と実窓での最終確認を行う。AudioLens 側の
projection/spec 生成は ruviform 契約内で正しく、変更対象は
`Cargo.lock` の解決 SHA と `ui/vendor` のみ。

## Scope

### In Scope

- `tools/update-ruviform-rev.mjs` による `Cargo.lock` pin 更新と
  `mise run sync-ui-deps` による `ui/vendor` 同期。
- 実 OS window(al-desktop debug build)での Spectrogram tab 表示と
  pixel 解析による intensity 勾配の確認。証拠は
  `F:/repositry/audio-products/.devin/evidence/` 配下に保存。
- 既存回帰テスト(`spectrogram_tab_paint`、`gpui_parity`)の pass 確認。

### Out of Scope

- ruviform 側 renderer の変更、AL 側の描画コード変更、spec 解像度の
  変更、新規自動テストの追加。

## Components and Boundaries

- `tools/update-ruviform-rev.mjs` / `tools/sync-ruviform-ui.mjs`:
  pin 更新と vendor 同期の既存正規手順(変更しない)。
- `crates/al-gui` の spectrogram spec 生成・投影: 変更しない
  (AL projection は検証済みで正常)。
- ruviform-ui-gpui の Spectrogram painter: 変更しない
  (RF#463 で `surface_bg` 事前合成の不透明色に是正済み)。
- 実窓検証: `.devin/evidence/al179-canvas-paint/`・
  `al180-spectrogram-hang/` の既存手法(al-desktop 起動、
  `SetCursorPos`/`mouse_event` による tab 選択、`CopyFromScreen`
  capture、pixel 色解析)を再利用する。

## Data and Control Flow

- `node tools/update-ruviform-rev.mjs 54439fdc...` → `cargo update
  --precise` が `Cargo.lock` の ruviform source fragment を単一 SHA
  へ更新 → `mise run sync-ui-deps` が `ui/vendor` を同 SHA で再生成
  → `check-ruviform-lock`/`check-ruviform-rev` が整合を検証。
- 実窓検証: `al-desktop <base-dir> JOB-Zt7KXJqz7qf8fo9JBcmlX1` で
  起動 → Jobs で job 選択 → Dataset tab → workspace Spectrogram tab
  を pointer click → settle 後に capture → Spectrogram canvas 領域の
  pixel 色分布を解析し、中間色・複数色の存在を確認する。

## Non-Functional Constraints

- lockfile が ruviform SHA の唯一の正本(manifest に rev を書かない
  既存契約を維持)。
- 検証は `mise run` タスクが canonical。cargo/node 直接実行は補助証拠。
- 無関係な既存変更(未追跡ファイル等)は保持し stage/commit しない。

## Risks and Trade-offs

- pin 追従で他の ruviform 変更が混入する可能性 → `863a7a9d..54439fdc`
  の差分は RF#463 / PR #466 の 1 commit のみであることを確認済み。
- capture の DComp artifact(断続的な単色 frame)→ 複数回 capture し、
  `Responding=True` と実描画 frame の両方を記録する(al179/al180 で
  確立済みの手法)。
- fixture `JOB-Zt7KXJqz7qf8fo9JBcmlX1` の dataset が既存 evidence
  ディレクトリ(`al166-acceptance-verify/jobs/`)配下にあるため、
  base-dir をそこへ向けて再現性を確保する。

## Deferred Design Concerns

- なし。

## Verification Notes

- [x] `mise run check-ruviform-lock` / `check-ruviform-rev` /
  `sync-ui-deps` が pass すること。
- [x] `mise run` の fmt/check/lint/test/build/check-cli-headless/
  check-ui/test-ui/check-ui-gpui/lint-ui-gpui/test-ui-gpui が pass
  すること。
- [x] 実窓 pixel 解析で intensity 勾配(中間色・複数色)を確認し、
  証拠を `.devin/evidence/` に保存すること。

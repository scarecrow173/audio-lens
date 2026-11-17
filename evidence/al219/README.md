# evidence/al219 — Issue #219 GPUI 実 OS E2E lane 証拠

`al-gui-real-scenario`(test-only bin、`gpui-real` feature)の
lane 実行証拠。JSONL は `run` header 1 行 + case 毎 1 行の
`ScenarioReport` 射影(`targets` は session-local id のため
非出力)。

## ファイル

- `gui-real-scenario.jsonl` — `mise run test-ui-gpui-real` の
  実走出力(interactive Windows desktop session、
  `environment_kind=real-window`)。全 7 case `pass`。
- `gui-real-headless.jsonl` — `mise run
  test-ui-gpui-real-headless` の blocked-probe 出力。
  全 7 case `not_run` + `observed.environment_blocked`、
  `environment_kind=deterministic-backend`(real-window 非主張)。

## case catalog(7 件)

| scenario | case | required capability |
|---|---|---|
| al.desktop.jobs | job-select-resume | SemanticAction |
| al.source.seek | seek-commit | SemanticAction |
| al.real-os.input | os-input-preflight | RealOsPointerOrKeyboard |
| al.real-os.pointer | seek-pointer-click | RealOsPointerOrKeyboard |
| al.real-os.keyboard | seek-key-end | RealOsPointerOrKeyboard |
| al.real-os.visual | capture-frame-desktop | VisualCapture |
| al.real-os.visual | capture-frame-source | VisualCapture |

## 実行環境(2026-11-18 計測)

- OS: Windows(x86_64)、対話 desktop session あり
- ruviform pin: `cbcc83437616fdd7418da37183c6be66b6cfe225`
- lane: `mise run test-ui-gpui-real` → exit 0、全 case pass
  (pointer: win32-SendInput、capture: win32-printwindow
  backend、1280x800 / scale 1.0)
- headless: `mise run test-ui-gpui-real-headless` → exit 0、
  全 case not_run+environment_blocked

## 注意

- 本証拠は本 slice 実行環境での 1 回の実測。Issue #219 の
  受入確定(実走 Pass の十分性)は別途判断 — PR は `Refs #219`。
- headless/semantic の Pass を実機 Pass へ昇格しない
  (POLICIES §2.1)。`environment_kind` が `real-window` でない
  記録は実 OS window の証拠ではない。

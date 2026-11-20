# Drive the GUI scenario catalog

The GUI's verifiable behaviors are pinned as a 9-case scenario
catalog: the same scenario bodies run under three drivers — Fake
(headless `ViewHost`), GPUI (simulated adapter), and RealOs (a real
window on an interactive desktop). Semantic cases (job select, seek
commit, workspace tabs) run on all three; real-input, capture, and
window-lifecycle cases run only on the real-os lane, which always
executes the whole catalog in one session and records unreachable
environments as `NotRun` + `environment_blocked` instead of `Pass`.

## Sub-features

- `catalog-list` exposes the pinned cases via `al-ctl drive --list`
  (scenario/case ids, drivers, required capabilities); the list is
  test-pinned against `al-gui-real-scenario.rs` `CASES` and the
  `scenario_real.rs` id pins, so drift fails CI.
- `semantic-cases` — `al.desktop.jobs/job-select-resume`,
  `al.source.seek/seek-commit`, `al.dataset.workspace/workspace-tabs`
  (require `SemanticAction`; drivers fake, gpui, real-os).
- `real-os-input` — `al.real-os.input/os-input-preflight`
  (`RealOsPointerOrKeyboard`).
- `real-os-pointer` — `al.real-os.pointer/seek-pointer-click`
  (`RealOsPointerOrKeyboard`).
- `real-os-keyboard` — `al.real-os.keyboard/seek-key-end`
  (`RealOsPointerOrKeyboard`).
- `real-os-visual` — `al.real-os.visual/capture-frame-desktop` and
  `capture-frame-source` (`VisualCapture`).
- `real-os-window` — `al.real-os.window/close-veto-approve` (no
  declared capability).

## How to get to it (user POV)

- The catalog is the verification surface — a user reaches the
  underlying features through `al-desktop` itself; the catalog is
  reached by running the lanes.
- `al-ctl drive --list` enumerates cases; `al-ctl drive` runs lanes;
  the per-case feature files (`gui-desktop`, `gui-source-view`,
  `gui-dataset-workspace`) name each case's scenario id.

## Driving it with al-ctl drive

Preconditions:

- `mise run build al-cli` has produced `al-ctl`; `cargo` is on PATH
  (the lanes spawn `cargo test`/`cargo run` child processes).
- The real-os lane needs an interactive Windows desktop session; on a
  headless host use `--blocked-probe` or let the run self-block —
  both record `NotRun` + `environment_blocked`.

- **Catalog.** `al-ctl drive --list [--json]` prints the 9 cases.
- **Fake lane.** `al-ctl drive --driver fake` (all semantic tests) or
  `al-ctl drive scenario <id> --driver fake` (one scenario's filter);
  spawns `cargo test -p al-gui --test scenario`.
- **GPUI lane.** `al-ctl drive --driver gpui` or `scenario <id>`;
  spawns `cargo test -p al-gui --features gpui-test --test
  scenario_gpui`.
- **Real-OS lane.** `al-ctl drive --driver real-os [--out
  evidence/al<N>/gui-real.jsonl]` spawns `cargo run -p al-gui
  --features gpui-real --bin al-gui-real-scenario`; add
  `--blocked-probe` for the deterministic no-window self-check.
  `--json` on `al-ctl drive` wraps the receipt (pid, exit, stdout,
  stderr, `--out` record count); `--dry-run` prints the resolved
  command without spawning.
- **Canonical equivalents.** `mise run test-ui-gpui` (gpui lane),
  `mise run test-ui-gpui-real` (live real-os), and
  `mise run test-ui-gpui-real-headless` (blocked-probe) run the same
  bodies — `al-ctl drive` is the control surface, not a different
  lane.

## Gotchas

- The real-os lane has **no per-scenario filter** — `scenario <id>
  --driver real-os` is a usage error by design (one session runs the
  whole catalog).
- `--blocked-probe` and `--out` are valid only with `--driver
  real-os`; fake/gpui reject them.
- `drive` forwards nothing after `--` — the lane argv is built by
  `al-ctl`, not the caller.
- A scenario id that exists but lacks a fake/gpui filter (the
  `al.real-os.*` ids) is rejected with guidance, not silently skipped.
- Semantic Pass on fake/gpui never substitutes for real-window
  evidence — `NotRun` + `environment_blocked` is the honest headless
  record (`POLICIES.md` §3).

# Browse and drive jobs on the desktop screen

`al-desktop` opens a single window whose root is a three-tab bar —
Jobs, Run, Dataset. Jobs lists every `JOB-*` under the base dir and
selecting a row is the action that populates the other tabs: Run shows
that job's stage table with Resume/Cancel buttons, Dataset opens the
job's analysis workspace. A tab that cannot read its data shows a
reason label in place of its panel instead of failing the screen.

## Sub-features

- `desktop-tabs` switches Jobs/Run/Dataset via pointer tab-select,
  issued as a `tab-select {"index": n}` intent on target `al.desktop`
  (index < 3).
- `jobs-list` renders one row per `JOB-*` in base dir; row selection
  issues `select {"row": n}` on target `al.job` and selects the job.
- `run-status` shows `job <id>: completed X/Y, failed Z` plus the
  `stages` table (stage, status, duration_ms, error — failed stages
  keep their error text).
- `run-actions` offers `Resume` (issues `resume {"job_id"}` on
  `al.job`) and `Cancel` (issues `cancel`, no params) for the selected
  job.
- `job-arg` — `al-desktop [<base-dir> [<job-id>]]` takes the job id as
  the initial selection; `base-dir` defaults to the stored preference.
- `unavailable-tab` replaces an unreadable tab's panel with a reason
  label (`base_dir is not set`, `no job selected`, `job has no
  input`); the window still opens.

## How to get to it (user POV)

- Run `al-desktop [<base-dir> [<job-id>]]` (binary from
  `mise run build-ui-gpui`, or `cargo run -p al-gui --features gpui
  --bin al-desktop`). A real OS window opens on an interactive desktop
  session.
- Click a tab header to switch Jobs/Run/Dataset; click a `JOB-*` row
  in Jobs to select it; click Resume/Cancel on the Run tab.
- The same screen is exercised headlessly by the
  `al.desktop.jobs/job-select-resume` scenario case — the recipe
  below.

## Driving it with al-ctl drive

Preconditions:

- `mise run build al-cli` has produced `al-ctl` and `al-ctl doctor`
  reports `ok: true`.
- The fake and gpui lanes run on any host; the real-os lane needs an
  interactive Windows desktop session — on a headless host it
  honestly records `NotRun` + `environment_blocked`.

- **List the case.** `al-ctl drive --list` shows
  `al.desktop.jobs/job-select-resume` with
  `drivers=fake,gpui,real-os`.
- **Fake lane.** `al-ctl drive scenario al.desktop.jobs --driver fake
  --json` spawns `cargo test -p al-gui --test scenario --
  fake_desktop_jobs` as a child process. Exit `0`; the receipt
  captures pid, exit code, stdout/stderr. The scenario selects a job
  row through the mounted view and observes the select intent and the
  Run/Dataset panels updating.
- **GPUI lane.** `al-ctl drive scenario al.desktop.jobs --driver gpui
  --json` runs the same scenario body on the simulated GPUI adapter
  (`gpui_desktop_jobs` filter) — Pass means the same intents resolve
  on the real rendering stack.
- **Real-OS lane.** `al-ctl drive --driver real-os --out
  evidence/al<N>/gui-real.jsonl` opens a real window and runs the
  whole 9-case catalog in one session (no per-scenario filter). On a
  headless host run `al-ctl drive --driver real-os --blocked-probe`
  or let the live run self-block — record `NotRun` +
  `environment_blocked`, never promote a fake/gpui Pass to
  real-window evidence.

## Gotchas

- With no `--base-dir` and no stored preference, all three tabs show
  `base_dir is not set` — the window opening is not proof the screen
  is usable.
- Run and Dataset show `no job selected` until a Jobs row is selected
  or a `job-id` arg is passed; Dataset shows `job has no input` for a
  job without inputs.
- `tab-select` accepts only indexes 0–2 for the root tabs; the Run
  actions (`resume`/`cancel`) are intents, not synchronous job
  control — observe the job state separately (Run panel or
  `al inspect`).
- Dataset opens the job's `analysis/` — a job whose `dataset` stage
  did not complete has no workspace to show.

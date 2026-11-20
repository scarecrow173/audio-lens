# AudioLens verification map

This directory is the maintained source for verifying AudioLens's
user-facing behavior. Read the index before driving the app, then use
the matching feature file as the recipe.

Status: normalized. The map covers the full `al` CLI surface
(`cli-job-run`), the `al-desktop` GUI surfaces (`gui-desktop`,
`gui-dataset-workspace`, `gui-source-view`), and the pinned scenario
catalog the GUI surfaces are verified through
(`gui-scenario-catalog`).

## Baseline preconditions

- Build once per checkout: `mise run build al-cli` produces
  `target/debug/al` and `target/debug/al-ctl`.
- `mise run setup` has run (LFS objects under `samples/` materialized)
  when a recipe uses real fixture media.
- Run `al-ctl doctor` and require `ok: true` before driving.
- Use a disposable `--base-dir` per run (e.g. a temp dir). Never drive
  a job dir a different run owns.
- Record results under `evidence/al<N>/` with the verdict vocabulary
  of `POLICIES.md` §3.

## Driving conventions

- Start every recipe from the baseline state unless its preconditions
  say otherwise.
- Drive `al` through `al-ctl launch -- <al args>` (or invoke `al`
  directly); drive GUI scenario lanes through `al-ctl drive`. Treat
  every command as literal.
- Parse `job_id=` and `job_dir=` from `al run` stdout; pass them to
  later `inspect`/`resume`/`export` calls.
- Capture the action and the resulting state. Confirm side effects
  with a second read-only observation, not by trusting the write
  command's output.
- Restore or delete scratch state after a mutation. Do not remove
  proof artifacts during cleanup.

## Proof and skip reporting

- CLI proof includes the command, stdout, stderr, and exit code —
  `al-ctl launch --json` captures all four plus the spawned PID.
- Mutation proof includes a read-only second view (`al inspect`, or
  `al-ctl doctor --base-dir` showing the new `JOB-*` dir).
- A path that cannot run records `NotRun` with the reason (e.g.
  `environment_blocked` for a real-window drive on a headless host) —
  never report it as `Pass`.
- Scaffold a ledger with `al-ctl evidence scaffold --issue <N>` and
  append each action with `al-ctl evidence append --issue <N>
  --action <name> --command <cmd> --exit-code <n> --observed <text>`.

## Feature entry contract

Each feature file starts with an H1 title and one paragraph describing
the user-visible behavior, then exactly four H2 sections in this
order:

1. `Sub-features` lists short IDs with one line for each behavior.
2. `How to get to it (user POV)` lists every user entry point.
3. `Driving it with <harness>` starts with `Preconditions:` and uses
   labeled bullets pairing each user action with an exact command and
   an observable result.
4. `Gotchas` lists traps that can waste or invalidate a run.

Keep implementation details out of the map: name only user paths,
stable handles, required state, commands, and observable proof.

## Features

- [Run and inspect a job](./cli-job-run.md) covers `al run` through
  `inspect`, `resume`, and `export` — the CLI analysis-job journey.
- [Browse and drive jobs on the desktop screen](./gui-desktop.md)
  covers the `al-desktop` root tabs — Jobs row select, Run status and
  Resume/Cancel, tab navigation.
- [Read and scrub the Source view](./gui-source-view.md) covers the
  Source tab's waveform, seek commit, time-axis ruler, and level
  meter.
- [Inspect a dataset workspace](./gui-dataset-workspace.md) covers
  the Dataset tab's Source/Spectrogram/Analysis workspace and its
  degrade-to-label rule.
- [Drive the GUI scenario catalog](./gui-scenario-catalog.md) covers
  the pinned 9-case catalog and the fake/gpui/real-os lanes behind
  `al-ctl drive`.

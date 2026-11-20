# Run and inspect a job

Running an analysis job lets a user feed capture audio/video and
reference inputs into the pipeline, watch it produce a persisted job,
and confirm the result from a second read-only view (`al inspect`).
Resuming and exporting are the companion journeys on the same job.

## Sub-features

- `job-run` executes the stage plan and writes `manifest.json` plus
  artifacts under `<base-dir>/<JOB-*>/`.
- `job-stage-select` limits the plan via `--stages`; a plan missing a
  dependency is rejected before any stage runs.
- `job-resume` continues a job whose stages are pending or were
  interrupted, reusing completed stage output.
- `job-inspect` prints the persisted manifest as JSON without writing.
- `job-export` packages a completed job into `export/<name>/` inside
  the job dir.

## How to get to it (user POV)

- Run `al run --base-dir <dir> --input <role>:<path>[,<role>:<path>...]
  [--stages <stage>[,<stage>...]]` in a terminal.
- Run `al resume --base-dir <dir> --job-id <JOB-...>`,
  `al inspect --base-dir <dir> --job-id <JOB-...>`, and
  `al export --base-dir <dir> --job-id <JOB-...>` for the follow-on
  actions.
- The same jobs are listed and resumed from the `al-desktop` GUI
  desktop screen (Run tab) — the GUI journey is a separate feature
  file in a later slice.

## Driving it with al-ctl

Preconditions:

- `mise run build al-cli` has produced `al` and `al-ctl`.
- `al-ctl doctor` reports `ok: true` (`fixtures.samples` may be
  skipped by passing a `--repo` without samples only when the recipe
  synthesizes its own WAVs).
- A disposable base dir, e.g. `BASE=$(mktemp -d)/jobs`, plus a
  synthesized mono 48 kHz WAV `cap.wav` (the `tests/cli.rs` fixture
  convention — any valid PCM WAV ingests).

- **Launch the job.** Run `al-ctl launch -- run --base-dir $BASE
  --input capture_bug:$PWD/cap.wav --stages ingest`. Exit `0`; stdout
  prints `job_id=<JOB-...>` and `job_dir=<BASE>/<JOB-...>`. Record the
  PID from `al-ctl launch --json` if the receipt is needed.
- **Confirm the job exists.** Run `al-ctl doctor --base-dir $BASE
  --json`. The `base_dir` check reports `job_dirs: 1` — the read-only
  second observation that the write happened.
- **Inspect the manifest.** Run `al-ctl launch --json -- inspect
  --base-dir $BASE --job-id <JOB-...>`. The child stdout is a JSON
  manifest whose `stages` array records `ingest` as `Completed`.
- **Resume.** Run `al-ctl launch -- resume --base-dir $BASE --job-id
  <JOB-...>`. Exit `0`; per-stage lines report `Completed`/`Reused`
  for already-finished stages.
- **Export.** Run `al-ctl launch -- export --base-dir $BASE --job-id
  <JOB-...>` (requires the dataset stage completed — run the default
  stage plan, or include `dataset` and its dependencies in `--stages`,
  first). Stdout prints `export=<job-dir>/export/<name>`; the package
  exists on disk.
- **Proof.** Keep the launch receipts and the inspect JSON in
  `evidence/al<N>/`, then delete `$BASE` — the ledger, not the job
  dir, is the proof.

## Gotchas

- `al run` requires both `--base-dir` and `--input`; omitting either
  exits `2` with a named error, not a partial run.
- `--stages` must satisfy stage dependencies (`locate` needs ingested
  inputs, `export` needs `dataset`). An under-specified plan is
  rejected before writing — check the error, fix the plan, rerun.
- Compressed MP4 inputs do not ingest for audio analysis; derive the
  uncompressed AVI per `samples/README.md` first. WAV/PCM inputs
  ingest directly.
- `al export` on a job whose dataset stage has not completed fails —
  verify `stages` in the inspect output before driving export.
- `job_id` values are `JOB-*` strings; capture them from stdout rather
  than listing the base dir.
- `al inspect` is the read-only observation surface — prefer it over
  reading `manifest.json` by hand so the recipe also exercises the
  user path.

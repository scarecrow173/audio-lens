---
name: verify-audio-lens
description: Drive and verify AudioLens — the `al` job-runner CLI, the `al-desktop` GPUI app, and the `al-ctl` control CLI — the way a user does, with evidence in `evidence/al<N>/`. Use for /verify-audio-lens, runtime verification of a change against the real binaries, or when a task needs a proof that exercises the real user path.
---

# Verify AudioLens

AudioLens aligns game-capture video with reference inputs, measures
what happened to the audio, and emits structured analysis data. Its
user-facing surfaces are the `al` CLI (`run`/`resume`/`export`/
`inspect` — `crates/al-cli/src/main.rs`) and the `al-desktop` GPUI app
(`crates/al-gui/src/bin/al-desktop.rs`). `al-ctl` is the thin control
CLI this skill drives through.

Canonical entry point for builds and checks: `mise run <task>`
(`mise.toml`). Verdict vocabulary: `POLICIES.md` §3 — Required /
Deferred / NotApplicable crossed with Pass / Fail / NotRun /
Unsupported. A check that cannot run records `NotRun` with a reason,
never `Pass`.

## Launch

One-time build, then each drive is a fresh spawn — `al` is a one-shot
CLI with no server to keep alive.

```bash
mise install && mise run setup   # fresh checkout only: tools + hooks + LFS
mise run build al-cli            # -> target/debug/al and target/debug/al-ctl
al --help                        # ready check: exits 0, lists 4 subcommands
al-ctl doctor                    # health report (see Doctor)
```

- `mise run setup` restores `apm_modules/`, pre-commit hooks, and the
  git-lfs objects under `samples/` — required before driving real media.
- GUI build: `mise run build-ui-gpui` produces `target/debug/al-desktop`.
  Opening its real window requires an interactive OS desktop session.
- Ready means usable: `al --help` exits 0; `al-ctl doctor` reports
  `ok: true`.
- Teardown: nothing to stop — `al` exits per invocation. If a spawned
  child is still running, kill only the PID `al-ctl launch` reported.

## Doctor

`al-ctl doctor` is a read-only health report — it writes nothing.
Run it first whenever anything looks off.

```bash
al-ctl doctor                                    # text report
al-ctl doctor --json                             # machine-readable
al-ctl doctor --base-dir <scratch> --json        # include a job data dir
```

Checks (each `ok` / `fail` / `skipped`, with guidance on failure):

- `environment` — os / arch / pid of the probe.
- `tool` — `al-ctl` version (`CARGO_PKG_VERSION`).
- `git` — `rev-parse HEAD` SHA and dirty flag of `--repo` (default:
  discovered via `git rev-parse --show-toplevel` from cwd).
- `fixtures.samples` — `samples/` file count and LFS materialization
  (pointer-file scan). Fails with "run `mise run setup` or `git lfs
  pull`" when objects are pointers.
- `fixtures.dictionaries` — `dictionaries/capture-artifacts.v1.json`
  parses and carries `version` + `entries`.
- `base_dir` — `--base-dir` existence, job-dir (`JOB-*`) inventory.
  `skipped` unless `--base-dir` is given.
- `al_binary` — `al` resolution (`--al-path`, sibling of `al-ctl`,
  `<repo>/target/{debug,release}`, PATH) plus a read-only `al --help`
  probe.

Exit codes: `0` when every check is `ok`/`skipped`, `1` on any `fail`,
`2` on usage error. Process identity: `al-ctl launch` reports the
spawned child PID; doctor itself does not spawn anything besides the
`al --help` probe.

## Drive

Two wired surfaces: the `al` CLI through `al-ctl launch` (or
directly), and the GUI scenario lanes through `al-ctl drive` (which
spawns the lane's `cargo` command as a child process).

```bash
al-ctl launch -- run --base-dir <scratch> \
  --input capture_bug:<cap.wav>,source_asset:<src.wav> \
  --stages ingest,locate
al-ctl launch --json -- inspect --base-dir <scratch> --job-id <JOB-...>
al-ctl launch --dry-run -- run ...     # resolve + print, do not spawn
```

- `al run` prints `job_id=` and `job_dir=` on stdout — parse them.
  Stages: `ingest`, `locate`, `cheap_analysis`, `reference_analysis`,
  `expensive_analysis`, `dataset`, `export` (a stage plan missing a
  dependency is rejected).
- Input roles: `capture_bug`, `capture_reference`, `source_asset`
  (`samples/README.md`, `crates/al-cli/tests/cli.rs`).
- Deterministic inputs: synthesize tiny mono 48 kHz WAV files (the
  `tests/cli.rs` convention) or use the `samples/` LFS fixtures.
  Compressed MP4s need the ffmpeg-derived AVI described in
  `samples/README.md`; WAVs ingest directly.
- `al resume`/`export`/`inspect` take `--base-dir` + `--job-id`.
  `inspect` is read-only; `export` writes `export/<name>/` under the
  job dir.

GUI scenario catalog (driven via `al-ctl drive`):
`crates/al-gui/src/bin/al-gui-real-scenario.rs` `CASES` is the source
of truth — currently 9 cases (`al.desktop.jobs/job-select-resume`,
`al.source.seek/seek-commit`, `al.dataset.workspace/workspace-tabs`,
`al.real-os.input/os-input-preflight`,
`al.real-os.pointer/seek-pointer-click`,
`al.real-os.keyboard/seek-key-end`,
`al.real-os.visual/capture-frame-desktop` + `capture-frame-source`,
`al.real-os.window/close-veto-approve`). `al-ctl drive --list` prints
the catalog; the same bodies run under three drivers:

```bash
al-ctl drive --list [--json]                       # pinned 9-case catalog
al-ctl drive --driver fake [--json]                # all semantic tests, headless
al-ctl drive scenario al.desktop.jobs --driver fake    # one scenario's filter
al-ctl drive --driver gpui                         # gpui-test adapter lane
al-ctl drive --driver real-os [--blocked-probe] [--out <jsonl>]
al-ctl drive --driver fake --dry-run               # resolve + print, do not spawn
```

Drivers: Fake spawns `cargo test -p al-gui --test scenario` (headless,
any host); GPUI spawns `cargo test -p al-gui --features gpui-test
--test scenario_gpui`; RealOs spawns `cargo run -p al-gui --features
gpui-real --bin al-gui-real-scenario` — opt-in, interactive Windows
desktop only, always the whole catalog in one session (no
per-scenario filter). The same bodies are reachable as the canonical
mise lanes (`mise run test-ui-gpui`, `test-ui-gpui-real`,
`test-ui-gpui-real-headless`). On a headless host the real-os lane
records every case `not_run` with `observed.environment_blocked` —
the correct fail-closed result, not a lane failure; `--blocked-probe`
reproduces that deterministically without opening a window.

Not wired yet: real-window Pass proof (interactive session required —
run `al-ctl drive --driver real-os` in one).

## Evidence

- Ledgers live per-issue at `evidence/al<N>/` — see
  `evidence/al219/README.md` and `evidence/al127/` for the convention:
  `<sha>-verification.md` or `verification.md` (environment, commands,
  exit codes, result table, honest NotRun list) plus raw JSON/JSONL
  outputs.
- `al-ctl evidence` manages the ledger skeleton:

  ```bash
  al-ctl evidence scaffold --issue <N>          # create evidence/al<N>/ + actions.jsonl + verification.md (no overwrite)
  al-ctl evidence append --issue <N> \
    --action <name> --command <cmd> \
    --exit-code <n> --observed <text>           # append one JSON line to actions.jsonl
  ```

  `scaffold` is idempotent (existing files report `exists`); `append`
  fails when the ledger does not exist — scaffold first. Both accept
  `--repo`, `--dry-run`, and `--json`.
- Record Required/Deferred/NotApplicable separately from
  Pass/Fail/NotRun/Unsupported (`POLICIES.md` §3). `NotRun` +
  `environment_blocked` is the honest record for a real-window drive
  on a headless host.
- Proof standard: capture the action and the resulting state — the
  `al-ctl launch --json` receipt (spawned PID, exit code, captured
  stdout/stderr) plus a second read-only observation (`al inspect`
  on the produced job, or `al-ctl doctor --base-dir` showing the new
  `JOB-*` dir). Exercise the real user path; do not assert on
  internals the user never sees.

## Cleanup

- Kill only what the run started — `al-ctl launch` reports each
  spawned PID. Never kill by process name. `al` processes are
  short-lived and usually already exited.
- Delete scratch the run created: temp `--base-dir` job trees,
  generated input media, contents of `samples/_runs/` (gitignored).
- Never delete `evidence/` — proof artifacts survive teardown in the
  ledger location.
- No ports, sockets, or daemons exist to clean up.

## Helpers

`al-ctl` — verification control CLI (`crates/al-cli/src/bin/al-ctl.rs`).

```bash
cargo run -p al-cli --bin al-ctl -- <command>   # via cargo
target/debug/al-ctl <command>                   # after mise run build al-cli
al-ctl doctor [--repo <dir>] [--base-dir <dir>] [--al-path <path>] [--json]
al-ctl launch [--repo <dir>] [--al-path <path>] [--dry-run] [--json] -- <al args>
al-ctl drive --list [--json]
al-ctl drive scenario <scenario-id> --driver <fake|gpui>
              [--repo <dir>] [--dry-run] [--json]
al-ctl drive --driver <fake|gpui|real-os> [--repo <dir>]
              [--blocked-probe] [--out <path>] [--dry-run] [--json]
al-ctl evidence scaffold --issue <N> [--repo <dir>] [--dry-run] [--json]
al-ctl evidence append --issue <N> --action <name> --command <cmd>
              --exit-code <n> --observed <text>
              [--repo <dir>] [--dry-run] [--json]
al-ctl --help
```

- `--json` emits a single JSON object per run; `--dry-run` resolves
  the command without spawning (`launch`/`drive`) or writing
  (`evidence`).
- `drive` spawns the resolved lane (`cargo test`/`cargo run` on
  `al-gui`) as a child process — the subprocess boundary is what keeps
  GUI crates out of the headless `al-ctl` binary. Its receipt reports
  the spawned pid, exit code, captured stdout/stderr, and the real-os
  `--out` record count.
- `drive` rejects unknown drivers/scenarios, per-scenario filters on
  the real-os lane, `--blocked-probe`/`--out` on fake/gpui, and any
  trailing `--` args — the lane argv is `al-ctl`-built, not caller
  input.
- Exit `0` success, `1` failed doctor check / non-zero child /
  unapplied evidence write, `2` usage error / unresolvable binary.
  Errors name the fix (e.g. "build `al` with `mise run build
  al-cli`").
- Everything after `--` is passed verbatim to `al` (launch only —
  `drive`/`evidence` take no trailing args).

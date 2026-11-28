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

## Formal verification

The normative policy is `POLICIES.md` §6; this section is only the
AudioLens-specific operating procedure. The individual WAV/MP4/AVI proof
specifications in `docs/specs/formal-verification.md` have not yet been
migrated to that policy. Their migration is tracked in
`scarecrow173/audio-lens#274`; until then, treat them as unmigrated
per-property specifications rather than as the source of a fixed tool
promotion path.

Targets so far: the `al-media` WAV codec
(`crates/al-media/src/wav.rs`) and the MP4/AVI container parse
(`mp4.rs`/`avi.rs`/`format.rs`). Two lanes:

```bash
mise run test-formal        # proptest suite, native, always runnable
mise run test-formal-kani   # Kani bounded proofs, environment-gated
```

- `test-formal` runs `cargo test --locked -p al-media --test
  formal_wav --test formal_containers`. WAV properties:
  `decode_wav`/`wav_frame_count`/`read_wav_range` never panic on
  arbitrary bytes; `WavWriter` → `decode_wav` round-trip is lossless
  per encoding (documented contract: integer encodings clamp+round,
  floats cast); `wav_frame_count` agrees with `read_wav_range`; writer
  parameter validation rejects bad input. Container properties:
  `decode_mp4`/`decode_avi`/`detect_format` never panic on arbitrary,
  mutated, or truncated bytes; `detect_format` follows the signature
  rules; `write_mp4`/`write_avi` output round-trips through decode
  with declared facts, timing, and PCM intact; compressed or missing
  audio is a capability observation, not a container error.
- `test-formal-kani` runs `tools/run-formal-kani.mjs`, which env-detects
  a usable `cargo kani` (native Linux/macOS, or a WSL2 distro on
  Windows). In WSL it first mirrors the cargo-visible source set to
  `~/.cache/al-formal-kani/src` (keeps `target/` for the dep cache;
  no target-dir config or env is touched — artifacts stay in the
  mirror's default `./target`), then runs each registered
  `kani_proofs` proof as its own `cargo kani -p al-media --tests
  --harness <name>` sequentially, because kani-driver buffers
  per-harness results until process exit. Output is a JSON verdict
  record: `pass`/`fail` with the harness counts, or
  `not_run`/`unsupported` with the reason and provision steps.
  Environment absence is `not_run` + exit 0 — the honest record, not
  a lane failure (POLICIES §3). Setting `AL_KANI_LOG_DIR=<dir>`
  additionally writes each harness's raw output to
  `<dir>/kani-<harness>.txt` and the final JSON verdict to
  `<dir>/verdict.json`. Provision inside WSL2:
  `cargo install --locked kani-verifier && cargo kani setup`.
- The runner's `HARNESSES` array is the registry: 56 harnesses
  (13 WAV + 1 MP4 scan + 14 MP4 table chunk-map + 2 MP4 PCM declaration/capability + 2 MP4/AVI boundary + 1 AVI movi tag + 1 AVI movi composition + 1 AVI movi scan + 1 AVI stream kind + 1 AVI strh declaration + 1 AVI video strf declaration + 1 AVI audio strf declaration + 1 format signature + 3 AVI strf capture + 1 AVI `wb` concatenation + 1 AVI `strh` presence + 1 AVI clock start + 10 legacy containers). It always passes
  `-Z stubbing`; this only enables the attribute mechanism, so harnesses
  without stub attributes keep the same verification semantics.
  `AL_KANI_HARNESSES="a,b,c"` selects a comma-separated subset of
  registered names for a rerun. `AL_KANI_HARNESS_GROUP="wav-chunk-scan"`
  selects the two Required WAV-CHUNK-003/WAV-SCAN-004 proofs
  (`wav_chunk_boundary_step_exact` and
  `wav_chunk_scan_progress_bounded_exact`);
  `AL_KANI_HARNESS_GROUP="wav-range"` selects the two Required
  WAV-RANGE-008 replacement proofs (`wav_range_plan_valid_ranges_exact`
  and `wav_range_plan_invalid_ranges_error`);
  `AL_KANI_HARNESS_GROUP="mp4-avi-boundary"` selects the two Required
  MP4-BOX-002/AVI-CHUNK-001 proofs (`mp4_box_header_step_exact` and
  `avi_chunk_step_exact`); `AL_KANI_HARNESS_GROUP="mp4-scan"` selects
  the Required MP4-SCAN-003 proof
  (`mp4_box_scan_progress_bounded_exact`); `AL_KANI_HARNESS_GROUP="mp4-table-chunk-map"`
  selects the fourteen Required MP4-TABLE-005 stsc expansion/chunk-map materialization proofs
  (`mp4_stsc_chunk_samples_len0_bounded_exact`, `mp4_stsc_chunk_samples_len1_bounded_exact`,
  `mp4_stsc_chunk_samples_len2_chunks0_bounded_exact`, `mp4_stsc_chunk_samples_len2_chunks1_bounded_exact`,
  `mp4_stsc_chunk_samples_len2_chunks2_bounded_exact`, `mp4_stsc_chunk_samples_len2_chunks3_bounded_exact`,
  four `mp4_chunk_map_helper_*` per-chunk proofs, and four `mp4_chunk_map_end_to_end_*` composition proofs);
  `AL_KANI_HARNESS_GROUP="mp4-pcm-declaration"`
  selects the two Required MP4-PCM-DECL-006 declaration/capability proofs
  (`mp4_pcm_declaration_classification_exact` and `mp4_pcm_capability_width_exact`);
  `AL_KANI_HARNESS_GROUP="avi-movi-tag"`
  selects the Required AVI-MOVI-003 tag identity proof
  (`avi_movi_packet_tag_exact`); `AL_KANI_HARNESS_GROUP="avi-movi-composition"`
  selects the Required AVI-MOVI-003 movi entry composition proof
  (`avi_movi_entry_decision_exact`); `AL_KANI_HARNESS_GROUP="avi-movi-scan"`
  selects the Required AVI-MOVI-003 movi scan collection proof
  (`avi_movi_scan_collection_bounded_exact`); `AL_KANI_HARNESS_GROUP="avi-stream-kind"`
  selects the Required AVI stream existence/kind classification proof
  (`avi_stream_kind_classification_bounded_exact`); `AL_KANI_HARNESS_GROUP="avi-strh-declaration"`
  selects the Required AVI `strh` declaration field extraction/validation proof
  (`avi_strh_declaration_bounded_exact`); `AL_KANI_HARNESS_GROUP="avi-strf-declaration"`
  selects the Required AVI video `strf` BITMAPINFOHEADER declaration field extraction/validation proof
  (`avi_video_strf_declaration_bounded_exact`); `AL_KANI_HARNESS_GROUP="avi-strf-audio-declaration"`
  selects the Required AVI audio `strf` WAVEFORMATEX declaration field extraction/validation proof
  (`avi_audio_strf_declaration_bounded_exact`); `AL_KANI_HARNESS_GROUP="format-signature"`
  selects the Required container format signature detection proof
  (`format_signature_detection_bounded_exact`); `AL_KANI_HARNESS_GROUP="avi-strf-capture"`
  selects the Required AVI `strf` capture presence/copy/ordering proofs
  (`avi_strl_entry_classification_exact`, `avi_strf_capture_ordering_bounded_exact`, and `avi_strf_materialize_bounded_exact`);
  `AL_KANI_HARNESS_GROUP="avi-wb-concatenation"` selects
  `avi_audio_wb_concatenation_bounded_exact`;
  `AL_KANI_HARNESS_GROUP="avi-strh-presence"` selects
  `avi_strh_presence_bounded_exact`;
  `AL_KANI_HARNESS_GROUP="avi-clock-start"` selects
  `avi_stream_start_tick_exact`. The WAV proofs and the audio strf declaration proof are registered in
  `crates/al-media/src/wav.rs`; the MP4 scan/boundary/table chunk-map/PCM declaration and AVI boundary/tag/scan/stream declaration proofs are
  registered in `crates/al-media/src/mp4.rs` and
  `crates/al-media/src/avi.rs`; the format signature proof is registered in
  `crates/al-media/src/format.rs`. The two selection variables are mutually
  exclusive; unregistered names or groups are fail-closed `not_run`,
  and leaving both unset runs all 56. When `/usr/bin/time` is available
  the runner wraps each harness with GNU time, records elapsed ms and
  maximum resident set size in the raw log header, copies both into the
  JSON verdict details, and adds a lane-level `cost_summary` with the
  selected/verified proof counts, total elapsed ms, maximum observed RSS,
  and measurement availability.
- For a change to MP4 sample-table `stsc` validation, per-chunk expansion,
  `stsz` count matching, `stco`/`co64` chunk offsets, or chunk-map materialization,
  run `AL_KANI_HARNESS_GROUP=mp4-table-chunk-map AL_KANI_TIMEOUT_MS=600000 mise run test-formal-kani`.
  Confirm each raw proof result with unwinding assertions enabled and record elapsed
  time and peak RSS. The validation family proves chunks 0..3 and stsc entries 0..2 in the declared bounded
  scalar domain. The materialization family proves production `append_chunk_samples`
  for (spc, sizes.len) pairs (1,0), (1,1), (2,1), and (2,2) with full u32 sample sizes and full u64 chunk offsets,
  plus minimal production `build_chunk_map` empty/single/count-mismatch composition.
  It does not prove multi-chunk end-to-end materialization, spc 3/4, sizes 3..4,
  three or more samples, stts/ctts, larger tables, parser integration,
  PCM extraction/conversion, diagnostics, I/O, allocation, or public API no-panic.
  Rerun `mp4-scan`, `mp4-avi-boundary`, `mp4-pcm-declaration`, and native parser
  tests for neighboring behavior. See `evidence/al274-mp4-table-chunk-map-proof/`
  for the boundary and mutation.
- For a change to MP4 sound-entry PCM declaration, `pcmC` handling,
  `lpcm` flags/defaults, capability gating, or supported sample width, run
  `AL_KANI_HARNESS_GROUP=mp4-pcm-declaration AL_KANI_TIMEOUT_MS=600000 mise run test-formal-kani`.
  Confirm the raw proof result with unwinding assertions enabled and record
  elapsed time and peak RSS. The proof owns fixed scalar declaration/capability
  only; parser integration, sample tables, extraction, conversion, diagnostics,
  I/O, allocation, and payload semantics remain outside it. See
  `evidence/al274-mp4-pcm-declaration-proof/` for the boundary and mutation.
- For a change to AVI audio selection or append order, run
  `AL_KANI_HARNESS_GROUP=avi-wb-concatenation AL_KANI_TIMEOUT_MS=600000 mise run test-formal-kani`
  and check both the verdict and the raw proof result. Unlike the `strf`
  capture proof (PR #295/#296), this proof checks the PCM body supplied to
  `decode_waveformatex`, which can affect `IngestResult.audio` and stream
  availability/provenance. It calls the production selection and append loop
  for 3 streams and 3 packets, 0–2 bytes per body, using exact class/index/kind
  equivalence classes and no input assumptions. The parser/walker, dynamic
  tables and buffers, arbitrary-length copy, missing/empty product behavior,
  full declaration correspondence, timing, video, and PCM semantics are not
  proved. Rerun the native fixtures for the decode-shell connection and see
  `evidence/al274-avi-wb-concatenation-proof/` for the boundary and mutation.
- For changes to AVI `strh` handling in `strl`, run
  `AL_KANI_HARNESS_GROUP=avi-strh-presence AL_KANI_TIMEOUT_MS=600000 mise run test-formal-kani`.
  Confirm the raw proof result with unwinding assertions enabled and record
  elapsed time and peak RSS. Rerun the native parser tests and the
  `avi-strh-declaration`, `avi-strf-capture`, and `avi-wb-concatenation` groups.
  The proof covers the presence decision or first rejected entry for three
  representative entries, not the nested walker or complete declarations. See
  `evidence/al274-avi-strh-presence-proof/` for the boundary and mutation.
- For changes to AVI stream start ticks, run
  `AL_KANI_HARNESS_GROUP=avi-clock-start AL_KANI_TIMEOUT_MS=600000 mise run test-formal-kani`.
  Check the raw proof result and the budget (at most 600 seconds, peak RSS
  below 4 GiB). The harness checks the checked start product for every
  `u32` input pair against a widened unsigned oracle without assumptions.
  Rerun `avi-strh-presence`, `avi-strh-declaration`, `avi-stream-kind`,
  and `mise run test al-media` for the connection to `packet_ticks`.
  Do not extend this proof to remainder, packet accumulation, stream
  separation, PCM, or the full parser. See
  `evidence/al274-avi-clock-start-proof/` for the boundary and mutation.
- Two registered WAV proofs —
  `wav_reads_never_panics_unsigned` and
  `read_wav_range_never_panics_signed` — carry
  `#[kani::stub(std::fmt::format, crate::kani_stub_models::fmt_format_empty)]`.
  A successful result for either proof is therefore a conditional
  guarantee: the proof executes the implementation under that explicit
  boundary while excluding the panic/allocation behavior of formatted
  diagnostic-string construction. Do not report it as an unconditional
  whole-API no-panic proof.
- Historical note: the 2026-11-24 residual run (`1/9 harnesses
  verified`, raw output in `evidence/al256/residual/`) predates the
  21-harness split. The current canonical state after that split and
  the stub spike is `not_run` — 1/21 verified at a 25-minute
  per-proof budget: only `detect_format_signature_rules` verified and
  the other 20 timed out. The PR #276 spike's three representative
  proofs also all timed out. That inventory evidence and analysis live in
  `evidence/al274/` and `evidence/al274/stub-spike/`. The legacy
  whole-call range proofs were retired only after the exact
  replacement wav-range group passed; current WAV-RANGE-008 evidence
  lives in `evidence/al274-proof-redesign/`.
- These proofs execute the implementation under explicit bounds,
  assumptions, and stubs; they are not a model substituted for the
  implementation. When a proof fails or cannot complete, classify the
  product/model question and the independent invalidity questions
  separately: product defect, modeling error, invalid stub or
  assumption, resource limit, or environment. Check concrete
  counterexamples against the documented contract before changing
  either side.
- Mutation evidence lives in the ledger: a deliberately broken variant
  is injected into `crates/al-media/src/`, the failing proptest seed is
  captured, then the edit is reverted — committed product diff stays
  zero. Recorded runs are in `evidence/al256/`; current al#274 boundary
  evidence is in `evidence/al274/`, WAV-RANGE-008 arithmetic
  mutation evidence is in `evidence/al274-proof-redesign/`, and
  WAV-CHUNK-003/WAV-SCAN-004 boundary/scan mutation evidence is in
  `evidence/al274-chunk-scan-proof/`, AVI movi entry composition
  mutation evidence is in `evidence/al274-avi-movi-composition-proof/`,
  and AVI movi scan collection mutation evidence is in
  `evidence/al274-avi-movi-scan-proof/`, and AVI stream existence/kind
  classification mutation evidence is in `evidence/al274-avi-stream-kind-proof/`,
  and AVI strh declaration mutation evidence is in
  `evidence/al274-avi-strh-declaration-proof/`, AVI video strf declaration mutation evidence is in
  `evidence/al274-avi-strf-declaration-proof2/`, and AVI audio strf declaration mutation evidence is in
  `evidence/al274-avi-strf-audio-proof/`, and container format signature
  detection mutation evidence is in `evidence/al274-format-signature-proof/`, and AVI `strf`
  capture mutation evidence is in `evidence/al274-avi-strf-capture-proof/`.

Change-impact procedure:

1. Before reusing existing evidence, compare the product or test change
   with the affected property, input domain, boundary, assumptions,
   stubs, bounds, and oracle. If correspondence is unclear, the prior
   result is stale evidence, not a current Pass.
2. Rerun the affected native properties with `mise run test-formal`.
   For Kani, use the runner's existing subset selection
   (`AL_KANI_HARNESSES`) for affected registered harnesses, or the full
   lane when the boundary/shared dependency impact is broader.
3. Update the evidence ledger with target SHA, tool versions, execution
   conditions, guarantee scope, and verdict. Do not advance a verified
   sync commit until correspondence review and the required reruns are
   complete.

## Bench

Frozen kernel + pipeline benchmark with a ratio regression gate
(Issue #257):

```sh
mise run bench-frozen                          # gated real run vs pinned baseline
mise run bench-frozen --test                   # 1-rep smoke (threshold gates report_only; correctness defects still exit 1)
mise run bench-frozen --selftest               # regression injection -> gate must fail
mise run bench-frozen --write-baseline <path>  # emit an un-promoted baseline candidate
mise run bench-frozen -- kernel/               # cell filter (substring): kernel/, pipeline/, full_ncc, ...
```

- Two arms in `crates/al-service/benches/frozen_pipeline.rs`
  (`harness = false`, bench profile). Arm K `kernel/<method>`:
  production `al-locate` `default_methods()` vs a frozen naive NCC
  (coarse stride grid + refine — same shape as
  `FullNcc::locate_coarse_to_fine`), alternating A/B, 7 reps,
  median + range; the metric is `median(A)/median(B)` vs
  `min_ratio` pins. Arm P `pipeline/<container>` (WAV/AVI/MP4):
  real `JobService::run` (`ingest..dataset`, the same path
  `al run` takes) on deterministic seeded per-container fixture
  pairs (AVI embeds a synthetic video track); e2e + locate
  stage walls are normalized per container by the same-session
  frozen kernel median vs `max_norm_ratio` pins.
- The pipeline also gates the analysis payload fingerprint —
  stored per `pipeline/<container>` cell (baseline schema v2).
  Every job-dir `*.json` (skipping table-structure files,
  `export/`, `analysis/visualization/`, `video_thumbs.json`)
  normalized per `baseline/contracts.json` vocabulary (record-only
  keys dropped, `JOB|ART|IN|ALN` IDs canonicalized, absolute paths
  placeholdered, f64 quantized to `rel=1e-9 abs=1e-12`) then
  sha256'd per file. Rep-to-rep divergence and pin diffs count as
  errors — correctness is fail-closed alongside the perf ratio.
- Errors counted and gated: kernel = lag mismatch vs the known
  embedded offset (`±64` samples, both sides), `validity !=
  valid` rejects, full_ncc peak deviation vs the naive reference;
  pipeline = stage failures / `run` errors, accepted-lag
  mismatch, fingerprint diffs. Work counted: kernel = naive MACs
  + explored lags, pipeline = manifest artifacts + output bytes.
- Fail-closed: unreadable/un-promoted baseline, threshold breach,
  or errors>0 all fail the run. `baseline.al257.json` is
  read-only during gating; updates are two-phase —
  `--write-baseline` emits a candidate with `"approval": null`,
  a human promotes it to the pin path with `approval` filled.
  Repin when `measured_env` (machine, rustc, repo/ruviform SHA)
  or fixture constants change.
- Evidence follows `evidence/al257/`: run JSONL, `--selftest`
  gate-fail, the candidate + promoted pin, `decision.tsv`, and
  `verification.md`. Contract:
  `docs/test-specs/issue-257-frozen-bench.md`.
- This lane is manual/opt-in only — there is no CI bench lane to
  wire into; do not add one.

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

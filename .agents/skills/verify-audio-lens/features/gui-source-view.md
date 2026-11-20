# Read and scrub the Source view

The Source view is the waveform surface of one dataset input — a
waveform of min/max bins over the visible range, a seek slider
spanning the input's full duration, a time-axis ruler whose ticks fit
the viewport, and a level meter computed from the same bins. Scrubbing
the seek and releasing issues one `Commit` intent on target
`transport.seek`; cancelling the gesture issues nothing.

## Sub-features

- `waveform` renders the input's samples as min/max/rms bins over the
  viewport (`waveform` key, a11y label `Waveform`); gap bins keep
  `NaN`/zero-count rather than being filled in.
- `seek` is a horizontal slider ranged `0..duration_s` (`seek` key,
  a11y `Seek`); a finished drag emits exactly one
  `CommandIntent::Commit` on `transport.seek`, a detached gesture
  cancels.
- `ruler` draws the time axis (`ruler` key): tick step is the smallest
  candidate that keeps the visible span under ~10 intervals, every 5th
  tick is major; ticks landing outside the viewport are dropped.
- `meter` shows per-bin `peak = max(|min|, |max|)` plus rms (`meter`
  key, a11y `level`) — peak is derived from the waveform's own bins,
  never recomputed; gap bins stay `NaN`.

## How to get to it (user POV)

- In `al-desktop`, select a job with a completed dataset stage, open
  the Dataset tab — Source is the workspace's first tab.
- Drag the seek slider under the waveform to move the play position;
  the commit fires on release.
- The same screen is mounted standalone by the
  `al.source.seek/seek-commit` scenario case — the recipe below.

## Driving it with al-ctl drive

Preconditions:

- `mise run build al-cli` has produced `al-ctl` and `al-ctl doctor`
  reports `ok: true`.
- The fake and gpui lanes run on any host; real window input needs an
  interactive Windows desktop session.

- **List the case.** `al-ctl drive --list` shows
  `al.source.seek/seek-commit` with `drivers=fake,gpui,real-os`.
- **Fake lane.** `al-ctl drive scenario al.source.seek --driver fake
  --json` spawns `cargo test -p al-gui --test scenario --
  fake_source_seek`. Exit `0`; the scenario performs the scrub
  sequence and asserts exactly one `Commit` intent on
  `transport.seek`.
- **GPUI lane.** `al-ctl drive scenario al.source.seek --driver gpui
  --json` runs the parity test (`gpui_and_fake_run_the_same_scenarios`
  filter): the gpui adapter must emit the same seek commit intents the
  fake lane recorded.
- **Real-OS lane.** `al-ctl drive --driver real-os` drives the whole
  catalog; `al.real-os.pointer/seek-pointer-click` and
  `al.real-os.keyboard/seek-key-end` are the real-input variants of
  this surface (pointer click and End key on the seek). On a headless
  host they record `NotRun` + `environment_blocked`.

## Gotchas

- The Source tab fails with `job has no input` when the job's manifest
  has no inputs — select a job that completed `ingest`+`dataset`.
- A source with zero frames is rejected (`waveform source has no
  frames`); the panel bin count is capped by the meter's
  `MAX_METER_BINS`.
- Ruler construction rejects viewport spans larger than the coarsest
  tick step allows (~10 days) — a real input never approaches it, but
  a synthetic duration can.
- `transport.seek` is the commit target — a drive that emits zero or
  two commits for one scrub means the gesture semantics broke, not
  that the user dragged twice.

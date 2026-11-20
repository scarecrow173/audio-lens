# Inspect a dataset workspace

The Dataset tab of `al-desktop` opens the selected job's `analysis/`
as a workspace: a second tab bar with Source, Spectrogram, and
Analysis. Spectrogram renders the persisted STFT as a bounded tile
with a frame-range label; Analysis renders the `events/candidates`
rows and the `locate` alignments as typed tables. Each tab and each
analysis section is read independently — anything unreadable becomes
a reason label while the rest of the workspace still opens.

## Sub-features

- `workspace-tabs` switches Source/Spectrogram/Analysis via pointer
  tab-select, issued as `tab-select {"index": n}` on target
  `al.workspace` (index < 3).
- `source-tab` hosts the Source view — see `gui-source-view.md` for
  the waveform/seek/ruler/meter surface.
- `spectrogram-tab` shows `frames A..B of N` (`spectrogram-range`)
  above the `spectrogram` tile (128×512 max-aggregated columns over
  the first ≤4096 STFT frames, magnitude range 0..max).
- `analysis-tab` shows `events X of N` (`events-note`) plus the
  `events` table (event, kind, input, start_s, end_s, strength,
  confidence, validity) and the `alignments` table (source, capture,
  alignment, status, validity, confidence, agreeing_methods,
  capture_start, capture_end). Non-finite numbers render as missing
  cells, never as zero.
- `unavailable-section` — a tab or table section that cannot be read
  is replaced by a reason label; missing `events`/`locate` artifacts
  degrade only their own section.

## How to get to it (user POV)

- In `al-desktop`, select a job whose `dataset` stage completed and
  click the Dataset tab; click the inner Source/Spectrogram/Analysis
  tab headers.
- The workspace is exercised headlessly by the
  `al.dataset.workspace/workspace-tabs` scenario case — the recipe
  below.

## Driving it with al-ctl drive

Preconditions:

- `mise run build al-cli` has produced `al-ctl` and `al-ctl doctor`
  reports `ok: true`.
- The fake and gpui lanes build a synthetic dataset fixture in a temp
  dir — no `samples/` media needed.

- **List the case.** `al-ctl drive --list` shows
  `al.dataset.workspace/workspace-tabs` with
  `drivers=fake,gpui,real-os`.
- **Fake lane.** `al-ctl drive scenario al.dataset.workspace --driver
  fake --json` spawns `cargo test -p al-gui --test scenario --
  fake_dataset_workspace`. Exit `0`; the scenario tab-selects
  1→2→0 through the workspace and checks the mounted table cells
  against the `al`-written dataset (the cell oracle).
- **GPUI lane.** `al-ctl drive scenario al.dataset.workspace
  --driver gpui --json` runs the `gpui_dataset_workspace` filter —
  same tab sequence on the simulated adapter.
- **Real-OS lane.** `al-ctl drive --driver real-os` drives the whole
  catalog; headless hosts record `NotRun` + `environment_blocked`.

## Gotchas

- The workspace needs the job's `dataset` stage completed — an
  `ingest`-only job yields `job has no input`/`no job selected` style
  reason labels, not a workspace.
- Spectrogram reads only persisted STFT artifacts (bounded to the
  first 4096 frames); a dataset without them shows the reason label
  in that tab — that is honest output, not a broken screen.
- `tab-select` accepts only indexes 0–2 for the workspace tabs; the
  index targets tab position, not tab name.
- `events` rows come from a bounded query (first `MAX_TABLE_ROWS`);
  `events X of N` may show X < N on large datasets — that is the
  designed cap, not truncation.

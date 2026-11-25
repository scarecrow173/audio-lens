#!/usr/bin/env bash
# al#274 stub-spike ablation runner. Auxiliary evidence only; the canonical
# lane is `mise run test-formal-kani` through tools/run-formal-kani.mjs.
set -u
export PATH="$HOME/.cargo/bin:$PATH"

SRC="$HOME/.cache/al-formal-kani/src"
CACHE="$HOME/.cache/al-formal-kani/stub-spike/ablation"
EVIDENCE="/mnt/g/orca/workspaces/audio-lens/al274-kani-stub-spike/evidence/al274/stub-spike/ablation"
CAP="${1:-600}"
CONCURRENCY="${2:-3}"

mkdir -p "$CACHE" "$EVIDENCE"

sync_mirror() {
  mkdir -p "$SRC"
  find "$SRC" -mindepth 1 -maxdepth 1 ! -name target -exec rm -rf {} +
  tar -C /mnt/g/orca/workspaces/audio-lens/al274-kani-stub-spike -cf - \
    Cargo.toml Cargo.lock .cargo crates dictionaries | tar -C "$SRC" -xf -
}

rss_for() {
  local h="$1"
  ps -eo comm=,rss=,args= | awk -v h="$h" '
    ($1 == "cbmc" || $1 == "kani-driver") && index($0, h) > 0 { sum += $2 }
    END { print sum + 0 }
  '
}

cleanup_harness() {
  local h="$1"
  pkill -f "/${h}[.]out" 2>/dev/null || true
}

run_one() {
  local h="$1"
  local wd="$CACHE/$h"
  mkdir -p "$wd"
  local start end elapsed code peak rss
  start=$(date +%s)
  (
    cd "$SRC" || exit 99
    timeout "${CAP}s" env -u GIT_DIR -u GIT_WORK_TREE \
      cargo kani -p al-media --tests --harness "$h" -Z stubbing \
      --log-file "$wd/$h.verbose.log" >"$wd/$h.out" 2>&1
    echo $? >"$wd/exitcode"
  ) &
  local pid=$!
  peak=0
  while kill -0 "$pid" 2>/dev/null; do
    rss=$(rss_for "$h")
    printf '%s rss_kb=%s\n' "$(date +%H:%M:%S)" "$rss" >>"$wd/$h.rss.txt"
    if [ "$rss" -gt "$peak" ]; then peak="$rss"; fi
    sleep 10
  done
  wait "$pid"
  end=$(date +%s)
  elapsed=$((end - start))
  code=124
  [ -f "$wd/exitcode" ] && code=$(cat "$wd/exitcode")
  cleanup_harness "$h"

  local verdict="timeout"
  if grep -q 'VERIFICATION:- SUCCESSFUL' "$wd/$h.out" "$wd/$h.verbose.log" 2>/dev/null; then
    verdict="successful"
  elif grep -q 'VERIFICATION:- FAILED' "$wd/$h.out" "$wd/$h.verbose.log" 2>/dev/null; then
    verdict="failed"
  elif [ "$code" != "124" ] && [ "$code" != "0" ]; then
    verdict="error"
  elif [ "$code" = "0" ]; then
    verdict="completed_unknown"
  fi

  local maxiter events lines tailline
  maxiter=$(sed -n 's/.* iteration \([0-9][0-9]*\).*/\1/p' "$wd/$h.verbose.log" 2>/dev/null | sort -n | tail -1)
  lines=$(wc -l <"$wd/$h.verbose.log" 2>/dev/null || echo 0)
  events=$(grep -c -E '^(Unwinding|Not unwinding|aborting path|entering)' "$wd/$h.verbose.log" 2>/dev/null || true)
  tailline=$(tail -1 "$wd/$h.verbose.log" 2>/dev/null | sed 's/\/\\/g; s/"/\\"/g' | cut -c1-500)
  printf '{"harness":"%s","variant_kind":"%s","cap_s":%s,"wall_clock_s":%s,"exit_code":%s,"peak_rss_kb":%s,"verdict":"%s","symex":{"verbose_lines":%s,"event_lines":%s,"max_unwind_iteration":"%s","tail":"%s"},"log":"%s"}\n' \
    "$h" "$(variant_kind "$h")" "$CAP" "$elapsed" "$code" "$peak" "$verdict" \
    "$lines" "$events" "$maxiter" "$tailline" "ablation/$h.verbose.log" \
    >"$wd/$h.result.json"

  cp "$wd/$h.verbose.log" "$EVIDENCE/$h.verbose.log" 2>/dev/null || true
  cp "$wd/$h.out" "$EVIDENCE/$h.out" 2>/dev/null || true
  cp "$wd/$h.rss.txt" "$EVIDENCE/$h.rss.txt" 2>/dev/null || true
  cp "$wd/$h.result.json" "$EVIDENCE/$h.result.json" 2>/dev/null || true
  printf '%s\n' "$h" >>"$CACHE/completed.txt"
}

variant_kind() {
  case "$1" in
    spike_*_fmt) echo fmt-only ;;
    spike_*_alloc) echo alloc-only ;;
    spike_*_both) echo combined ;;
    *) echo baseline-control ;;
  esac
}

worker() {
  local lane="$1"
  local jobs_file="$CACHE/jobs.$lane"
  while IFS= read -r h; do
    [ -n "$h" ] && run_one "$h"
  done <"$jobs_file"
}

printf 'sync_start=%s\n' "$(date -u +%Y-%m-%dT%H:%M:%SZ)" >"$CACHE/queue.state"
sync_mirror
printf 'sync_done=%s\n' "$(date -u +%Y-%m-%dT%H:%M:%SZ)" >>"$CACHE/queue.state"
rm -f "$CACHE/completed.txt" "$EVIDENCE/results.jsonl"

# Controls first: they use the same proof bodies as the spike variants and
# therefore replace the non-identical al#274 trial baselines.
JOBS=(
  wav_reads_never_panics_unsigned
  read_wav_range_never_panics_signed
  mp4_writer_output_decodes
  spike_wav_unsigned_fmt
  spike_wav_unsigned_alloc
  spike_wav_unsigned_both
  spike_wav_range_fmt
  spike_wav_range_alloc
  spike_wav_range_both
  spike_mp4_writer_fmt
  spike_mp4_writer_alloc
  spike_mp4_writer_both
)
for lane in $(seq 0 $((CONCURRENCY - 1))); do : >"$CACHE/jobs.$lane"; done
i=0
for h in "${JOBS[@]}"; do
  lane=$((i % CONCURRENCY))
  printf '%s\n' "$h" >>"$CACHE/jobs.$lane"
  i=$((i + 1))
done

pids=()
for lane in $(seq 0 $((CONCURRENCY - 1))); do
  worker "$lane" &
  pids+=($!)
done
for pid in "${pids[@]}"; do wait "$pid"; done
cat "$CACHE"/*/*.result.json >"$EVIDENCE/results.jsonl" 2>/dev/null || true
printf 'queue_done=%s\n' "$(date -u +%Y-%m-%dT%H:%M:%SZ)" >>"$CACHE/queue.state"
cp "$CACHE/queue.state" "$EVIDENCE/queue.state" 2>/dev/null || true

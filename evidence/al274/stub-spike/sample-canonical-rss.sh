#!/usr/bin/env bash
# Auxiliary RSS sampler for the canonical representative subset run.
set -u
OUT="/mnt/g/orca/workspaces/audio-lens/al274-kani-stub-spike/evidence/al274/stub-spike/canonical/rss-samples.txt"
mkdir -p "$(dirname "$OUT")"
for _ in $(seq 1 360); do
  line=$(ps -eo comm=,rss=,args= | awk '
    ($1 == "cbmc" || $1 == "kani-driver") && index($0, "al-formal-kani/src") > 0 { sum += $2 }
    END { print sum + 0 }
  ')
  harness=$(ps -eo args= | sed -n 's/.*--harness \([^ ]*\).*/\1/p' | head -1)
  printf '%s harness=%s rss_kb=%s\n' "$(date -u +%H:%M:%S)" "${harness:-unknown}" "$line" >>"$OUT"
  sleep 10
done

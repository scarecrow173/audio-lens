#!/usr/bin/env python3
"""Summarize al#274 stub-spike ablation artifacts (auxiliary evidence)."""
import json
import re
from pathlib import Path

ROOT = Path(__file__).resolve().parent
ABL = ROOT / "ablation"
CATS = {
    "to_isize": "ToISize",
    "same_allocation": "same_allocation",
    "padadapter": "PadAdapter",
    "memchr": "function core::slice::memchr",
    "alloc_error": "function std::alloc::handle_alloc_error::rt_error",
    "rawvec": "RawVec",
    "fmt_write": "function core::fmt::write",
}

def last_nonempty(text: str) -> str:
    return next((line for line in reversed(text.splitlines()) if line.strip()), "")

rows = []
for path in sorted(ABL.glob("*.result.json")):
    row = json.loads(path.read_text())
    verbose = ABL / f"{row['harness']}.verbose.log"
    text = verbose.read_text(errors="replace") if verbose.exists() else ""
    row["event_counts"] = {name: text.count(needle) for name, needle in CATS.items()}
    product = [
        line.split(" function ", 1)[1].rsplit(" thread", 1)[0]
        for line in text.splitlines()
        if " function al_media::" in line or " function al_testkit::" in line
    ]
    row["symex"]["product_function_events"] = len(product)
    row["symex"]["last_product_function"] = product[-1][:240] if product else ""
    product_iters = [
        int(m.group(1))
        for line in text.splitlines()
        if "al_media" in line and (m := re.search(r" iteration (\d+)", line))
    ]
    row["symex"]["max_product_loop_iteration"] = max(product_iters, default=0)
    row["symex"]["tail_nonempty"] = last_nonempty(text)[:500]
    rows.append(row)

order = {
    "wav_reads_never_panics_unsigned": 0,
    "spike_wav_unsigned_fmt": 1,
    "spike_wav_unsigned_alloc": 2,
    "spike_wav_unsigned_both": 3,
    "read_wav_range_never_panics_signed": 10,
    "spike_wav_range_fmt": 11,
    "spike_wav_range_alloc": 12,
    "spike_wav_range_both": 13,
    "mp4_writer_output_decodes": 20,
    "spike_mp4_writer_fmt": 21,
    "spike_mp4_writer_alloc": 22,
    "spike_mp4_writer_both": 23,
}
rows.sort(key=lambda r: order.get(r["harness"], 99))
(ABL / "summary.json").write_text(json.dumps(rows, ensure_ascii=False, indent=2) + "\n")

lines = [
    "| proof | variant | wall-clock s | peak RSS MiB | verbose/event lines | max unwind | product/testkit events | product loop iter | verdict | memchr | alloc_rt_error |",
    "| --- | --- | ---: | ---: | ---: | ---: | ---: | ---: | --- | ---: | ---: |",
]
for r in rows:
    lines.append(
        "| {h} | {v} | {w} | {rss:.0f} | {lines}/{events} | {it} | {prod} | {prodit} | {verdict} | {memchr} | {alloc} |".format(
            h=r["harness"],
            v=r["variant_kind"],
            w=r["wall_clock_s"],
            rss=r["peak_rss_kb"] / 1024,
            lines=r["symex"]["verbose_lines"],
            events=r["symex"]["event_lines"],
            it=r["symex"]["max_unwind_iteration"] or "n/a",
            prod=r["symex"]["product_function_events"],
            prodit=r["symex"]["max_product_loop_iteration"] or 0,
            verdict=r["verdict"],
            memchr=r["event_counts"]["memchr"],
            alloc=r["event_counts"]["alloc_error"],
        )
    )
(ABL / "table.md").write_text("\n".join(lines) + "\n")
print("\n".join(lines))

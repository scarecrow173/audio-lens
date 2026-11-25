#!/usr/bin/env python3
"""Summarize canonical representative-subset artifacts (mise lane evidence)."""
import json
import re
from pathlib import Path

ROOT = Path(__file__).resolve().parent
CAN = ROOT / "canonical"
rows = []
for path in sorted(CAN.glob("kani-*.txt")):
    harness = path.name.removeprefix("kani-").removesuffix(".txt")
    text = path.read_text(errors="replace")
    lines = text.splitlines()
    product = [
        line.split(" function ", 1)[1].rsplit(" thread", 1)[0]
        for line in lines
        if " function al_media::" in line or " function al_testkit::" in line
    ]
    product_iters = [
        int(m.group(1))
        for line in lines
        if "al_media" in line and (m := re.search(r" iteration (\d+)", line))
    ]
    rows.append({
        "harness": harness,
        "timed_out": "timedOut=true" in text.splitlines()[0],
        "verification_successful": "VERIFICATION:- SUCCESSFUL" in text,
        "verification_failed": "VERIFICATION:- FAILED" in text,
        "bytes": path.stat().st_size,
        "lines": len(lines),
        "event_lines": sum(
            1
            for line in lines
            if line.startswith(("Unwinding", "Not unwinding", "aborting path", "entering"))
        ),
        "memchr_events": text.count("function core::slice::memchr"),
        "padadapter_events": text.count("PadAdapter"),
        "alloc_rt_error_events": text.count("function std::alloc::handle_alloc_error::rt_error"),
        "product_function_events": len(product),
        "max_product_loop_iteration": max(product_iters, default=0),
        "last_product_function": product[-1][:240] if product else "",
        "log": f"canonical/{path.name}",
    })
(CAN / "raw-summary.json").write_text(json.dumps(rows, ensure_ascii=False, indent=2) + "\n")
print(json.dumps(rows, ensure_ascii=False, indent=2))

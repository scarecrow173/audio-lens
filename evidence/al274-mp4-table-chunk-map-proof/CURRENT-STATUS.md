# Current implementation status (do not lose)

## Active process

- Canonical 14-proof Kani lane restarted in background after reducing len2 chunks0/1 to unwind(3). Windows pid file: `evidence/al274-mp4-table-chunk-map-proof/kani-canonical.pid`; output `kani-canonical-lane.txt`; raw dir `kani-canonical/`.
- First check it before editing `crates/al-media/src/mp4.rs` or `tools/run-formal-kani.mjs`.
- If any validation len2 harness exceeds ~4 GiB RSS, abort and split/reduce structural shape further. Do not treat as product failure.

## Final proof architecture now implemented

Production refactor in `crates/al-media/src/mp4.rs`:
- extracted `append_chunk_samples` from `build_chunk_map`, behavior-preserving.
- unit test `mp4::tests::build_chunk_map_accumulates_offsets_and_rejects_count_or_overflow` passed after extraction.
- `cargo check --locked --all-targets -p al-media` passed after final split.

Runner group `mp4-table-chunk-map` now has 14 harnesses:
1-6 validation:
- mp4_stsc_chunk_samples_len0_bounded_exact
- mp4_stsc_chunk_samples_len1_bounded_exact
- mp4_stsc_chunk_samples_len2_chunks0_bounded_exact
- mp4_stsc_chunk_samples_len2_chunks1_bounded_exact
- mp4_stsc_chunk_samples_len2_chunks2_bounded_exact
- mp4_stsc_chunk_samples_len2_chunks3_bounded_exact
7-14 materialization/composition:
- mp4_chunk_map_helper_spc1_len0_reject_bounded_exact
- mp4_chunk_map_helper_spc1_len1_bounded_exact
- mp4_chunk_map_helper_spc2_len1_reject_bounded_exact
- mp4_chunk_map_helper_spc2_len2_bounded_exact
- mp4_chunk_map_end_to_end_empty_bounded_exact
- mp4_chunk_map_end_to_end_single_sample_bounded_exact
- mp4_chunk_map_end_to_end_short_sizes_reject_bounded_exact
- mp4_chunk_map_end_to_end_overlong_sizes_reject_bounded_exact

Registry total should be 56. Docs/mise/skill were bulk-updated from 13/55 to 14/56, but MUST be audited for stale 9/11/13/53/55 text.

## Measured proof evidence

- helper spc2/len2 alone PASS: 61.147 s, max RSS 372,560 KiB, evidence `kani-helper-spc2len2-trial-lane.txt` plus raw dir.
- end-to-end single sample spike PASS before rename: 23.966 s, max RSS 429,724 KiB, evidence `kani-mat-single-spike-lane.txt` plus raw dir. The canonical renamed proof is `mp4_chunk_map_end_to_end_single_sample_bounded_exact`; canonical lane must still supply final evidence.
- validation len1 PASS earlier: 23.710 s, 648,188 KiB.
- validation len2 chunks3 PASS earlier at unwind4: 30.104 s, 2,587,916 KiB.
- canonical trial 1 aborted: len2 chunks0+1 combined hit 4,891,052 KiB at 28s. Split into chunks0/chunks1. See `aborted-canonical-1.txt` and `kani-canonical-aborted-1-*`.
- many direct multi-chunk materialization trials exploded; see `aborted-mat-*.txt` and actions.jsonl. Record them honestly as resource aborts, not product failures.

## Boundary decision

Original direct constructor-valid multi-chunk materialization domain was infeasible. Final boundary is compositional:
- validation keeps original bounded domain (chunks 0..3, entries 0..2, first_chunk 0..5, spc 0..4, sd 0..2)
- helper proofs exact for spc1/2, sizes len0/1/2, full u32 sizes/full u64 offsets
- end-to-end proofs cover empty/single/count mismatch composition
- multi-chunk end-to-end materialization, spc3/4, sizes len3/4, 3+ samples remain NotRun residual.
This is documented in appended section 9 of `boundary-analysis.md`, but earlier sections 2/3.3/5/6 still contain superseded statements and need reconciliation.

## Immediate remaining work

1. Watch canonical lane. Verify every harness `VERIFICATION:- SUCCESSFUL`, unwinding assertions enabled, elapsed/RSS.
2. If pass, preserve lane output and create verification summary. If fail due proof assertion, debug; if resource blowup, split further.
3. Run/refresh `mise run fmt`, `mise run check al-media`, `mise run lint al-media`, `cargo test --locked -p al-media`, `mise run test-formal`.
4. Run neighboring Kani regression groups: at least `mp4-scan`, `mp4-avi-boundary`, `mp4-pcm-declaration` because same `mp4.rs`/registry. Record outputs.
5. Mutation: change `append_chunk_samples` checked_add overflow rejection to wrapping/addition (semantic mutation), run group lane; it must fail in materialization proof. Save original file byte-for-byte first, restore exact copy, verify sha256/cmp, rerun post-revert lane.
6. Finish evidence: README.md, verification.md, ledger.json, cost-comparison.md; include all aborted trials/operator error, canonical logs, regression, mutation/revert hashes. Normalize timestamp format in actions.jsonl.
7. Fully sync docs/specs/formal-verification.md, docs/specs/formal-verification-boundary.md, docs/designs/al274-formal-verification-boundary.md, `.agents/skills/verify-audio-lens/SKILL.md`, `mise.toml` to final 14-proof/56-registry compositional boundary. Never edit generated docs index.
8. Merge latest origin/main before final verification; never push to main. Then final checks.
9. Commit/push branch `feature/274-mp4-table-chunk-map-proof`, create Japanese PR with `Closes #274` if appropriate and verification section.
10. Inspect branch CI read-only, then send `worker_done` exactly once with PR URL to coordinator session `orca_session_id:ebcbd033-b058-446a-a85a-4934b22d75d1`, task `task_e4eec6dc8dcc`, dispatch `ctx_b610078ba286`.

## Cautions

- Do not use `kani::assume` or stubs.
- Do not weaken assertions to make lane pass.
- Manual aborts must be recorded as resource/cost trials, not product counterexamples.
- No commit/push/PR/mutation/regression lanes have been completed yet.

## Latest update

Canonical trial 2 was aborted because len2 chunks0 hit 2,015,880 KiB in 10s at unwind4. chunks0/1 are now unwind3. Current canonical output is again `kani-canonical-lane.txt` / `kani-canonical/`. See `aborted-canonical-2.txt`. If chunks0/1 report unwinding assertion failure, restore unwind4 only if memory fits; otherwise further decompose valid/invalid cases without weakening domain.

## Canonical trial 3

len2 chunks2 hit 4,373,808 KiB at 95s with unwind4; aborted. chunks2 is now unwind3. chunks0/1 remain unwind3, chunks3 remains unwind4. Canonical restarted again. Partial logs archived under `kani-canonical-aborted-3*`.

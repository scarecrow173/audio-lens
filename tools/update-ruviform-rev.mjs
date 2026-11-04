// Explicit follow-up operation for the Cargo.lock source-of-truth
// contract (docs/designs/ruviform-lockfile-source-of-truth.md §5):
// move every ruviform crate to a candidate Ruviform commit SHA by
// updating ONLY Cargo.lock.
//
//   node tools/update-ruviform-rev.mjs <40-hex-sha>
//
// `cargo update -p <ruviform-pkg> --precise <sha>` applies the precise
// rev to the whole shared git source atomically — all ruviform crates
// move together, and if the target SHA lacks any required crate cargo
// fails without touching Cargo.lock. After the update the lockfile is
// re-verified through tools/ruviform-lock-rev.mjs (single SHA, fail-
// closed). Rollback is `git checkout -- Cargo.lock`; the vendored UI
// tree follows with `mise run sync-ui-deps`.
//
// Intended use: update PRs and CI candidate-injection (Ruviform #219 /
// #294 consumer-verify). Never used by ordinary builds — `--locked`
// keeps the recorded SHA.

import { execFileSync } from "node:child_process";
import { resolve } from "node:path";
import { resolveRuviformRev } from "./ruviform-lock-rev.mjs";

const REPO_ROOT = resolve(import.meta.dirname, "..");
const SHA40 = /^[0-9a-f]{40}$/;

const sha = process.argv[2];
if (sha === undefined || !SHA40.test(sha)) {
  console.error("usage: node tools/update-ruviform-rev.mjs <40-hex ruviform commit sha>");
  process.exit(1);
}

const before = resolveRuviformRev(REPO_ROOT);
if (before.rev === sha) {
  console.log(
    `ruviform already resolved to ${sha.slice(0, 10)} (${before.packages.length} packages)`,
  );
  process.exit(0);
}

// The precise rev is a property of the shared git source; any ruviform
// package selects it. Sorted list order is stable across runs.
const target = before.packages[0];
console.log(
  `updating ruviform source ${before.rev.slice(0, 10)} -> ${sha.slice(0, 10)} (${target})`,
);
try {
  execFileSync("cargo", ["update", "-p", target, "--precise", sha], {
    cwd: REPO_ROOT,
    stdio: "inherit",
  });
} catch (e) {
  console.error(
    `cargo update failed (Cargo.lock left unchanged): ${e.message ?? "see cargo output"}`,
  );
  process.exit(1);
}

const after = resolveRuviformRev(REPO_ROOT);
if (after.rev !== sha) {
  console.error(`post-update verification failed: resolved ${after.rev}, expected ${sha}`);
  process.exit(1);
}
console.log(`ruviform resolved: ${before.rev} -> ${after.rev} (${after.packages.length} packages)`);
console.log(
  "next: `mise run sync-ui-deps` for ui/vendor, then `mise run verify-ci` + record diffs",
);

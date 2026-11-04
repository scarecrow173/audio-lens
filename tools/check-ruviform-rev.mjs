// Verify that every ruviform git dependency resolves to a single
// commit SHA in Cargo.lock, that Cargo.toml `rev` pins (when present)
// agree with it, and that the vendored ui tree (ui/vendor/ruviform/REV)
// matches. Works in both dependency modes:
//   - manifest-pinned: ruviform-* workspace deps carry rev = "<sha>"
//     (current; docs/designs/ruviform-dependency-ci.md §2)
//   - lockfile-primary (#140): manifest carries no rev; the resolved
//     SHA lives only in Cargo.lock `git+...#<sha>` source fragments
// Fail-closed: zero ruviform lockfile sources, divergent SHAs, a
// manifest rev that disagrees with the lock, or a mixed some-have-rev
// manifest state are all errors. `--require-vendor` also makes a
// missing REV file an error — the ui-verify CI lane uses it after
// `mise run sync-ui-deps` materializes the vendor tree.
//
// Usage:
//   node tools/check-ruviform-rev.mjs                   lockfile + manifest (+ REV if present)
//   node tools/check-ruviform-rev.mjs --require-vendor  + REV must exist and match
//
// On success the resolved SHA is printed and, when GITHUB_OUTPUT is
// set (GitHub Actions), exported as `resolved_sha` so evidence rows
// can record it. Portable across Windows/macOS/Linux.

import { appendFileSync, existsSync, readFileSync } from "node:fs";
import { join, resolve } from "node:path";

const REPO_ROOT = resolve(import.meta.dirname, "..");
const CARGO_TOML = join(REPO_ROOT, "Cargo.toml");
const CARGO_LOCK = join(REPO_ROOT, "Cargo.lock");
const REV_FILE = join(REPO_ROOT, "ui", "vendor", "ruviform", "REV");
const RUVIFORM_GIT = /scarecrow173\/ruviform\.git/;
const SHA = /[0-9a-f]{40}/;

const failures = [];
const fail = (msg) => failures.push(msg);

/** Cargo.toml: ruviform git dep lines, manifest rev consistency. */
function manifestPin() {
  const lines = readFileSync(CARGO_TOML, "utf8").split("\n");
  const depLines = lines.filter((l) => RUVIFORM_GIT.test(l));
  if (depLines.length === 0) {
    fail("no ruviform git dependencies found in Cargo.toml");
    return { rev: null };
  }
  const revs = new Set();
  let pinned = 0;
  for (const line of depLines) {
    const rev = line.match(/rev = "([0-9a-f]{40})"/)?.[1];
    if (rev !== undefined) {
      revs.add(rev);
      pinned += 1;
    }
  }
  if (revs.size > 1) {
    fail(`manifest revs diverged: ${[...revs].join(", ")}`);
  }
  if (pinned > 0 && pinned < depLines.length) {
    // Mixed manifest state (some deps pinned, some following the
    // default branch) has no single source of truth — neither the
    // manifest-pinned nor the lockfile-primary contract holds.
    fail(
      `mixed manifest state: ${pinned} of ${depLines.length} ruviform deps ` +
        "carry a rev pin — pin all deps or none (lockfile-primary mode)",
    );
  }
  return { rev: revs.size === 1 ? [...revs][0] : null };
}

/** Cargo.lock: resolved SHA of every ruviform git source. */
function lockfileSha() {
  const lines = readFileSync(CARGO_LOCK, "utf8").split("\n");
  const shas = new Set();
  let count = 0;
  for (const line of lines) {
    if (!line.trimStart().startsWith("source = ") || !RUVIFORM_GIT.test(line)) {
      continue;
    }
    count += 1;
    const sha = line.match(new RegExp(`#(${SHA.source})"?\\s*$`))?.[1];
    if (sha === undefined) {
      fail(`ruviform lockfile source without #<sha> fragment: ${line.trim()}`);
      continue;
    }
    shas.add(sha);
  }
  if (count === 0) {
    fail("no ruviform git sources in Cargo.lock");
    return { count, sha: null };
  }
  if (shas.size !== 1) {
    fail(`ruviform sources resolved to divergent SHAs: ${[...shas].join(", ")}`);
    return { count, sha: null };
  }
  return { count, sha: [...shas][0] };
}

const requireVendor = process.argv.includes("--require-vendor");
const manifest = manifestPin();
const lock = lockfileSha();

if (manifest.rev !== null && lock.sha !== null && manifest.rev !== lock.sha) {
  fail(`manifest rev ${manifest.rev} != lockfile resolved ${lock.sha}`);
}

const mode = manifest.rev === null ? "lockfile-primary" : "manifest-pinned";
const revExists = existsSync(REV_FILE);
if (revExists && lock.sha !== null) {
  const vendored = readFileSync(REV_FILE, "utf8").trim();
  if (vendored !== lock.sha) {
    fail(`ui vendor REV ${vendored} != lockfile resolved ${lock.sha}`);
  }
} else if (!revExists && requireVendor) {
  fail("ui/vendor/ruviform/REV is missing — run `mise run sync-ui-deps` first");
}

if (failures.length > 0) {
  for (const f of failures) {
    process.stderr.write(`check-ruviform-rev: ${f}\n`);
  }
  process.exit(1);
}

const vendorNote = revExists ? "REV match" : "no vendor tree";
process.stdout.write(
  `check-ruviform-rev: resolved ${lock.sha} (${lock.count} lockfile sources, ` +
    `${mode}, ${vendorNote})\n`,
);
if (process.env.GITHUB_OUTPUT) {
  appendFileSync(process.env.GITHUB_OUTPUT, `resolved_sha=${lock.sha}\n`);
}

// Sync the ruviform `ui/*` TypeScript packages (ui/core, ui/components,
// ui/binding, ui/viz — Issues #258/#259/#260/#261) into `ui/vendor/ruviform/`
// at the same commit SHA the Cargo workspace resolved.
//
// Ruviform is consumed via git dependencies
// (docs/designs/ruviform-lockfile-source-of-truth.md): the `ui/*` packages
// are `private` npm packages that ship as TS source, so there is no
// registry/git-subdir npm dependency mechanism. Materializing them here
// mirrors cargo's git checkout: the resolved SHA in `Cargo.lock` is the
// single source of truth (tools/ruviform-lock-rev.mjs), the vendored tree
// is a gitignored generated artifact restored by `mise run sync-ui-deps`/
// the `*-ui` tasks, and `.worktreeinclude` propagates it to new worktrees.
//
// Usage:
//   node tools/sync-ruviform-ui.mjs           ensure the vendor tree matches the lockfile SHA
//   node tools/sync-ruviform-ui.mjs --check   fail unless it already matches
//
// Fetch uses system git (same credential path as `cargo` with
// `net.git-fetch-with-cli = true`): `git fetch --depth 1 origin <sha>` +
// `git archive`. GitHub serves reachable-SHA fetches; no extra credential
// beyond the existing ruviform read access is needed.

import { execFileSync } from "node:child_process";
import { existsSync, mkdirSync, mkdtempSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join, resolve } from "node:path";
import { resolveRuviformRev } from "./ruviform-lock-rev.mjs";

const REPO_ROOT = resolve(import.meta.dirname, "..");
const VENDOR_DIR = join(REPO_ROOT, "ui", "vendor", "ruviform");
const REV_FILE = join(VENDOR_DIR, "REV");
const PACKAGES = ["ui/core", "ui/components", "ui/binding", "ui/viz"];

/**
 * Read the shared ruviform git revision + canonical url from the
 * Cargo.lock contract (single SHA, fail-closed on divergence).
 */
function readPin() {
  const { rev, url } = resolveRuviformRev(REPO_ROOT);
  return { rev, url };
}

function syncedRev() {
  return existsSync(REV_FILE) ? readFileSync(REV_FILE, "utf8").trim() : undefined;
}

function vendorComplete() {
  return PACKAGES.every((p) => existsSync(join(VENDOR_DIR, p, "src", "index.ts")));
}

function run(cmd, args, opts = {}) {
  // stdin is piped (not ignored) so `input:` callers can stream data in.
  return execFileSync(cmd, args, { stdio: ["pipe", "pipe", "inherit"], ...opts });
}

/** MSYS2-safe path for external tools: backslashes/forward slashes both work, `C:` in -f position does not. */
function toolPath(p) {
  return p.replaceAll("\\", "/");
}

function sync(pin) {
  const tmp = mkdtempSync(join(tmpdir(), "rvf-ui-sync-"));
  try {
    run("git", ["init", "-q", tmp]);
    run("git", ["-C", tmp, "remote", "add", "origin", pin.url]);
    run("git", ["-C", tmp, "fetch", "-q", "--depth", "1", "origin", pin.rev]);
    const archive = run("git", ["-C", tmp, "archive", "FETCH_HEAD", ...PACKAGES]);
    rmSync(VENDOR_DIR, { recursive: true, force: true });
    mkdirSync(VENDOR_DIR, { recursive: true });
    // tar reads the archive on stdin: `-f C:\...` would be parsed as a
    // remote host by GNU tar. Works with GNU tar and bsdtar alike.
    run("tar", ["-x", "-C", toolPath(VENDOR_DIR)], { input: archive });
    writeFileSync(REV_FILE, `${pin.rev}\n`);
    if (!vendorComplete()) {
      throw new Error(`vendor extraction incomplete in ${VENDOR_DIR}`);
    }
    console.log(`synced ruviform ui/* @ ${pin.rev.slice(0, 10)} -> ui/vendor/ruviform`);
  } finally {
    rmSync(tmp, { recursive: true, force: true });
  }
}

const checkOnly = process.argv.includes("--check");
const pin = readPin();
const current = syncedRev();
if (current === pin.rev && vendorComplete()) {
  if (!checkOnly) console.log(`ruviform ui/* already synced @ ${pin.rev.slice(0, 10)}`);
  process.exit(0);
}
if (checkOnly) {
  console.error(
    `ruviform ui/* vendor out of sync: vendored=${current ?? "<none>"} lockfile=${pin.rev}`,
  );
  console.error("run `node tools/sync-ruviform-ui.mjs` (or `mise run sync-ui-deps`)");
  process.exit(1);
}
sync(pin);

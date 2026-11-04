// Single authoritative contract for reading the Ruviform resolved git
// revision: the workspace `Cargo.lock` is the source of truth
// (docs/designs/ruviform-lockfile-source-of-truth.md §2). The manifest
// declares only `git`/`features`; the commit SHA lives in each git
// `[[package]].source` fragment `git+<url>[?<query>]#<sha>`.
//
// This module is both the CLI (`node tools/ruviform-lock-rev.mjs`,
// `mise run check-ruviform-lock`) and the import surface shared by
// `sync-ruviform-ui.mjs` and `update-ruviform-rev.mjs`. Detection is
// structural — `[[package]]` blocks and `key = "…"` scalars — never
// arbitrary string grep. Every failure mode is fail-closed (non-zero
// exit / thrown RuviformLockError): zero ruviform packages, divergent
// resolved SHAs, unknown source format, unreadable manifests.
//
// Usage:
//   node tools/ruviform-lock-rev.mjs          verify + print the resolved SHA
//   node tools/ruviform-lock-rev.mjs --json   machine-readable {rev,url,packages}

import { readFileSync } from "node:fs";
import { join, resolve } from "node:path";
import { pathToFileURL } from "node:url";

export class RuviformLockError extends Error {}

const SHA40 = /^[0-9a-f]{40}$/;

/**
 * Normalize a git URL (manifest `git = "…"` or lockfile git source) to a
 * comparable `host/path` key. Strips `git+`, scheme, scp-style `user@`,
 * `.git` suffix and trailing slashes; lowercases the result.
 */
export function normalizeGitUrl(raw) {
  let s = raw.trim();
  if (s.startsWith("git+")) s = s.slice("git+".length);
  // Drop any ?<query> / #<fragment> the caller left on the string.
  s = s.split("#")[0].split("?")[0];
  if (s.includes("://")) {
    // scheme://[user@]host/path — keep host+path only.
    s = s.replace(/^[A-Za-z][A-Za-z0-9+.-]*:\/\//, "").replace(/^[^/@]+@/, "");
  } else {
    // scp-like user@host:path
    const scp = s.match(/^([A-Za-z0-9_.-]+)@([A-Za-z0-9._-]+):(.+)$/);
    if (scp) s = `${scp[2]}/${scp[3]}`;
  }
  s = s.replace(/\/+$/, "");
  if (s.toLowerCase().endsWith(".git")) s = s.slice(0, -".git".length);
  return s.toLowerCase();
}

/** Parse a lockfile git source: `git+<url>[?<query>]#<sha>`. */
export function parseGitSource(source) {
  if (!source.startsWith("git+")) return undefined;
  const hash = source.indexOf("#");
  if (hash === -1) {
    throw new RuviformLockError(`git source without resolved fragment: ${source}`);
  }
  const fragment = source.slice(hash + 1);
  if (!SHA40.test(fragment)) {
    throw new RuviformLockError(`git source with non-40-hex resolved fragment: ${source}`);
  }
  const base = source.slice("git+".length, hash);
  const q = base.indexOf("?");
  const url = q === -1 ? base : base.slice(0, q);
  const query = q === -1 ? "" : base.slice(q + 1);
  if (!url) {
    throw new RuviformLockError(`git source with empty url: ${source}`);
  }
  return { url, query, sha: fragment };
}

/**
 * Parse Cargo.lock into package records. Understands the generated
 * lockfile grammar: `#` comments, top-level scalars, `[[package]]`
 * sections with `key = "…"` scalars and `key = [ … ]` arrays.
 * Unknown shapes (missing name, non-scalar name/source) fail closed.
 */
export function parseCargoLock(toml) {
  const packages = [];
  let current;
  let inArray = false;
  for (const rawLine of toml.split("\n")) {
    const line = rawLine.trim();
    if (!line || line.startsWith("#")) continue;
    if (inArray) {
      if (line.endsWith("]")) inArray = false;
      continue;
    }
    if (line === "[[package]]") {
      current = { name: undefined, source: undefined };
      packages.push(current);
      continue;
    }
    if (line.startsWith("[")) {
      current = undefined;
      continue;
    }
    if (current === undefined) continue; // top-level scalars (version = N)
    const kv = line.match(/^([A-Za-z0-9_-]+)\s*=\s*(.*)$/);
    if (!kv) {
      throw new RuviformLockError(`unparseable lockfile line: ${line}`);
    }
    const [, key, value] = kv;
    if (value.startsWith("[") && !value.endsWith("]")) {
      inArray = true;
      continue;
    }
    if (key === "name" || key === "source") {
      const scalar = value.match(/^"((?:[^"\\]|\\.)*)"$/);
      if (!scalar) {
        throw new RuviformLockError(`non-scalar ${key} in [[package]] section: ${line}`);
      }
      current[key] = scalar[1];
    }
  }
  for (const pkg of packages) {
    if (pkg.name === undefined) {
      throw new RuviformLockError("[[package]] section without name");
    }
  }
  return packages;
}

/**
 * Collect the ruviform git URLs declared in `[workspace.dependencies]`.
 * Returns a Map<normalizedUrl, rawUrl>. Every `ruviform*` entry (key or
 * `package = "ruviform…"` rename) must carry `git = "<url>"`; a
 * ruviform entry without `git` fails closed. Dependency values may
 * span multiple lines (unbalanced `{` pulls continuation lines).
 */
export function manifestRuviformUrls(toml) {
  const urls = new Map();
  let inWorkspaceDeps = false;
  const lines = toml.split("\n");
  for (let i = 0; i < lines.length; i++) {
    const line = lines[i].trim();
    if (line.startsWith("[")) {
      inWorkspaceDeps = line === "[workspace.dependencies]";
      continue;
    }
    if (!inWorkspaceDeps || !line || line.startsWith("#")) continue;
    const dep = line.match(/^([A-Za-z0-9_.-]+)\s*=\s*(.*)$/);
    if (!dep) {
      throw new RuviformLockError(`unparseable [workspace.dependencies] line: ${line}`);
    }
    const key = dep[1];
    let body = dep[2];
    // Continue accumulating while the inline-table braces are open.
    while (
      (body.match(/\{/g) ?? []).length > (body.match(/\}/g) ?? []).length &&
      i + 1 < lines.length
    ) {
      body += ` ${lines[++i].trim()}`;
    }
    const renamed = body.match(/package\s*=\s*"([^"]+)"/);
    const packageName = renamed ? renamed[1] : key;
    if (!packageName.startsWith("ruviform")) continue;
    const git = body.match(/git\s*=\s*"([^"]+)"/);
    if (!git) {
      throw new RuviformLockError(`ruviform dep without git url in manifest: ${line}`);
    }
    urls.set(normalizeGitUrl(git[1]), git[1]);
  }
  return urls;
}

/**
 * Resolve + verify the single ruviform revision from `<root>/Cargo.lock`.
 * Returns { rev, url, urls, packages[] } — `url` is the canonical git
 * URL declared in the manifest (usable for `git fetch`), `urls` every
 * raw lockfile git URL seen, `packages` the ruviform package names.
 * Throws RuviformLockError on every contract violation.
 */
export function resolveRuviformRev(root) {
  const lockPath = join(root, "Cargo.lock");
  const manifestPath = join(root, "Cargo.toml");
  let lockToml;
  let manifestToml;
  try {
    lockToml = readFileSync(lockPath, "utf8");
    manifestToml = readFileSync(manifestPath, "utf8");
  } catch (e) {
    throw new RuviformLockError(`cannot read manifest/lockfile: ${e.message}`);
  }

  const manifestUrls = manifestRuviformUrls(manifestToml);
  const urlSet = new Set(manifestUrls.keys());
  if (urlSet.size === 0) {
    throw new RuviformLockError("no ruviform git urls found in [workspace.dependencies]");
  }
  const packages = parseCargoLock(lockToml);

  // Every package named ruviform* must resolve from a declared
  // ruviform git URL — a registry/path source or a repointed remote
  // is an anomaly, not a new URL to follow (transitive crates keep
  // matching via the URL set below regardless of their name).
  const matched = [];
  for (const pkg of packages) {
    const isRuviformName = pkg.name.startsWith("ruviform");
    const parsed = pkg.source === undefined ? undefined : parseGitSource(pkg.source);
    const fromRepo = parsed !== undefined && urlSet.has(normalizeGitUrl(parsed.url));
    if (isRuviformName && !fromRepo) {
      throw new RuviformLockError(
        `ruviform package resolved from unexpected source: ${pkg.name} -> ${pkg.source ?? "<no source>"}`,
      );
    }
    if (fromRepo) {
      matched.push({ name: pkg.name, sha: parsed.sha, url: parsed.url });
    }
  }
  if (matched.length === 0) {
    throw new RuviformLockError("no packages resolved from ruviform git urls in Cargo.lock");
  }
  const shas = new Map();
  for (const m of matched) {
    const list = shas.get(m.sha) ?? [];
    list.push(m.name);
    shas.set(m.sha, list);
  }
  if (shas.size !== 1) {
    const detail = [...shas].map(([sha, names]) => `${sha}: ${names.join(",")}`).join("; ");
    throw new RuviformLockError(`ruviform resolved revisions diverged: ${detail}`);
  }
  const rev = [...shas.keys()][0];
  const rawUrls = [...new Set(matched.map((m) => m.url))].sort();
  const canonical = [...manifestUrls.values()].find((u) =>
    rawUrls.map(normalizeGitUrl).includes(normalizeGitUrl(u)),
  );
  return {
    rev,
    url: canonical ?? rawUrls[0],
    urls: rawUrls,
    packages: matched.map((m) => m.name).sort(),
  };
}

function main(argv) {
  const json = argv.includes("--json");
  const root = resolve(import.meta.dirname, "..");
  const resolved = resolveRuviformRev(root);
  if (json) {
    console.log(JSON.stringify(resolved));
  } else {
    console.log(
      `ruviform lockfile rev: ${resolved.rev} (${resolved.packages.length} packages, ${resolved.url})`,
    );
  }
}

if (
  process.argv[1] !== undefined &&
  import.meta.url === pathToFileURL(resolve(process.argv[1])).href
) {
  try {
    main(process.argv.slice(2));
  } catch (e) {
    if (e instanceof RuviformLockError) {
      console.error(`ruviform-lock-rev: ${e.message}`);
      process.exit(1);
    }
    throw e;
  }
}

// Verify that rustc/cargo resolve to the mise-pinned Rust toolchain and
// that the rustfmt/clippy components are installed. Invoked via
// `mise run verify-toolchain` (see docs/designs/ruviform-dependency-ci.md
// §5). Portable across Windows/macOS/Linux (no POSIX text tools).
import { execFileSync } from "node:child_process";

function run(cmd, args) {
  return execFileSync(cmd, args, { encoding: "utf8" }).trim();
}

function versionOf(output) {
  const m = output.match(/^\S+ (\d+\.\d+\.\d+)/);
  if (!m) {
    throw new Error(`cannot parse version from: ${output}`);
  }
  return m[1];
}

const pin = run("mise", ["current", "rust"]);
if (!/^\d+\.\d+\.\d+$/.test(pin)) {
  process.stderr.write(`verify-toolchain: unexpected 'mise current rust' output: ${pin}\n`);
  process.exit(1);
}

const failures = [];
const rustc = versionOf(run("rustc", ["--version"]));
const cargo = versionOf(run("cargo", ["--version"]));
if (rustc !== pin) {
  failures.push(`rustc ${rustc} != pinned ${pin}`);
}
if (cargo !== pin) {
  failures.push(`cargo ${cargo} != pinned ${pin}`);
}

for (const [cmd, args] of [
  ["rustfmt", ["--version"]],
  ["cargo", ["clippy", "--version"]],
]) {
  try {
    run(cmd, args);
  } catch {
    failures.push(`${cmd} ${args.join(" ")} unavailable (missing rustfmt/clippy component?)`);
  }
}

if (failures.length > 0) {
  for (const f of failures) {
    process.stderr.write(`verify-toolchain: ${f}\n`);
  }
  process.exit(1);
}
process.stdout.write(`verify-toolchain: rustc/cargo ${pin}, rustfmt+clippy present\n`);

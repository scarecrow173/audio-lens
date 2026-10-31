// Guard: headless crates must not depend on GUI/WebView/GPU/windowing
// framework crates (docs/designs/ruviform-dependency-ci.md §7). Invoked
// via `mise run check-cli-headless`. Portable across
// Windows/macOS/Linux (no POSIX text tools).
import { execFileSync } from "node:child_process";

// Crates that must stay free of GUI framework dependencies. `al-viz`
// was removed by AL#106 (portable export moved to al-dataset; the
// static report and display products were deleted). `al-math` was
// removed in AL#101 (its kernels moved to the ruviform-numeric
// dependency, checked transitively via the remaining crates' dep
// trees).
// `al-service` is the shared headless service layer consumed by both
// `al-cli` and `al-gui` (AL#105); it must stay GUI-free. `al-gui` is
// the GUI crate itself and is intentionally absent — presentation
// crates own the GUI deps.
const HEADLESS_CRATES = [
  "al-cli",
  "al-core",
  "al-job",
  "al-dataset",
  "al-media",
  "al-locate",
  "al-analysis",
  "al-service",
  "al-testkit",
  "al-observability",
];

// Curated exact package names of GUI/WebView/GPU/windowing framework
// crates. OS API crates (`windows-sys`, `windows-targets`, `winapi`,
// `libc`, …) are deliberately NOT listed — they are not GUI frameworks.
const EXACT = new Set([
  "azul",
  "druid",
  "eframe",
  "egui",
  "fltk",
  "gdk",
  "gdk4",
  "gtk",
  "gtk4",
  "iced",
  "naga",
  "slint",
  "tao",
  "tauri",
  "tauri-runtime",
  "webkit2gtk",
  "webview2-com",
  "wgpu",
  "winit",
  "wry",
]);

// Prefix families for scoped crate ecosystems.
const PREFIXES = [
  "egui-",
  "gdk-",
  "gtk-",
  "iced-",
  "iced_",
  "qt-",
  "qt6-",
  "tauri-",
  "webview2-",
  "wgpu-",
  "winit-",
];

function isGuiFramework(name) {
  if (EXACT.has(name)) {
    return true;
  }
  return PREFIXES.some((p) => name.startsWith(p));
}

function tree(crate) {
  return execFileSync(
    "cargo",
    ["tree", "-p", crate, "-e", "normal,build,dev", "--prefix", "none"],
    { encoding: "utf8" },
  );
}

const violations = [];
for (const crate of HEADLESS_CRATES) {
  for (const line of tree(crate).split(/\r?\n/)) {
    const m = line.match(/^([A-Za-z0-9_-]+)\s+v[\d.]+/);
    if (m && isGuiFramework(m[1])) {
      violations.push(`${crate}: ${line.trim()}`);
    }
  }
}

if (violations.length > 0) {
  process.stderr.write(
    "check-cli-headless: GUI framework dependencies found in headless crates:\n",
  );
  for (const v of violations) {
    process.stderr.write(`  ${v}\n`);
  }
  process.exit(1);
}
process.stdout.write(
  `check-cli-headless: no GUI framework deps in ${HEADLESS_CRATES.length} headless crates\n`,
);

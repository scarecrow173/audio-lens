// Issue #256 (D 層 formal verification 第 1 標的): al-media WAV codec
// の #[cfg(kani)] bounded proof harness(tests/formal_wav.rs)を
// `cargo kani` で実行する lane runner。`mise run test-formal-kani`
// 経由で呼ぶ。
//
// Kani は Windows native では動かないため、実行環境を検出する:
//   1. native の `cargo kani`(Linux/macOS で provision 済みの場合)
//   2. Windows では WSL2 distro 内の `cargo kani`
// 環境・kani-verifier が provision されていない場合は verdict
// `not_run`(環境不在 — POLICIES §3、lane 失敗ではない)または
// `unsupported`(この platform では実行できない)を理由付きで出力して
// exit 0。実走して verification 結果が出れば `pass` -> exit 0、
// `fail` -> exit 1。
//
// Provision(honest not_run の案内と同じ手順):
//   wsl -d Ubuntu -- bash -lc "cargo install --locked kani-verifier && cargo kani setup"
//
// WSL 経路では /mnt 経由のビルド I/O を避けるため、cargo が必要とする
// source tree(Cargo.toml/Cargo.lock/.cargo/crates)を WSL native FS の
// mirror へ同期してから実行する。cargo の target-dir 設定・env 変数は
// 一切変更せず、build 出力は mirror 内の既定 ./target に収まる —
// Windows 側 repo の target/ には異 OS の artifact を書き込まない。
// mirror 更新では target/ だけを保持して source を再展開するため、
// dep build cache は再実行間で持続する。
import { spawnSync } from "node:child_process";
import path from "node:path";
import { fileURLToPath } from "node:url";

const REPO = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");
const PACKAGE = "al-media";
const TIMEOUT_MS = Number(process.env.AL_KANI_TIMEOUT_MS ?? 45 * 60 * 1000);
const WSL_MIRROR = "$HOME/.cache/al-formal-kani/src";

function verdict(v, reason, extra = {}) {
  const rec = { lane: "test-formal-kani", verdict: v, reason, ...extra };
  process.stdout.write(`${JSON.stringify(rec)}\n`);
  process.exit(v === "fail" ? 1 : 0);
}

function probe(args, opts = {}) {
  const r = spawnSync(args[0], args.slice(1), {
    encoding: "utf8",
    timeout: 60_000,
    ...opts,
  });
  return { status: r.status, out: `${r.stdout ?? ""}${r.stderr ?? ""}` };
}

function nativeKani() {
  const r = probe(["cargo", "kani", "--version"]);
  return r.status === 0;
}

function wslDistros() {
  const r = spawnSync("wsl.exe", ["-l", "-q"], {
    encoding: "buffer",
    timeout: 30_000,
  });
  if (r.status !== 0 || !r.stdout) return [];
  // wsl.exe -l -q は UTF-16LE で出力する。
  return r.stdout
    .toString("utf16le")
    .split(/\r?\n/)
    .map((s) => s.replace(/[^\x20-\x7e]/g, "").trim())
    .filter((s) => s.length > 0 && s !== "docker-desktop");
}

function wslProbe(distro, script) {
  return probe(["wsl.exe", "-d", distro, "--", "bash", "-lc", script]);
}

function toWslPath(winPath) {
  const m = winPath.match(/^([A-Za-z]):[\\/](.*)$/);
  if (!m) return null;
  return `/mnt/${m[1].toLowerCase()}/${m[2].replace(/\\/g, "/")}`;
}

// --- env detect ------------------------------------------------------------

let distro = null;
if (nativeKani()) {
  distro = "native";
} else if (process.platform === "win32") {
  const distros = wslDistros();
  if (distros.length === 0) {
    verdict(
      "unsupported",
      "win32 host with no usable WSL2 distro (wsl.exe -l -q empty); " +
        "kani needs Linux/macOS — install WSL2 Ubuntu then provision " +
        "kani-verifier inside it",
    );
  }
  for (const d of distros) {
    const cargo = wslProbe(d, "command -v cargo");
    if (cargo.status !== 0) continue;
    const kani = wslProbe(d, "cargo kani --version");
    if (kani.status === 0) {
      distro = d;
      break;
    }
  }
  if (distro === null) {
    const seen = distros.join(", ");
    verdict(
      "not_run",
      `no WSL distro with working 'cargo kani' (distros seen: ${seen}); ` +
        "provision: wsl -d Ubuntu -- bash -lc " +
        "'cargo install --locked kani-verifier && cargo kani setup', " +
        "then re-run `mise run test-formal-kani`",
    );
  }
} else {
  verdict(
    "not_run",
    "'cargo kani' not installed on this host; provision: " +
      "cargo install --locked kani-verifier && cargo kani setup",
  );
}

// --- locate repo in the detected environment --------------------------------

let repoDir;
if (distro === "native") {
  repoDir = REPO;
} else {
  repoDir = toWslPath(REPO);
  if (repoDir === null) {
    verdict("unsupported", `repo path ${REPO} has no drive-letter mapping for WSL (/mnt/<drive>)`);
  }
  const ok = wslProbe(distro, `test -d '${repoDir}/crates/${PACKAGE}'`);
  if (ok.status !== 0) {
    verdict(
      "not_run",
      `repo not visible inside WSL ${distro} at ${repoDir} ` + `(Windows path: ${REPO})`,
    );
  }
}

// --- kani flag capability ----------------------------------------------------

const helpOut =
  distro === "native" ? probe(["cargo", "kani", "--help"]) : wslProbe(distro, "cargo kani --help");
// --tests: tests/ 内の harness を探索対象に含める。--harness:
// verification を filter 一致の harness に限る — 同じ
// formal_wav.rs の proptest #[test] を proof 対象として取り込まない
// ため、kani_proofs module 名で絞る。
const needFlags = ["--tests", "--harness"];
const missing = needFlags.filter((f) => !new RegExp(`(^|\\s)${f}(\\s|$)`, "m").test(helpOut.out));
if (missing.length > 0) {
  verdict(
    "not_run",
    `kani-verifier (env: ${distro}) lacks ${missing.join("/")}; ` +
      `harnesses live under ${PACKAGE}/tests/formal_wav.rs ` +
      `and ${PACKAGE}/tests/formal_containers.rs ` +
      "and cannot be reached",
  );
}

// --- run ---------------------------------------------------------------------
//
// Harnesses run SEQUENTIALLY, one `cargo kani` each: kani-driver does
// not flush per-harness results until the process exits, so a kill
// mid-run loses completed proofs. Per-harness runs keep every
// finished verdict. TIMEOUT_MS is the per-harness budget (env
// override: AL_KANI_TIMEOUT_MS). Keep this list in sync with the
// `mod kani_proofs` blocks in tests/formal_wav.rs and
// tests/formal_containers.rs.
const HARNESSES = [
  "decode_never_panics_on_bounded_input",
  "writer_round_trip_is_exact",
  "header_reads_agree_with_writer_layout",
  "containers_never_panic_on_bounded_input",
  "detect_format_signature_rules",
  "mp4_writer_output_decodes",
  "avi_writer_output_decodes",
  "mutated_mp4_never_panics",
  "mutated_avi_never_panics",
];

// script は base64 で渡す — wsl.exe → bash -lc の入れ子 quoting で
// 変数展開・$ 記号が壊れるのを避けるため。
const b64sh = (s) => `echo ${Buffer.from(s, "utf8").toString("base64")} | base64 -d | bash`;

// WSL 側: source mirror を先に1回更新(target/ だけ保持)→ mirror 内で
// 各 harness を実行。tar の対象は cargo build に必要な最小集合
// (Cargo.toml/Cargo.lock/.cargo/crates/dictionaries)。
// al-core/build.rs は祖先を遡って Cargo.lock を読み、al-media の
// lib test は crate 外の dictionaries/*.json を include_str! する。
if (distro !== "native") {
  const mirrorScript = [
    `dst="${WSL_MIRROR}"`,
    `mkdir -p "$dst"`,
    `find "$dst" -mindepth 1 -maxdepth 1 ! -name target -exec rm -rf {} +`,
    `tar -C '${repoDir}' -cf - Cargo.toml Cargo.lock .cargo crates dictionaries | tar -C "$dst" -xf -`,
  ].join(" && ");
  const mirror = spawnSync("wsl.exe", ["-d", distro, "--", "bash", "-lc", b64sh(mirrorScript)], {
    encoding: "utf8",
    timeout: 300_000,
  });
  if (mirror.status !== 0) {
    verdict(
      "not_run",
      `source mirror into WSL ${distro} failed ` +
        `(exit ${mirror.status}): ${(mirror.stderr ?? "").trim()}`,
    );
  }
}

function runKani(harness) {
  const kaniArgs = ["kani", "-p", PACKAGE, "--tests", "--harness", harness];
  if (distro === "native") {
    return spawnSync("cargo", kaniArgs, {
      cwd: repoDir,
      encoding: "utf8",
      timeout: TIMEOUT_MS,
      maxBuffer: 64 * 1024 * 1024,
    });
  }
  const script = `cd "${WSL_MIRROR}" && env -u GIT_DIR -u GIT_WORK_TREE cargo ${kaniArgs.join(" ")}`;
  return spawnSync("wsl.exe", ["-d", distro, "--", "bash", "-lc", b64sh(script)], {
    encoding: "utf8",
    timeout: TIMEOUT_MS,
    maxBuffer: 64 * 1024 * 1024,
  });
}

function cleanupOrphans() {
  // spawnSync timeout kills wsl.exe, but the WSL-side verifier
  // processes can survive as orphans and keep burning CPU — clean
  // them up so a timed-out harness leaves nothing behind. Scope by
  // the mirror path: cbmc/kani-driver args contain the .out under
  // it, so only this lane's proofs are matched.
  wslProbe(distro, `pkill -f 'al-formal-kani/src' ; pkill -f 'cargo kani -p ${PACKAGE}' ; true`);
}

const results = [];
for (const harness of HARNESSES) {
  const r = runKani(harness);
  const out = `${r.stdout ?? ""}\n${r.stderr ?? ""}`;
  const timedOut = r.error?.code === "ETIMEDOUT";
  if (timedOut && distro !== "native") cleanupOrphans();
  results.push({
    harness,
    ok: /VERIFICATION:- SUCCESSFUL/.test(out),
    bad: /VERIFICATION:- FAILED/.test(out),
    timedOut,
    status: r.status,
    tail: out.trim().split(/\r?\n/).slice(-8).join("\n"),
  });
}

const verified = results.filter((r) => r.ok).map((r) => r.harness);
const failedList = results.filter((r) => r.bad);
const stalled = results.filter((r) => !r.ok && !r.bad);

if (failedList.length > 0) {
  verdict(
    "fail",
    `${failedList.length} harness(es) failed verification ` +
      `(${verified.length} verified): ` +
      failedList.map((r) => r.harness).join(", "),
    { env: distro, details: results },
  );
}
if (stalled.length === 0) {
  verdict("pass", `all ${verified.length} kani harness(es) verified`, {
    env: distro,
  });
}
verdict(
  "not_run",
  `${verified.length}/${HARNESSES.length} harness(es) verified; ` +
    `incomplete: ${stalled.map((r) => `${r.harness}(${r.timedOut ? `timeout ${TIMEOUT_MS}ms` : `exit ${r.status}`})`).join(", ")}`,
  { env: distro, details: results },
);

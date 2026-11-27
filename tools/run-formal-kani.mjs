// Issue #256 (D 層 formal verification): al-media WAV codec(第 1 標的)
// と MP4/AVI container parse(第 2 標的)の #[cfg(kani)] bounded proof
// harness(tests/formal_wav.rs・tests/formal_containers.rs と、
// WAV-CHUNK-003/WAV-SCAN-004/WAV-RANGE-008 replacement については
// src/wav.rs、MP4-BOX-002/MP4-SCAN-003/AVI-CHUNK-001/AVI-MOVI-003
// tag identity/movi entry composition/movi scan collection/stream kind
// classification/strh declaration/strf declaration/strf capture については src/mp4.rs/src/avi.rs、
// audio strf WAVEFORMATEX declaration については src/wav.rs、
// format signature detection については src/format.rs)を
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
import fs from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";

const REPO = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");
const PACKAGE = "al-media";
const TIMEOUT_MS = Number(process.env.AL_KANI_TIMEOUT_MS ?? 45 * 60 * 1000);
const WSL_MIRROR = "$HOME/.cache/al-formal-kani/src";
// 証拠記録用: AL_KANI_LOG_DIR が指定された場合のみ、各 harness の
// raw 出力(stdout+stderr と終了メタ)を <dir>/kani-<harness>.txt に、
// 最終 JSON verdict を <dir>/verdict.json に書き出す。未指定では
// 何も書かない — env-detect・mirror・逐次実行・JSON verdict・
// fail-closed exit の意味論は不変。
const LOG_DIR = process.env.AL_KANI_LOG_DIR ? path.resolve(process.env.AL_KANI_LOG_DIR) : null;

function verdict(v, reason, extra = {}) {
  const rec = { lane: "test-formal-kani", verdict: v, reason, ...extra };
  const line = `${JSON.stringify(rec)}\n`;
  process.stdout.write(line);
  if (LOG_DIR) {
    fs.mkdirSync(LOG_DIR, { recursive: true });
    fs.writeFileSync(path.join(LOG_DIR, "verdict.json"), line);
  }
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
// tests/formal_containers.rs, and the `mod proofs` blocks in src/wav.rs,
// src/mp4.rs, src/avi.rs, and src/format.rs.
const WAV_CHUNK_SCAN_HARNESSES = [
  "wav_chunk_boundary_step_exact",
  "wav_chunk_scan_progress_bounded_exact",
];
const WAV_RANGE_HARNESSES = [
  "wav_range_plan_valid_ranges_exact",
  "wav_range_plan_invalid_ranges_error",
];
const MP4_AVI_BOUNDARY_HARNESSES = ["mp4_box_header_step_exact", "avi_chunk_step_exact"];
const MP4_SCAN_HARNESSES = ["mp4_box_scan_progress_bounded_exact"];
const AVI_MOVI_TAG_HARNESSES = ["avi_movi_packet_tag_exact"];
const AVI_MOVI_COMPOSITION_HARNESSES = ["avi_movi_entry_decision_exact"];
const AVI_MOVI_SCAN_HARNESSES = ["avi_movi_scan_collection_bounded_exact"];
const AVI_STREAM_KIND_HARNESSES = ["avi_stream_kind_classification_bounded_exact"];
const AVI_STRH_DECLARATION_HARNESSES = ["avi_strh_declaration_bounded_exact"];
const AVI_STRF_DECLARATION_HARNESSES = ["avi_video_strf_declaration_bounded_exact"];
const AVI_STRF_AUDIO_DECLARATION_HARNESSES = ["avi_audio_strf_declaration_bounded_exact"];
const FORMAT_SIGNATURE_HARNESSES = ["format_signature_detection_bounded_exact"];
const AVI_STRF_CAPTURE_HARNESSES = [
  "avi_strl_entry_classification_exact",
  "avi_strf_capture_ordering_bounded_exact",
  "avi_strf_materialize_bounded_exact",
];

const HARNESS_GROUPS = {
  "wav-chunk-scan": WAV_CHUNK_SCAN_HARNESSES,
  "wav-range": WAV_RANGE_HARNESSES,
  "mp4-avi-boundary": MP4_AVI_BOUNDARY_HARNESSES,
  "mp4-scan": MP4_SCAN_HARNESSES,
  "avi-movi-tag": AVI_MOVI_TAG_HARNESSES,
  "avi-movi-composition": AVI_MOVI_COMPOSITION_HARNESSES,
  "avi-movi-scan": AVI_MOVI_SCAN_HARNESSES,
  "avi-stream-kind": AVI_STREAM_KIND_HARNESSES,
  "avi-strh-declaration": AVI_STRH_DECLARATION_HARNESSES,
  "avi-strf-declaration": AVI_STRF_DECLARATION_HARNESSES,
  "avi-strf-audio-declaration": AVI_STRF_AUDIO_DECLARATION_HARNESSES,
  "format-signature": FORMAT_SIGNATURE_HARNESSES,
  "avi-strf-capture": AVI_STRF_CAPTURE_HARNESSES,
};

const HARNESSES = [
  "decode_wav_never_panics_signed",
  "wav_frame_count_never_panics_signed",
  "read_wav_range_never_panics_signed",
  "wav_reads_never_panics_unsigned",
  "writer_round_trip_pcm_s16",
  "writer_round_trip_pcm_s24",
  "writer_round_trip_pcm_s32",
  "writer_round_trip_float32",
  "writer_round_trip_float64",
  ...WAV_CHUNK_SCAN_HARNESSES,
  ...WAV_RANGE_HARNESSES,
  ...MP4_AVI_BOUNDARY_HARNESSES,
  ...MP4_SCAN_HARNESSES,
  ...AVI_MOVI_TAG_HARNESSES,
  ...AVI_MOVI_COMPOSITION_HARNESSES,
  ...AVI_MOVI_SCAN_HARNESSES,
  ...AVI_STREAM_KIND_HARNESSES,
  ...AVI_STRH_DECLARATION_HARNESSES,
  ...AVI_STRF_DECLARATION_HARNESSES,
  ...AVI_STRF_AUDIO_DECLARATION_HARNESSES,
  ...FORMAT_SIGNATURE_HARNESSES,
  ...AVI_STRF_CAPTURE_HARNESSES,
  "mp4_never_panics_ftyp_signed",
  "avi_never_panics_riff_signed",
  "containers_never_panics_unsigned",
  "detect_format_signature_rules",
  "mp4_writer_output_decodes",
  "avi_writer_output_decodes",
  "mutated_mp4_never_panics",
  "truncated_mp4_never_panics",
  "mutated_avi_never_panics",
  "truncated_avi_never_panics",
];

for (const [group, members] of Object.entries(HARNESS_GROUPS)) {
  const missing = members.filter((name) => !HARNESSES.includes(name));
  if (missing.length > 0) {
    verdict("fail", `harness group ${group} is not covered by registry: ${missing.join(", ")}`);
  }
}

// AL_KANI_HARNESSES="a,b,c" で登録済み名の subset だけを逐次実行する
// env-gated な追加(harness 分割・bound 調整の個別試行用)。未指定は
// 現行どおり全件 — 既定挙動は不変。未登録名は fail-closed で not_run。
const GROUP = process.env.AL_KANI_HARNESS_GROUP?.trim() || null;
const SUBSET = process.env.AL_KANI_HARNESSES
  ? new Set(
      process.env.AL_KANI_HARNESSES.split(",")
        .map((s) => s.trim())
        .filter(Boolean),
    )
  : null;
if (GROUP && SUBSET) {
  verdict("not_run", "AL_KANI_HARNESS_GROUP and AL_KANI_HARNESSES are mutually exclusive");
}
if (GROUP && !HARNESS_GROUPS[GROUP]) {
  verdict(
    "not_run",
    `unknown harness group in AL_KANI_HARNESS_GROUP: ${GROUP} ` +
      `(registered: ${Object.keys(HARNESS_GROUPS).join(", ")})`,
  );
}
if (SUBSET) {
  const unknown = [...SUBSET].filter((n) => !HARNESSES.includes(n));
  if (unknown.length > 0) {
    verdict(
      "not_run",
      `unknown harness name(s) in AL_KANI_HARNESSES: ${unknown.join(", ")} ` +
        `(registered: ${HARNESSES.length})`,
    );
  }
}
const SELECTED = GROUP
  ? HARNESS_GROUPS[GROUP]
  : SUBSET
    ? HARNESSES.filter((h) => SUBSET.has(h))
    : HARNESSES;

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

const GNU_TIME =
  distro === "native"
    ? fs.existsSync("/usr/bin/time")
    : wslProbe(distro, "test -x /usr/bin/time").status === 0;

function runKani(harness) {
  // al#274: representative proofs adopted per-harness #[kani::stub] models.
  // `-Z stubbing` only enables the attribute mechanism; harnesses without
  // stub attributes keep the same verification semantics, so the lane does
  // not need a second env-gated mode.
  const kaniArgs = ["kani", "-p", PACKAGE, "--tests", "--harness", harness, "-Z", "stubbing"];
  if (distro === "native") {
    const args = GNU_TIME ? ["/usr/bin/time", "-v", "cargo", ...kaniArgs] : ["cargo", ...kaniArgs];
    return spawnSync(args[0], args.slice(1), {
      cwd: repoDir,
      encoding: "utf8",
      timeout: TIMEOUT_MS,
      maxBuffer: 64 * 1024 * 1024,
    });
  }
  const cargoCommand = GNU_TIME
    ? `/usr/bin/time -v cargo ${kaniArgs.join(" ")}`
    : `cargo ${kaniArgs.join(" ")}`;
  const script = `cd "${WSL_MIRROR}" && env -u GIT_DIR -u GIT_WORK_TREE ${cargoCommand}`;
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

if (LOG_DIR) fs.mkdirSync(LOG_DIR, { recursive: true });
const results = [];
for (const harness of SELECTED) {
  const startedAt = performance.now();
  const r = runKani(harness);
  const elapsedMs = Math.round(performance.now() - startedAt);
  const out = `${r.stdout ?? ""}\n${r.stderr ?? ""}`;
  const timedOut = r.error?.code === "ETIMEDOUT";
  const maxRssKb = out.match(/Maximum resident set size \(kbytes\):\s*(\d+)/)?.[1];
  if (timedOut && distro !== "native") cleanupOrphans();
  if (LOG_DIR) {
    fs.writeFileSync(
      path.join(LOG_DIR, `kani-${harness}.txt`),
      `# exit=${r.status} signal=${r.signal ?? ""} timedOut=${timedOut} error=${r.error?.code ?? ""} elapsed_ms=${elapsedMs} max_rss_kb=${maxRssKb ?? "unmeasured"}\n${out}`,
    );
  }
  results.push({
    harness,
    ok: /VERIFICATION:- SUCCESSFUL/.test(out),
    bad: /VERIFICATION:- FAILED/.test(out),
    timedOut,
    status: r.status,
    elapsed_ms: elapsedMs,
    max_rss_kb: maxRssKb ? Number(maxRssKb) : null,
    peak_memory_measurement: GNU_TIME ? "/usr/bin/time -v" : "unmeasured",
    tail: out.trim().split(/\r?\n/).slice(-8).join("\n"),
  });
}

const verified = results.filter((r) => r.ok).map((r) => r.harness);
const failedList = results.filter((r) => r.bad);
const stalled = results.filter((r) => !r.ok && !r.bad);
const measuredRss = results.map((r) => r.max_rss_kb).filter((v) => typeof v === "number");
// Additive lane-level cost summary. `details` remains the per-harness
// array for compatibility; this object only aggregates the selected run.
const costSummary = {
  selected_proofs: SELECTED.length,
  verified_proofs: verified.length,
  total_elapsed_ms: results.reduce((sum, r) => sum + r.elapsed_ms, 0),
  max_observed_rss_kb: measuredRss.length > 0 ? Math.max(...measuredRss) : null,
  rss_measured_proofs: measuredRss.length,
  peak_memory_measurement: GNU_TIME ? "/usr/bin/time -v" : "unmeasured",
};

if (failedList.length > 0) {
  verdict(
    "fail",
    `${failedList.length} harness(es) failed verification ` +
      `(${verified.length} verified): ` +
      failedList.map((r) => r.harness).join(", "),
    { env: distro, details: results, cost_summary: costSummary },
  );
}
if (stalled.length === 0) {
  verdict("pass", `all ${verified.length} kani harness(es) verified`, {
    env: distro,
    details: results,
    cost_summary: costSummary,
  });
}
verdict(
  "not_run",
  `${verified.length}/${SELECTED.length} harness(es) verified; ` +
    `incomplete: ${stalled.map((r) => `${r.harness}(${r.timedOut ? `timeout ${TIMEOUT_MS}ms` : `exit ${r.status}`})`).join(", ")}`,
  { env: distro, details: results, cost_summary: costSummary },
);

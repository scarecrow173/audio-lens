// 子プロセスの CPU 時間・RSS をポーリングする。
// Windows: PowerShell Get-Process(CPU[s], WorkingSet64)
// POSIX: /proc/<pid>/stat(utime+stime[jiffies])と status VmRSS
// いずれも「標本値」— stage 境界には帰属しない(ADR: stage 別
// busy/CPU/RSS/IO は実測未実施)。
import { execFile } from "node:child_process";
import { readFile } from "node:fs/promises";

const IS_WIN = process.platform === "win32";

async function sampleWindows(pid) {
  const out = await new Promise((resolve, reject) => {
    execFile(
      "powershell.exe",
      [
        "-NoProfile",
        "-Command",
        `$p = Get-Process -Id ${pid} -ErrorAction SilentlyContinue; ` +
          `if ($p) { "{0}|{1}" -f $p.CPU, $p.WorkingSet64 }`,
      ],
      { timeout: 5000 },
      (err, stdout) => (err ? reject(err) : resolve(stdout.trim())),
    );
  });
  const [cpu, rss] = out.split("|").map(Number);
  return { cpuSec: cpu, rssBytes: rss };
}

async function samplePosix(pid) {
  const stat = await readFile(`/proc/${pid}/stat`, "utf8");
  // comm 内の空白対策: 最後の ")" 以降を使う
  const close = stat.lastIndexOf(")");
  const rest = stat.slice(close + 2).split(" ");
  const utime = Number(rest[11]); // field 14
  const stime = Number(rest[12]); // field 15
  const hz = 100; // 一般的な CLK_TCK。正確には getconf。
  const status = await readFile(`/proc/${pid}/status`, "utf8");
  const m = status.match(/VmRSS:\s+(\d+)\s+kB/);
  return {
    cpuSec: (utime + stime) / hz,
    rssBytes: m ? Number(m[1]) * 1024 : null,
  };
}

export function monitorProcess(child, intervalMs = 1000) {
  const samples = [];
  const timer = setInterval(async () => {
    try {
      const s = IS_WIN ? await sampleWindows(child.pid) : await samplePosix(child.pid);
      samples.push({ t: Date.now(), ...s });
    } catch {
      // プロセス終了後のサンプル失敗は無視
    }
  }, intervalMs);
  timer.unref?.();

  return {
    async finish() {
      clearInterval(timer);
      let peakRss = null;
      let lastCpu = null;
      for (const s of samples) {
        if (s.rssBytes != null && (peakRss == null || s.rssBytes > peakRss)) {
          peakRss = s.rssBytes;
        }
        if (s.cpuSec != null) lastCpu = s.cpuSec;
      }
      return {
        method: IS_WIN ? "windows:Get-Process" : "posix:/proc",
        interval_ms: intervalMs,
        sample_count: samples.length,
        cpu_sec_last_sample: lastCpu,
        peak_rss_bytes: peakRss,
        note: "poll 標本値。peak RSS は真のピークを下回り得る下限推定",
      };
    },
  };
}

// AudioLens GUI frontend — rvf.ipc.v1 上の al.* op を駆動する。
// 波形(RVBW)・スペクトログラム(RVST)は rvf://data の pull path から
// 有界取得し、解析データの全量ロードは行わない。

const $ = (s) => document.querySelector(s);
const call = (op, params) => __rvf.call(op, params || {});
const log = (m, err) => {
  const el = document.createElement("div");
  if (err) el.className = "err";
  el.textContent = m;
  $("#log").appendChild(el);
  $("#log").scrollTop = $("#log").scrollHeight;
};

const state = {
  session: null,
  timeline: null, // al.ds.overview の timeline 部
  events: [],
  aligns: [],
  view: { input: null, channel: 0, start_s: 0, end_s: 1 },
  sel: null, // 範囲選択 {start_s, end_s}
  runningJob: null,
};

// ---------- helpers ----------

async function pull(name) {
  // rvf://data は len cap あり — X-Rvf-Total までページングする。
  const CHUNK = 4 * 1024 * 1024;
  const first = await fetch(__rvf.dataUrl(name, 0, CHUNK));
  if (!first.ok) throw new Error(`pull failed: ${first.status}`);
  const total = Number(first.headers.get("X-Rvf-Total") || 0);
  const buf = new Uint8Array(total);
  buf.set(new Uint8Array(await first.arrayBuffer()), 0);
  for (let off = CHUNK; off < total; off += CHUNK) {
    const r = await fetch(__rvf.dataUrl(name, off, CHUNK));
    if (!r.ok) throw new Error(`pull failed: ${r.status}`);
    buf.set(new Uint8Array(await r.arrayBuffer()), off);
  }
  return buf;
}

function fitCanvas(c) {
  const dpr = devicePixelRatio || 1;
  const w = c.clientWidth || 600;
  const h = Number(c.getAttribute("height")) || c.clientHeight || 120;
  c.width = Math.round(w * dpr);
  c.height = Math.round(h * dpr);
  const g = c.getContext("2d");
  g.setTransform(dpr, 0, 0, dpr, 0, 0);
  return { g, w, h };
}

// RVBW: 16B header + 44B/bin (u64 start, u64 end, f64 min, f64 max, f64 rms, u64 count)
function decodeWaveform(buf) {
  const dv = new DataView(buf.buffer, buf.byteOffset, buf.byteLength);
  if (String.fromCharCode(...buf.slice(0, 4)) !== "RVBW") throw new Error("bad RVBW magic");
  const n = dv.getUint32(8, true);
  const bins = [];
  for (let i = 0; i < n; i++) {
    const o = 16 + i * 44;
    bins.push({
      start: Number(dv.getBigUint64(o, true)),
      end: Number(dv.getBigUint64(o + 8, true)),
      min: dv.getFloat64(o + 16, true),
      max: dv.getFloat64(o + 24, true),
      rms: dv.getFloat64(o + 32, true),
      count: Number(dv.getBigUint64(o + 40, true)),
    });
  }
  return bins;
}

// RVST: 24B header (magic, ver, dtype, cols, rows, frame_start u64) + rows*cols values
function decodeTile(buf) {
  const dv = new DataView(buf.buffer, buf.byteOffset, buf.byteLength);
  if (String.fromCharCode(...buf.slice(0, 4)) !== "RVST") throw new Error("bad RVST magic");
  const dtype = dv.getUint16(6, true);
  const cols = dv.getUint32(8, true);
  const rows = dv.getUint32(12, true);
  const frameStart = Number(dv.getBigUint64(16, true));
  const n = rows * cols;
  let values;
  if (dtype === 1) values = new Float32Array(buf.buffer, buf.byteOffset + 24, n);
  else if (dtype === 0) values = new Float64Array(buf.buffer, buf.byteOffset + 24, n);
  else values = new Uint8Array(buf.buffer, buf.byteOffset + 24, n);
  return { cols, rows, frameStart, values };
}

// 簡易 inferno 風 colormap (表示正規化はここで行う — 測定値の
// 変換ではなく描画用の dB 正規化)。
function colormap(t) {
  const stops = [
    [0, 0, 4],
    [31, 12, 72],
    [85, 15, 109],
    [136, 34, 106],
    [186, 54, 85],
    [227, 89, 51],
    [249, 140, 10],
    [249, 201, 50],
    [252, 255, 164],
  ];
  const i = Math.min(stops.length - 2, Math.floor(t * (stops.length - 1)));
  const f = t * (stops.length - 1) - i;
  const a = stops[i],
    b = stops[i + 1];
  return [
    Math.round(a[0] + (b[0] - a[0]) * f),
    Math.round(a[1] + (b[1] - a[1]) * f),
    Math.round(a[2] + (b[2] - a[2]) * f),
  ];
}

// ---------- rendering ----------

const DB_MIN = -100,
  DB_MAX = 0;
const toDb = (m) => (m > 0 ? 20 * Math.log10(m) : -Infinity);
const dbUnit = (db) => Math.max(0, Math.min(1, (db - DB_MIN) / (DB_MAX - DB_MIN)));

function drawWave(bins, meta) {
  const { g, w, h } = fitCanvas($("#wave"));
  g.fillStyle = "#101216";
  g.fillRect(0, 0, w, h);
  if (!bins.length) return;
  const rate = meta.sample_rate;
  const s0 = state.view.start_s * rate,
    s1 = state.view.end_s * rate;
  const span = Math.max(1, s1 - s0);
  const mid = h / 2,
    amp = h / 2 - 6;
  g.fillStyle = "#4f8fe8";
  for (const b of bins) {
    const x0 = ((b.start - s0) / span) * w;
    const x1 = Math.max(x0 + 1, ((b.end - s0) / span) * w);
    if (!Number.isFinite(b.min) || !Number.isFinite(b.max)) continue;
    g.fillRect(x0, mid - b.max * amp, x1 - x0, Math.max(1, (b.max - b.min) * amp));
  }
  // イベント marker
  drawEventMarkers(g, w, h, rate, s0, span, "#e0a86c");
}

function drawSpec(tile, _meta) {
  const { g, w, h } = fitCanvas($("#spec"));
  const img = g.createImageData(tile.cols, tile.rows);
  for (let r = 0; r < tile.rows; r++) {
    for (let c = 0; c < tile.cols; c++) {
      const v = tile.values[r * tile.cols + c];
      const _i = r * tile.cols + c;
      const off = ((tile.rows - 1 - r) * tile.cols + c) * 4;
      if (!Number.isFinite(v)) {
        img.data[off + 3] = 0;
        continue;
      }
      const [rr, gg, bb] = colormap(dbUnit(toDb(v)));
      img.data[off] = rr;
      img.data[off + 1] = gg;
      img.data[off + 2] = bb;
      img.data[off + 3] = 255;
    }
  }
  const tmp = document.createElement("canvas");
  tmp.width = tile.cols;
  tmp.height = tile.rows;
  tmp.getContext("2d").putImageData(img, 0, 0);
  g.imageSmoothingEnabled = false;
  g.drawImage(tmp, 0, 0, w, h);
}

function drawEventMarkers(g, w, h, rate, s0, span, color) {
  g.fillStyle = color;
  for (const e of state.events) {
    const t = Number(e.start_s) * rate;
    const x = ((t - s0) / span) * w;
    if (x < 0 || x > w) continue;
    g.fillRect(x, 0, 2, h);
  }
}

function drawTimeline() {
  const { g, w, h } = fitCanvas($("#timeline"));
  g.fillStyle = "#101216";
  g.fillRect(0, 0, w, h);
  const tl = state.timeline;
  if (!tl) return;
  const dur = Math.max(0.001, ...tl.inputs.map((i) => i.duration_s || 0));
  const span = Math.max(0.001, state.view.end_s - state.view.start_s);
  // locate alignment 区間(capture_range は sample 単位)
  const cap = tl.inputs.find((i) => i.input_id === state.view.input) || tl.inputs[0];
  const rate = cap ? cap.sample_rate : 48000;
  g.fillStyle = "#3a6ea5";
  for (const a of state.aligns) {
    if (a.status !== "accepted") continue;
    const st = Number(a.capture_start) / rate,
      en = Number(a.capture_end) / rate;
    const x0 = ((st - state.view.start_s) / span) * w;
    const x1 = ((en - state.view.start_s) / span) * w;
    g.fillRect(x0, h - 14, Math.max(2, x1 - x0), 10);
  }
  // event ticks
  g.fillStyle = "#e0a86c";
  for (const e of state.events) {
    const x = ((Number(e.start_s) - state.view.start_s) / span) * w;
    if (x >= 0 && x <= w) g.fillRect(x, 2, 2, 12);
  }
  // viewport 位置インジケータ(全期間に対する現在範囲)
  g.strokeStyle = "#4f8fe8";
  const vx0 = (state.view.start_s / dur) * w;
  const vx1 = (state.view.end_s / dur) * w;
  g.strokeRect(vx0, h - 26, Math.max(2, vx1 - vx0), 22);
  if (state.sel) {
    g.fillStyle = "rgba(79,143,232,.25)";
    const x0 = ((state.sel.start_s - state.view.start_s) / span) * w;
    const x1 = ((state.sel.end_s - state.view.start_s) / span) * w;
    g.fillRect(x0, 0, x1 - x0, h);
  }
  $("#tlRange").textContent =
    `${state.view.start_s.toFixed(2)}s – ${state.view.end_s.toFixed(2)}s / total ${dur.toFixed(1)}s`;
}

async function redraw() {
  if (!state.session || !state.view.input) return;
  const v = state.view;
  try {
    // 波形 (RVBW pull)
    const wr = await call("al.view.waveform", {
      session: state.session,
      input_id: v.input,
      channel: v.channel,
      start_s: v.start_s,
      end_s: v.end_s,
      bins: 1200,
    });
    const wbins = decodeWaveform(await pull(wr.source));
    drawWave(wbins, wr.meta);
    // スペクトログラム (RVST pull)
    const spec = await call("al.view.spectrogram", {
      session: state.session,
      input_id: v.input,
      channel: v.channel,
      start_s: v.start_s,
      end_s: v.end_s,
      rows: 220,
      cols: 1200,
    }).catch(() => null); // STFT 中間生成物が無い入力は非表示
    if (spec) drawSpec(decodeTile(await pull(spec.source)), spec.meta);
    else {
      const { g, w, h } = fitCanvas($("#spec"));
      g.fillStyle = "#101216";
      g.fillRect(0, 0, w, h);
    }
    drawTimeline();
  } catch (e) {
    log(`view: ${e.message}`, true);
  }
}

// ---------- detail panel ----------

function showDetail(title, obj) {
  const el = $("#detail");
  const pre = document.createElement("pre");
  pre.textContent = typeof obj === "string" ? obj : JSON.stringify(obj, null, 1);
  el.innerHTML = `<h3 style="margin:4px 0">${title}</h3>`;
  el.appendChild(pre);
}

function showRecords(records, truncated) {
  const el = $("#detail");
  let html = `<h3 style="margin:4px 0">FeatureRecords (${records.length}${truncated ? "+" : ""})</h3>`;
  html +=
    "<table><tr><th>feature_id</th><th>value</th><th>unit</th><th>validity</th><th>conf</th><th>support</th><th>artifact</th></tr>";
  for (const r of records.slice(0, 300)) {
    const v = r.value && typeof r.value === "object" ? JSON.stringify(r.value) : r.value;
    html +=
      `<tr><td>${r.feature_id || ""}</td><td>${typeof v === "number" ? v.toPrecision(5) : (v ?? "")}</td>` +
      `<td>${r.unit || ""}</td><td class="${r.validity === "valid" ? "valid" : "invalid"}">${r.validity || ""}</td>` +
      `<td>${r.confidence || ""}</td><td>${r.support_count ?? ""}</td><td>${r._artifact || ""}</td></tr>`;
  }
  el.innerHTML = `${html}</table>`;
  for (const [i, tr] of [...el.querySelectorAll("tr")].entries()) {
    if (i === 0) continue;
    tr.style.cursor = "pointer";
    tr.onclick = () => showDetail("record detail", records[i - 1]);
  }
}

// ---------- ops wiring ----------

async function refreshJobs() {
  try {
    const r = await call("al.job.list", {});
    const ul = $("#jobList");
    ul.innerHTML = "";
    for (const j of r.jobs) {
      const li = document.createElement("li");
      const failed = j.stages.filter((s) => s.status === "failed").length;
      li.textContent = `${j.job_id} ${j.dataset_ready ? "[ds]" : ""}${failed ? ` [${failed} failed]` : ""}`;
      li.title = j.inputs.map((i) => `${i.input_id}(${i.role}): ${i.path}`).join("\n");
      li.onclick = () => inspectJob(j.job_id);
      ul.appendChild(li);
    }
    $("#status").textContent = `${r.jobs.length} job(s)`;
  } catch (e) {
    log(`job.list: ${e.message}`, true);
  }
}

async function inspectJob(jobId) {
  try {
    const r = await call("al.job.inspect", { job_id: jobId });
    const stages = r.manifest.stages || [];
    $("#jobStageList").innerHTML = stages
      .map(
        (s) =>
          `<div class="${s.status}">${s.stage}: ${s.status}${s.error ? ` — ${s.error}` : ""}${s.duration_ms != null ? ` (${s.duration_ms}ms)` : ""}</div>`,
      )
      .join("");
    if (r.dataset_ready) openDataset({ job_id: jobId });
    else showDetail(`inspect ${jobId}`, r.manifest);
  } catch (e) {
    log(`job.inspect: ${e.message}`, true);
  }
}

async function openDataset(params) {
  try {
    const r = await call("al.ds.open", params);
    state.session = r.session;
    const ov = await call("al.ds.overview", { session: r.session });
    state.timeline = ov.timeline;
    // input 選択肢
    const sel = $("#tlInput");
    sel.innerHTML = "";
    for (const i of ov.timeline.inputs) {
      const o = document.createElement("option");
      o.value = i.input_id;
      o.textContent = `${i.input_id} (${i.sample_rate}Hz ${i.channels}ch)`;
      sel.appendChild(o);
    }
    if (ov.timeline.inputs.length) {
      state.view.input = ov.timeline.inputs[0].input_id;
      state.view.end_s = Math.max(...ov.timeline.inputs.map((i) => i.duration_s || 1));
      state.view.start_s = 0;
    }
    // events + locate (markers)
    try {
      state.events = (await call("al.ds.events", { session: r.session })).rows || [];
    } catch {
      state.events = [];
    }
    try {
      state.aligns = (await call("al.ds.locate", { session: r.session })).alignments || [];
    } catch {
      state.aligns = [];
    }
    // artifact tree
    const ul = $("#dsTree");
    ul.innerHTML = "";
    const arts = (await call("al.ds.artifacts", { session: r.session })).artifacts;
    for (const a of arts) {
      const li = document.createElement("li");
      li.textContent = a.path;
      li.onclick = async () => {
        try {
          showDetail(a.path, await call("al.ds.artifact", { session: r.session, path: a.path }));
        } catch (e) {
          log(`ds.artifact: ${e.message}`, true);
        }
      };
      ul.appendChild(li);
    }
    // feature record 一覧(初期表示)
    try {
      const f = await call("al.ds.features", { session: r.session, limit: 300 });
      showRecords(f.records, f.truncated);
    } catch {
      /* features 未生成なら空のまま */
    }
    $("#tlControls").hidden = false;
    $("#status").textContent = `dataset: ${r.root}`;
    redraw();
  } catch (e) {
    log(`ds.open: ${e.message}`, true);
  }
}

function inputRow() {
  const div = document.createElement("div");
  div.className = "inputEntry";
  div.innerHTML =
    '<select><option value="capture_bug">capture_bug</option>' +
    '<option value="capture_reference">capture_reference</option>' +
    '<option value="source_asset">source_asset</option>' +
    '<option value="other">other</option></select>' +
    '<input placeholder="path" />' +
    '<input placeholder="input_id (任意)" style="max-width:110px" />' +
    '<button class="rm">×</button>';
  div.querySelector(".rm").onclick = () => div.remove();
  return div;
}

async function runJob() {
  const inputs = [...$("#newJobInputs").children]
    .map((d) => {
      const [sel, path, id] = d.querySelectorAll("select,input");
      return { role: sel.value, path: path.value, ...(id.value ? { input_id: id.value } : {}) };
    })
    .filter((i) => i.path);
  if (!inputs.length) return log("run: 入力が未指定", true);
  try {
    const r = await call("al.job.run", {
      inputs,
      stages: [
        "ingest",
        "locate",
        "cheap_analysis",
        "reference_analysis",
        "expensive_analysis",
        "dataset",
      ],
    });
    state.runningJob = r.job;
    $("#bCancel").disabled = false;
    log(`job started: ${r.job}`);
  } catch (e) {
    log(`job.run: ${e.message}`, true);
  }
}

// ---------- events ----------

__rvf.onEvent((m) => {
  if (m.kind === "job") {
    if (m.state === "progress") {
      $("#jobBar").style.width = `${(m.done / Math.max(1, m.total)) * 100}%`;
    } else if (m.state === "finished") {
      $("#jobBar").style.width = "100%";
      state.runningJob = null;
      $("#bCancel").disabled = true;
      log(`job ${m.job} finished: ${m.data?.job_id || ""}`);
      if (m.data?.job_id) {
        refreshJobs();
        inspectJob(m.data.job_id);
      }
    } else {
      state.runningJob = null;
      $("#bCancel").disabled = true;
      log(`job ${m.job} ${m.state}: ${m.error ? m.error.message : ""}`, true);
    }
  } else if (m.kind === "lifecycle") {
    log(`lifecycle: ${m.state}`);
  }
});

$("#bSetBase").onclick = async () => {
  try {
    await call("al.app.configure", { base_dir: $("#baseDir").value });
    log("base_dir 設定");
  } catch (e) {
    log(`configure: ${e.message}`, true);
  }
};
$("#bJobs").onclick = refreshJobs;
$("#bAddInput").onclick = () => $("#newJobInputs").appendChild(inputRow());
$("#bRun").onclick = runJob;
$("#bCancel").onclick = async () => {
  if (state.runningJob == null) return;
  try {
    await call("job.cancel", { job: state.runningJob });
    log("cancel requested");
  } catch (e) {
    log(`cancel: ${e.message}`, true);
  }
};
$("#bOpenJobDs").onclick = () => {
  const li = $("#jobList .sel");
  const text = li ? li.textContent : prompt("job_id を入力");
  const jobId = (text || "").trim().split(" ")[0];
  if (jobId) openDataset({ job_id: jobId });
};
$("#tlInput").onchange = () => {
  state.view.input = $("#tlInput").value;
  redraw();
};
$("#tlChannel").onchange = () => {
  state.view.channel = Number($("#tlChannel").value);
  redraw();
};
$("#bZoomIn").onclick = () => {
  const v = state.view,
    c = (v.start_s + v.end_s) / 2,
    s = (v.end_s - v.start_s) / 4;
  v.start_s = c - s;
  v.end_s = c + s;
  redraw();
};
$("#bZoomOut").onclick = () => {
  const v = state.view,
    c = (v.start_s + v.end_s) / 2,
    s = v.end_s - v.start_s;
  const dur = Math.max(1, ...(state.timeline?.inputs.map((i) => i.duration_s) || [1]));
  v.start_s = Math.max(0, c - s);
  v.end_s = Math.min(dur, c + s);
  redraw();
};

// スクラブ: waveform canvas クリックで viewport センタリング、
// ドラッグで範囲選択。
(() => {
  const c = $("#timeline");
  let drag = null;
  const toS = (x) =>
    state.view.start_s + (x / c.clientWidth) * (state.view.end_s - state.view.start_s);
  c.addEventListener("mousedown", (e) => {
    drag = toS(e.offsetX);
  });
  addEventListener("mouseup", (e) => {
    if (drag == null) return;
    const t = toS(e.offsetX);
    state.sel = { start_s: Math.min(drag, t), end_s: Math.max(drag, t) };
    drag = null;
    drawTimeline();
  });
  c.addEventListener("click", (e) => {
    const t = toS(e.offsetX);
    const v = state.view,
      s = (v.end_s - v.start_s) / 2;
    v.start_s = Math.max(0, t - s);
    v.end_s = t + s;
    redraw();
  });
})();

$("#newJobInputs").appendChild(inputRow());
// job list の選択状態をクリックで付ける
$("#jobList").addEventListener("click", (e) => {
  if (e.target.tagName !== "LI") return;
  for (const li of $("#jobList").children) li.classList.remove("sel");
  e.target.classList.add("sel");
});

(async () => {
  try {
    const info = await call("al.app.info");
    $("#status").textContent =
      `${info.name} ${info.version}${info.base_dir ? ` — ${info.base_dir}` : ""}`;
    if (info.base_dir) {
      $("#baseDir").value = info.base_dir;
      refreshJobs();
    }
  } catch (e) {
    log(`app.info: ${e.message}`, true);
  }
})();

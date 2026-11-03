/**
 * Dataset → ruviform viz source adapter 検証(#132)。
 *
 * - `DatasetWaveformSource` は `al.view.waveform` の範囲 query +
 *   `pull` 転送で RVBW を復号し、`WaveformDataSource` 契約
 *   (bounded buckets / revision / NaN gap / channel 選択)を満たす
 * - `DatasetSpectrogramSource` は `al.view.spectrogram` の tile query
 *   で RVST-f32 を復号し、frame/Hz 軸写像を持つ tile を返す
 * - stale revision / cancel / over-budget は偽装せず表面化する
 * - 全量ロードは行わない(query params は viewport 範囲のみ、
 *   pull は宣言 bytes 以内)
 */

import assert from "node:assert/strict";
import { describe, it } from "node:test";
import { AlGuiGateway } from "../src/gateway.ts";
import {
  DatasetSpectrogramSource,
  DatasetWaveformSource,
  VizQueryFailure,
} from "../src/viz/sources.ts";
import { FakeAlService, type FakeDataset } from "./fakes.ts";
import { tick } from "./helpers.ts";

const PRECISE = 0.12345678901234568;

const setup = async (dsArgs: Partial<FakeDataset> = {}) => {
  const svc = new FakeAlService();
  svc.addDataset({
    inputs: [{ input_id: "in-01", role: "capture_bug" }],
    durationS: 100,
    sampleRate: 48000,
    channels: 2,
    ...dsArgs,
    root: "ds-1",
  });
  const gateway = new AlGuiGateway(svc);
  const opened = await gateway.openDataset({ root: "ds-1" });
  const pull = (s: string, o: number, l: number) => svc.pull(s, o, l);
  return { svc, gateway, opened, pull };
};

const waveSrc = (
  ctx: Awaited<ReturnType<typeof setup>>,
  extra: Partial<ConstructorParameters<typeof DatasetWaveformSource>[0]> = {},
) =>
  new DatasetWaveformSource({
    gateway: ctx.gateway,
    pull: ctx.pull,
    session: ctx.opened.session,
    inputId: "in-01",
    sampleRate: 48000,
    frames: 48000 * 100,
    channels: 2,
    ...extra,
  });

describe("DatasetWaveformSource", () => {
  it("meta() は秒軸・extent・channels・revision を返す", async () => {
    const ctx = await setup();
    const src = waveSrc(ctx);
    const meta = src.meta();
    assert.equal(meta.axis, "seconds");
    assert.equal(meta.extent, 100);
    assert.equal(meta.channels, 2);
    assert.equal(meta.sampleRate, 48000);
    assert.equal(meta.revision, ctx.opened.revision);
  });

  it("bounded query が RVBW を復号し f64 値・NaN gap を保持する", async () => {
    const ctx = await setup({
      waveformFn: (i) => (i === 5 ? Number.NaN : PRECISE + i),
    });
    const src = waveSrc(ctx);
    const view = await src.query({ start: 10, end: 20, buckets: 8 });
    if (view.kind !== "bins") assert.fail("expected bins view");
    const call = ctx.svc.calls.find((c) => c.op === "al.view.waveform");
    assert.deepEqual([call?.params.start_s, call?.params.end_s, call?.params.bins], [10, 20, 8]);
    assert.equal(view.revision, ctx.opened.revision);
    // covered range = [10*48000, 20*48000) → 1.25s/bucket
    assert.equal(view.start, 10);
    assert.equal(view.unitsPerBucket, 1.25);
    const bins = view.channels[0];
    assert.ok(bins !== undefined);
    assert.equal(bins.length, 8);
    assert.equal(bins[0]?.min, PRECISE);
    // bin 5 = NaN gap — 補間も 0 埋めもしない
    assert.ok(Number.isNaN(bins[5]?.min));
    assert.ok(Number.isNaN(bins[5]?.max));
    assert.ok(Number.isNaN(bins[5]?.rms));
    assert.equal(bins[5]?.count, 0);
    // 隣接 bucket は健全値を保持
    assert.equal(bins[4]?.min, PRECISE + 4);
    assert.equal(bins[6]?.min, PRECISE + 6);
  });

  it("channels 選択は per-channel の bounded op に分解される", async () => {
    const ctx = await setup({ waveformFn: (_i, ch) => (ch === 0 ? 1 : -1) });
    const src = waveSrc(ctx);
    const view = await src.query({ start: 0, end: 10, buckets: 4, channels: [1, 0] });
    if (view.kind !== "bins") assert.fail("expected bins view");
    assert.equal(view.channels.length, 2);
    const calls = ctx.svc.calls.filter((c) => c.op === "al.view.waveform");
    assert.deepEqual(
      calls.map((c) => c.params.channel),
      [1, 0],
    );
    assert.equal(view.channels[0]?.[0]?.min, -1); // ch1 が先
    assert.equal(view.channels[1]?.[0]?.min, 1);
  });

  it("pull は宣言 bytes 以内のみ読む(全量ロードなし)", async () => {
    const ctx = await setup();
    const pulls: { source: string; offset: number; limit: number }[] = [];
    const pull = async (s: string, o: number, l: number) => {
      pulls.push({ source: s, offset: o, limit: l });
      return ctx.svc.pull(s, o, l);
    };
    const src = waveSrc({ ...ctx, pull });
    const view = await src.query({ start: 0, end: 10, buckets: 8 });
    if (view.kind !== "bins") assert.fail("expected bins view");
    assert.equal(view.channels[0]?.length, 8);
    // payload = 16 + 48*8 = 400B — 各 pull は宣言 bytes 範囲内のみ
    // 要求する(offset/limit 契約。channel 毎に独立した publish
    // なので集計は per-source)
    assert.ok(pulls.every((p) => p.offset >= 0 && p.offset + p.limit <= 400));
    // pull 対象は publish 名(`al/waveform/<seq>`)
    assert.ok(pulls.every((p) => p.source.startsWith("al/waveform/")));
    // dataset 全量読み op は一切呼ばれない
    assert.equal(ctx.svc.calls.filter((c) => c.op === "al.ds.read").length, 0);
  });

  it("publish 競合は stale として表面化する", async () => {
    const ctx = await setup();
    // query を保留にし、in-flight 中に dataset を進める
    let release!: (v: unknown) => void;
    const park = new Promise((r) => (release = r));
    const orig = ctx.svc.call.bind(ctx.svc);
    ctx.svc.call = (op: string, params?: Record<string, unknown>) =>
      op === "al.view.waveform" ? (park as Promise<unknown>) : orig(op, params);
    const src = waveSrc(ctx);
    const p = src.query({ start: 0, end: 10, buckets: 4 });
    ctx.svc.advanceDataset("ds-1");
    await ctx.gateway.refreshRevision("ds-1");
    release({ source: "al/waveform/x", bytes: 0, meta: {} });
    await assert.rejects(p, (e) => e instanceof VizQueryFailure && e.kind === "stale");
  });

  it("abort は cancel として伝播し cancelQuery も発火する", async () => {
    const ctx = await setup();
    let release!: (v: unknown) => void;
    const park = new Promise((r) => (release = r));
    const orig = ctx.svc.call.bind(ctx.svc);
    ctx.svc.call = (op: string, params?: Record<string, unknown>) =>
      op === "al.view.waveform" ? (park as Promise<unknown>) : orig(op, params);
    const src = waveSrc(ctx);
    const ctl = new AbortController();
    const p = src.query({ start: 0, end: 10, buckets: 4 }, ctl.signal);
    await tick();
    ctl.abort();
    await assert.rejects(p, (e: unknown) => (e as { name: string }).name === "AbortError");
    assert.ok(ctx.svc.cancelledQueries.length > 0);
    release({ source: "x", bytes: 0, meta: {} });
  });

  it("over-budget reply は over-budget として表面化する", async () => {
    const ctx = await setup();
    const src = waveSrc(ctx, { maxBytes: 64 });
    await assert.rejects(
      src.query({ start: 0, end: 10, buckets: 8 }),
      (e) => e instanceof VizQueryFailure && e.kind === "over-budget",
    );
  });

  it("4ch 超えの fan-out は自己 evict せず全 channel を返す", async () => {
    // QueryBinding の既定 maxInflight=4 を超える channel 数でも、
    // binding を channel 数に合わせて広げるため先頭 request が
    // cancelled で落ちない(5.1/7.1 capture 対応)。
    const ctx = await setup({ channels: 6 });
    const src = waveSrc(ctx, { channels: 6 });
    const view = await src.query({ start: 0, end: 10, buckets: 4 });
    if (view.kind !== "bins") assert.fail("expected bins view");
    assert.equal(view.channels.length, 6);
    const calls = ctx.svc.calls.filter((c) => c.op === "al.view.waveform");
    assert.deepEqual(
      calls.map((c) => c.params.channel),
      [0, 1, 2, 3, 4, 5],
    );
  });

  it("channels: [] は全 channel 指定と同じ(vendored 規約)", async () => {
    const ctx = await setup();
    const src = waveSrc(ctx);
    const view = await src.query({ start: 0, end: 10, buckets: 4, channels: [] });
    if (view.kind !== "bins") assert.fail("expected bins view");
    assert.equal(view.channels.length, 2);
  });

  it("範囲外 channel は unsupported で表面化する(黙って捨てない)", async () => {
    const ctx = await setup();
    const src = waveSrc(ctx);
    await assert.rejects(
      src.query({ start: 0, end: 10, buckets: 4, channels: [0, 99] }),
      (e) => e instanceof VizQueryFailure && e.kind === "unsupported",
    );
  });

  it("反転/非有限 viewport は error を返す(service の silent clamp を防ぐ)", async () => {
    const ctx = await setup();
    const src = waveSrc(ctx);
    await assert.rejects(
      src.query({ start: 10, end: 5, buckets: 4 }),
      (e) => e instanceof VizQueryFailure && e.kind === "error",
    );
    await assert.rejects(
      src.query({ start: 0, end: Number.NaN, buckets: 4 }),
      (e) => e instanceof VizQueryFailure && e.kind === "error",
    );
    // 反転 query は op に到達しない
    assert.equal(ctx.svc.calls.filter((c) => c.op === "al.view.waveform").length, 0);
  });

  it("pullChunkBytes <= 0 は ctor で落とす(無限ループ防止)", async () => {
    const ctx = await setup();
    assert.throws(() => waveSrc(ctx, { pullChunkBytes: 0 }), /pullChunkBytes/);
    assert.throws(() => waveSrc(ctx, { pullChunkBytes: -8 }), /pullChunkBytes/);
  });
});

describe("DatasetSpectrogramSource", () => {
  it("tile query が RVST-f32 を復号し rows/cols/frame 範囲を保持する", async () => {
    const ctx = await setup({
      spectrogramFn: (r, c) => (r === 2 && c === 3 ? Number.NaN : r * 100 + c),
    });
    const src = new DatasetSpectrogramSource({
      gateway: ctx.gateway,
      pull: ctx.pull,
      session: ctx.opened.session,
      inputId: "in-01",
      channel: 1,
    });
    const tile = await src.query({ start: 0, end: 4, cols: 8, rows: 16 });
    const call = ctx.svc.calls.find((c) => c.op === "al.view.spectrogram");
    assert.deepEqual([call?.params.channel, call?.params.rows, call?.params.cols], [1, 16, 8]);
    assert.equal(tile.revision, ctx.opened.revision);
    assert.equal(tile.dtype, "f32");
    assert.equal(tile.rows, 16);
    assert.equal(tile.cols, 8);
    assert.equal(tile.values.length, 128);
    // frame 換算: start=0s→frame 0、end=4s→floor(4*48000/1024)=187
    assert.equal(tile.frameStart, 0);
    assert.equal(tile.frameEnd, 187);
    // row-major: (r=1,c=4) → 104 (f32 rounding)
    assert.equal(tile.values[1 * 8 + 4], Math.fround(104));
    // NaN tile value は保持される
    assert.ok(Number.isNaN(tile.values[2 * 8 + 3]));
    // 軸写像 — frame f は f*hop 始点。cols=8 の tile 末端は
    // 最終 col(index 7)の end = frameEnd*hop/rate
    assert.equal(tile.colStartSeconds(0), 0);
    assert.ok(Math.abs(tile.colEndSeconds(7) - (187 * 1024) / 48000) < 1e-9);
    assert.equal(tile.rowStartHz(0), 0);
    // rows=16 の tile 上端 = 最終 row(index 15)の end = rate
    assert.equal(tile.rowEndHz(15), 48000);
    assert.equal(tile.hop, 1024);
    assert.equal(tile.window, 2048);
    assert.equal(tile.bins, 1025);
  });

  it("abort は cancel として伝播する", async () => {
    const ctx = await setup();
    let release!: (v: unknown) => void;
    const park = new Promise((r) => (release = r));
    const orig = ctx.svc.call.bind(ctx.svc);
    ctx.svc.call = (op: string, params?: Record<string, unknown>) =>
      op === "al.view.spectrogram" ? (park as Promise<unknown>) : orig(op, params);
    const src = new DatasetSpectrogramSource({
      gateway: ctx.gateway,
      pull: ctx.pull,
      session: ctx.opened.session,
      inputId: "in-01",
      channel: 0,
    });
    const ctl = new AbortController();
    const p = src.query({ start: 0, end: 4, cols: 8, rows: 16 }, ctl.signal);
    await tick();
    ctl.abort();
    await assert.rejects(p, (e: unknown) => (e as { name: string }).name === "AbortError");
    release({ source: "x", bytes: 0, meta: {} });
  });

  it("非整数 cols・反転 viewport は error を返す", async () => {
    const ctx = await setup();
    const src = new DatasetSpectrogramSource({
      gateway: ctx.gateway,
      pull: ctx.pull,
      session: ctx.opened.session,
      inputId: "in-01",
      channel: 0,
    });
    await assert.rejects(
      src.query({ start: 0, end: 4, cols: 2.5, rows: 16 }),
      (e) => e instanceof VizQueryFailure && e.kind === "error",
    );
    await assert.rejects(
      src.query({ start: 4, end: 0, cols: 8, rows: 16 }),
      (e) => e instanceof VizQueryFailure && e.kind === "error",
    );
  });
});

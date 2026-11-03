/**
 * Query 検証(#131)— 明示 Source/Run/Range が bounded read に届く。
 *
 * - `al.view.range` の確定値が QueryBinding.request の start/end に
 *   そのまま乗り、sourceId(session/inputId/channel)と sourceRevision
 *   が op params として `al.view.waveform` 等へ正確に伝達される
 * - stale revision・missing source・over-budget・cancel が
 *   honest QueryError として返る(完了を偽装しない)
 * - `revisionProbe` が issue→resolve 間の publish を stale 検出する
 */

import assert from "node:assert/strict";
import { describe, it } from "node:test";
import { AlGuiGateway } from "../src/gateway.ts";
import { mountSeekRange } from "../src/screens/seek-range.ts";
import { mount } from "./env.ts";
import { FakeAlService, flushSync } from "./fakes.ts";
import { part, ptr, stubRect, tick } from "./helpers.ts";

const setup = async () => {
  const svc = new FakeAlService();
  svc.addDataset({ root: "ds-1", inputs: [{ input_id: "in-01", role: "capture_bug" }] });
  const gateway = new AlGuiGateway(svc);
  const opened = await gateway.openDataset({ root: "ds-1" });
  const host = mount("div");
  const fx = mountSeekRange(host, { gateway, durationS: 100, flushWith: flushSync });
  return { svc, gateway, opened, fx, host };
};

const driveRange = (fx: ReturnType<typeof mountSeekRange>, lo: number, hi: number) => {
  const track = part(fx.selector, "track");
  stubRect(track, { left: 0, width: 100 });
  const loThumb = part(fx.selector, "thumb-lo");
  const hiThumb = part(fx.selector, "thumb-hi");
  // track 全幅 = durationS(100)
  loThumb?.dispatchEvent(ptr("pointerdown", { clientX: 0 }));
  loThumb?.dispatchEvent(ptr("pointermove", { clientX: lo }));
  loThumb?.dispatchEvent(ptr("pointerup", { clientX: lo }));
  hiThumb?.dispatchEvent(ptr("pointerdown", { clientX: 100 }));
  hiThumb?.dispatchEvent(ptr("pointermove", { clientX: hi }));
  hiThumb?.dispatchEvent(ptr("pointerup", { clientX: hi }));
};

describe("explicit Source/Run/Range → bounded query", () => {
  it("commit した range が waveform query の start_s/end_s に乗る", async () => {
    const { svc, opened, fx, host } = await setup();
    driveRange(fx, 20, 70);
    const out = await fx.issueRangeQuery(`waveform/${opened.session}/in-01/0`, opened.revision, {
      buckets: 512,
    });
    assert.equal(out.ok, true);
    const call = svc.calls.find((c) => c.op === "al.view.waveform");
    assert.ok(call !== undefined);
    const p = call.params;
    assert.deepEqual({ s: p.start_s, e: p.end_s }, { s: 20, e: 70 });
    assert.equal(p.session, opened.session);
    assert.equal(p.input_id, "in-01");
    assert.equal(p.channel, 0);
    assert.equal(p.bins, 512);
    fx.unmount();
    host.remove();
  });

  it("range 未操作なら full range が乗る", async () => {
    const { svc, opened, fx, host } = await setup();
    const out = await fx.issueRangeQuery(`waveform/${opened.session}/in-01/0`, opened.revision, {});
    assert.equal(out.ok, true);
    const call = svc.calls.find((c) => c.op === "al.view.waveform");
    assert.deepEqual({ s: call?.params.start_s, e: call?.params.end_s }, { s: 0, e: 100 });
    fx.unmount();
    host.remove();
  });

  it("spectrogram/events/features も同じ source grammar で届く", async () => {
    const { svc, opened, fx, host } = await setup();
    await fx.query.request({
      sourceId: `spectrogram/${opened.session}/in-01/1`,
      sourceRevision: opened.revision,
      start: 1,
      end: 2,
    });
    const spec = svc.calls.find((c) => c.op === "al.view.spectrogram");
    assert.equal(spec?.params.channel, 1);
    assert.equal(spec?.params.input_id, "in-01");
    await fx.query.request({
      sourceId: `events/${opened.session}`,
      sourceRevision: opened.revision,
      start: 3,
      end: 4,
    });
    const ev = svc.calls.find((c) => c.op === "al.ds.events");
    assert.deepEqual({ s: ev?.params.start_s, e: ev?.params.end_s }, { s: 3, e: 4 });
    await fx.query.request({
      sourceId: `features/${opened.session}/features/cheap`,
      sourceRevision: opened.revision,
      start: 0,
      end: 100,
    });
    const feats = svc.calls.find((c) => c.op === "al.ds.features");
    assert.deepEqual(feats?.params.roles, ["features/cheap"]);
    fx.unmount();
    host.remove();
  });
});

describe("既存 Dataset の表示は再解析しない", () => {
  it("open + view query 系は al.job.run/resume を一切呼ばない", async () => {
    const { svc, opened, fx, host } = await setup();
    // 表示用途の全 op を叩く
    await fx.issueRangeQuery(`waveform/${opened.session}/in-01/0`, opened.revision);
    await fx.query.request({
      sourceId: `spectrogram/${opened.session}/in-01/0`,
      sourceRevision: opened.revision,
      start: 0,
      end: 10,
    });
    await fx.query.request({
      sourceId: `events/${opened.session}`,
      sourceRevision: opened.revision,
      start: 0,
      end: 10,
    });
    await fx.query.request({
      sourceId: `overview/${opened.session}`,
      sourceRevision: opened.revision,
      start: 0,
      end: 10,
    });
    const jobOps = svc.calls.filter(
      (c) => c.op === "al.job.run" || c.op === "al.job.resume" || c.op === "al.job.cancel",
    );
    assert.equal(jobOps.length, 0);
    // 呼ばれたのは ds/view 系のみ
    const ops = new Set(svc.calls.map((c) => c.op));
    for (const op of ops) {
      assert.ok(
        op.startsWith("al.ds.") || op.startsWith("al.view.") || op === "al.app.info",
        `unexpected op: ${op}`,
      );
    }
    fx.unmount();
    host.remove();
  });
});

describe("honest query states", () => {
  it("stale revision — dataset 進展後の旧 revision は stale", async () => {
    const { svc, gateway, opened, fx, host } = await setup();
    svc.advanceDataset("ds-1");
    // dataset が進展したことに view が追いつく前の旧 revision で query
    await gateway.refreshRevision("ds-1");
    const out = await fx.issueRangeQuery(`waveform/${opened.session}/in-01/0`, opened.revision);
    assert.equal(out.ok, false);
    if (!out.ok) assert.equal(out.error.kind, "stale");
    fx.unmount();
    host.remove();
  });

  it("issue→resolve 間の publish も revisionProbe で stale になる", async () => {
    const svc = new FakeAlService();
    svc.addDataset({ root: "ds-1" });
    const gateway = new AlGuiGateway(svc);
    const opened = await gateway.openDataset({ root: "ds-1" });
    // query を保留にし、in-flight 中に dataset を進める
    let release!: (v: unknown) => void;
    const park = new Promise((r) => (release = r));
    const orig = svc.call.bind(svc);
    svc.call = (op: string, params?: Record<string, unknown>) =>
      op === "al.view.waveform" ? (park as Promise<unknown>) : orig(op, params);
    const host = mount("div");
    const fx = mountSeekRange(host, { gateway, durationS: 100, flushWith: flushSync });
    const p = fx.query.request({
      sourceId: `waveform/${opened.session}/in-01/0`,
      sourceRevision: opened.revision,
      start: 0,
      end: 10,
    });
    svc.advanceDataset("ds-1");
    await gateway.refreshRevision("ds-1");
    release({ source: "al/waveform/1", bytes: 8, meta: {} });
    const out = await p;
    // resolve 時に probe が新 revision を返す → stale
    assert.equal(out.ok, false);
    if (!out.ok) assert.equal(out.error.kind, "stale");
    fx.unmount();
    host.remove();
  });

  it("missing source(session 不在)は error で返り成功を装わない", async () => {
    const { fx, host } = await setup();
    const out = await fx.query.request({
      sourceId: "waveform/999/in-01/0",
      sourceRevision: "fp-x",
      start: 0,
      end: 10,
    });
    assert.equal(out.ok, false);
    if (!out.ok) assert.equal(out.error.kind, "error");
    fx.unmount();
    host.remove();
  });

  it("malformed sourceId は error", async () => {
    const { fx, host } = await setup();
    const out = await fx.query.request({
      sourceId: "garbage",
      sourceRevision: "fp-x",
      start: 0,
      end: 10,
    });
    assert.equal(out.ok, false);
    if (!out.ok) assert.equal(out.error.kind, "error");
    fx.unmount();
    host.remove();
  });

  it("over-budget reply は over-budget(データを偽装しない)", async () => {
    const { opened, fx, host } = await setup();
    const out = await fx.query.request({
      sourceId: `waveform/${opened.session}/in-01/0`,
      sourceRevision: opened.revision,
      start: 0,
      end: 10,
      buckets: 1024, // → bytes = 8192
      maxBytes: 4096,
    });
    assert.equal(out.ok, false);
    if (!out.ok) assert.equal(out.error.kind, "over-budget");
    fx.unmount();
    host.remove();
  });

  it("detach は in-flight query を cancel する(cancelled outcome)", async () => {
    const svc = new FakeAlService();
    svc.addDataset({ root: "ds-1" });
    const gateway = new AlGuiGateway(svc);
    const opened = await gateway.openDataset({ root: "ds-1" });
    // call を保留にして in-flight を作る
    let release!: (v: unknown) => void;
    const park = new Promise((r) => (release = r));
    const orig = svc.call.bind(svc);
    svc.call = (op: string, params?: Record<string, unknown>) =>
      op === "al.view.waveform" ? (park as Promise<unknown>) : orig(op, params);
    const host = mount("div");
    const fx = mountSeekRange(host, { gateway, durationS: 100, flushWith: flushSync });
    const p = fx.query.request({
      sourceId: `waveform/${opened.session}/in-01/0`,
      sourceRevision: opened.revision,
      start: 0,
      end: 10,
    });
    await tick();
    fx.link.detach("unmount");
    release({ source: "x", bytes: 1, meta: {} });
    const out = await p;
    assert.equal(out.ok, false);
    if (!out.ok) assert.equal(out.error.kind, "cancelled");
    fx.unmount();
    host.remove();
  });
});

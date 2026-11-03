/**
 * f64 精度と display rounding の一方向性検証(#131)。
 *
 * - query 応答に含まれる f64 配列が gateway/binding を通っても
 *   値として劣化しない(display 用の丸めが source に逆流しない)
 * - commit された値は丸められず plain f64 のまま store に入る
 * - element の display formatter が出力する文字列が persisted
 *   value にフィードバックしない(controlled write 抑止)
 */

import assert from "node:assert/strict";
import { describe, it } from "node:test";
import { AlGuiGateway } from "../src/gateway.ts";
import { mountMediaPreview } from "../src/screens/media-preview.ts";
import { mountSeekRange } from "../src/screens/seek-range.ts";
import { mount } from "./env.ts";
import { FakeAlService, flushSync } from "./fakes.ts";
import { part, ptr, stubRect, tick } from "./helpers.ts";

const PRECISE = 0.010000000000000002; // display "0.010" と区別できる f64

describe("f64 精度保持", () => {
  it("element commit の plain f64 がそのまま確定値になる(step 無し target)", async () => {
    const svc = new FakeAlService();
    const gateway = new AlGuiGateway(svc);
    const host = mount("div");
    const fx = mountMediaPreview(host, { gateway, durationS: 100, flushWith: flushSync });
    // seekbar の descriptor は step を持たない — plain は丸められず
    // intent.value.plain としてそのまま gateway に届く。
    (fx.seekbar as unknown as { dispatchInput(i: unknown): void }).dispatchInput({
      type: "set",
      value: { plain: 33.333333333 },
      source: "keyboard",
    });
    await tick();
    assert.equal(gateway.targetValue("al.view.seek"), 33.333333333);
    // 表示は "33.333 s"(toFixed(3))だが、その丸め文字列は値に戻らない
    assert.notEqual(gateway.targetValue("al.view.seek"), 33.333);
    fx.unmount();
    host.remove();
  });

  it("commit intent の f64 が丸められず config draft → RunSpec に入る", async () => {
    const svc = new FakeAlService();
    const gateway = new AlGuiGateway(svc);
    // cfg target への commit — descriptor step の snap は control 側の
    // 責務であり、gateway は intent.value.plain を verbatim に保持する。
    gateway.dispatchFor(
      {
        kind: "commit",
        target: "al.cfg.analysis.cheap.level_window_s",
        controlId: "n1",
        sequence: 1,
        gestureId: "g1",
        value: { plain: PRECISE, normalized: PRECISE },
      },
      { commandId: "cmd-1", generation: 1, issuedAt: 0, expectReceipt: true },
      () => {},
    );
    const spec = gateway.buildRunSpec();
    const cfg = spec.config as unknown as { analysis: { cheap: { level_window_s: number } } };
    assert.equal(cfg.analysis.cheap.level_window_s, PRECISE);
    assert.notEqual(cfg.analysis.cheap.level_window_s, 0.01);
  });

  it("waveform query の f64 bins がそのまま返る", async () => {
    const svc = new FakeAlService();
    // biome-ignore lint/correctness/noPrecisionLoss: f64 表現境界の検査用リテラル
    const preciseBins = [1.2345678901234567e-9, Math.PI, Number.MAX_SAFE_INTEGER + 0.5];
    svc.addDataset({ root: "ds-1", inputs: [{ input_id: "in-01", role: "capture_bug" }] });
    const gateway = new AlGuiGateway(svc);
    const opened = await gateway.openDataset({ root: "ds-1" });
    const orig = svc.call.bind(svc);
    svc.call = (op, params) =>
      op === "al.view.waveform"
        ? Promise.resolve({ source: "s", bytes: 24, meta: { bins: preciseBins } })
        : orig(op, params ?? {});
    const host = mount("div");
    const fx = mountSeekRange(host, { gateway, durationS: 100, flushWith: flushSync });
    const out = await fx.query.request({
      sourceId: `waveform/${opened.session}/in-01/0`,
      sourceRevision: opened.revision,
      start: 0,
      end: 10,
    });
    assert.equal(out.ok, true);
    if (out.ok) {
      assert.deepEqual((out.reply.payload as { meta: { bins: number[] } }).meta.bins, preciseBins);
    }
    fx.unmount();
    host.remove();
  });

  it("dataset rows の f64 値が変換なしで返る(provenance 保持)", async () => {
    const svc = new FakeAlService();
    svc.addDataset({
      root: "ds-1",
      featureRows: [
        {
          frame: 0,
          rms: 0.12345678901234568,
          source: "features/cheap",
          provenance: { run: "r-1" },
        },
      ],
    });
    const gateway = new AlGuiGateway(svc);
    const opened = await gateway.openDataset({ root: "ds-1" });
    const host = mount("div");
    const fx = mountSeekRange(host, { gateway, durationS: 100, flushWith: flushSync });
    const out = await fx.query.request({
      sourceId: `read/${opened.session}/features/cheap/level_table`,
      sourceRevision: opened.revision,
      start: 0,
      end: 10,
    });
    assert.equal(out.ok, true);
    if (out.ok) {
      const rows = (out.reply.payload as { rows: { rms: number }[] }).rows;
      assert.equal(rows.at(0)?.rms, 0.12345678901234568);
    }
    fx.unmount();
    host.remove();
  });
});

describe("display rounding は一方向", () => {
  it("formatter の出力文字列が store に逆流しない", async () => {
    const svc = new FakeAlService();
    const gateway = new AlGuiGateway(svc);
    const host = mount("div");
    const fx = mountMediaPreview(host, { gateway, durationS: 100, flushWith: flushSync });
    const track = part(fx.seekbar, "track");
    stubRect(track, { left: 0, width: 100 });
    track?.dispatchEvent(ptr("pointerdown", { clientX: 33.333333 }));
    track?.dispatchEvent(ptr("pointerup", { clientX: 33.333333 }));
    await tick();
    const committed = gateway.targetValue("al.view.seek") as number;
    // display "33.33" に丸められず f64 のまま
    assert.notEqual(committed, 33.33);
    assert.ok(Math.abs(committed - 33.333333) < 1e-9);
    // 以降の外部 write も丸め文字列ではなく数値
    gateway.applyExternal("al.view.seek", 42.5);
    assert.equal(fx.seekbar.value, 42.5);
    fx.unmount();
    host.remove();
  });

  it("external write(programmatic)は gesture event を再発火しない(ループ抑止)", async () => {
    const svc = new FakeAlService();
    const gateway = new AlGuiGateway(svc);
    const host = mount("div");
    const fx = mountMediaPreview(host, { gateway, durationS: 100, flushWith: flushSync });
    const before = gateway.receiptLog.length;
    // 外部 truth push → element の controlled write → 新 gesture が出ない
    gateway.applyExternal("al.view.seek", 55);
    await tick();
    assert.equal(gateway.receiptLog.length, before);
    assert.equal(gateway.targetValue("al.view.seek"), 55);
    fx.unmount();
    host.remove();
  });
});

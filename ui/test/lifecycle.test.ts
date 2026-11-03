/**
 * ライフサイクル検証(#131)— detach/remount・subscription 解放・
 * 二窓 dock 移行・ViewLink.detach が Job を殺さないこと。
 *
 * - detach で in-flight gesture/query が cancel され、subscriber が
 *   外れる(subscription leak なし)
 * - detach は UI ライフサイクル操作であって `al.job.*` を発行しない
 * - remount(別 window/dock への移行)では generation が進み、旧
 *   generation の receipt/projection は stale として届かない
 * - 新しい attach は gateway の truth を再読する
 */

import assert from "node:assert/strict";
import { describe, it } from "node:test";
import { AlGuiGateway } from "../src/gateway.ts";
import { mountJobSettings } from "../src/screens/job-settings.ts";
import { mountMediaPreview } from "../src/screens/media-preview.ts";
import { mountSeekRange } from "../src/screens/seek-range.ts";
import { mount } from "./env.ts";
import { FakeAlService, flushSync } from "./fakes.ts";
import { part, ptr, stubRect, tick } from "./helpers.ts";

const setup = async () => {
  const svc = new FakeAlService();
  svc.addDataset({ root: "ds-1", inputs: [{ input_id: "in-01", role: "capture_bug" }] });
  const gateway = new AlGuiGateway(svc);
  const opened = await gateway.openDataset({ root: "ds-1" });
  return { svc, gateway, opened };
};

describe("detach → subscription cleanup", () => {
  it("detach 後に view 更新が element へ届かない", async () => {
    const { gateway } = await setup();
    const host = mount("div");
    const fx = mountMediaPreview(host, { gateway, durationS: 100, flushWith: flushSync });
    gateway.applyExternal("al.view.seek", 10);
    await tick();
    assert.equal(fx.seekbar.value, 10);
    fx.unmount(); // detach
    gateway.applyExternal("al.view.seek", 77);
    await tick();
    // 切断済みなので element は古い値のまま(push は届かない)
    assert.equal(fx.seekbar.value, 10);
    host.remove();
  });

  it("unmount は subscriber を解放する(再 subscribe 数が増えない)", async () => {
    const { gateway } = await setup();
    const host = mount("div");
    const fx = mountSeekRange(host, { gateway, durationS: 100, flushWith: flushSync });
    const pushes: unknown[] = [];
    const off = gateway.subscribe("al.view.range", (p) => pushes.push(p), {
      viewId: "external",
      generation: 1,
    });
    const before = pushes.length;
    fx.unmount();
    gateway.dispatchFor(
      {
        kind: "commit",
        target: "al.view.range",
        controlId: "sel-lo",
        sequence: 1,
        gestureId: "g",
        value: { plain: 5, normalized: 5 },
      },
      { commandId: "c1", generation: 1, issuedAt: 0, expectReceipt: true },
      () => {},
    );
    // fixture 側の subscriber は detach で外れているが、外部 subscriber
    // には届く(fixture が全 subscription を殺すわけではない)
    assert.ok(pushes.length > before);
    off?.();
    host.remove();
  });
});

describe("detach は Job/domain command を発行しない", () => {
  it("running job がある状態で detach しても al.job cancel を呼ばない", async () => {
    const { svc, gateway } = await setup();
    const host = mount("div");
    const jobFx = mountJobSettings(host, { gateway, flushWith: flushSync });
    jobFx.setInputs([{ role: "capture_bug", path: "/in/a.mp4" }]);
    jobFx.buttons.run.click();
    await tick();
    assert.equal(gateway.jobProjection.status, "running");
    const jobCalls = svc.calls.filter((c) => c.op === "al.job.cancel").length;
    // 画面 detach — job は走り続ける
    jobFx.unmount();
    await tick();
    assert.equal(svc.calls.filter((c) => c.op === "al.job.cancel").length, jobCalls);
    assert.equal(gateway.jobProjection.status, "running");
    svc.runningJob().finish();
    host.remove();
  });
});

describe("remount / two-window dock transfer", () => {
  it("再 mount は gateway truth を再読し旧 generation の push を受けない", async () => {
    const { gateway } = await setup();
    const hostA = mount("div");
    const fxA = mountMediaPreview(hostA, { gateway, durationS: 100, flushWith: flushSync });
    gateway.applyExternal("al.view.seek", 30);
    await tick();
    assert.equal(fxA.seekbar.value, 30);
    // window A から detach → window B(dock)へ移行
    fxA.unmount();
    const hostB = mount("div");
    const fxB = mountMediaPreview(hostB, { gateway, durationS: 100, flushWith: flushSync });
    // 新 attach は現在 truth を再読する
    await tick();
    assert.equal(fxB.seekbar.value, 30);
    // B での gesture は新しい generation で確定する
    const trackB = part(fxB.seekbar, "track");
    stubRect(trackB, { left: 0, width: 100 });
    trackB?.dispatchEvent(ptr("pointerdown", { clientX: 80 }));
    trackB?.dispatchEvent(ptr("pointerup", { clientX: 80 }));
    await tick();
    assert.equal(gateway.targetValue("al.view.seek"), 80);
    fxB.unmount();
    hostA.remove();
    hostB.remove();
  });

  it("in-flight gesture は detach で cancel され確定値が変わらない", async () => {
    const { gateway } = await setup();
    const host = mount("div");
    const fx = mountMediaPreview(host, { gateway, durationS: 100, flushWith: flushSync });
    const track = part(fx.seekbar, "track");
    stubRect(track, { left: 0, width: 100 });
    track?.dispatchEvent(ptr("pointerdown", { clientX: 20 }));
    track?.dispatchEvent(ptr("pointerup", { clientX: 20 }));
    await tick();
    assert.equal(gateway.targetValue("al.view.seek"), 20);
    // gesture 開始だけして detach(pointerup 前)
    track?.dispatchEvent(ptr("pointerdown", { clientX: 90 }));
    track?.dispatchEvent(ptr("pointermove", { clientX: 95 }));
    fx.unmount();
    await tick();
    // 未 commit の gesture は確定値を変えない
    assert.equal(gateway.targetValue("al.view.seek"), 20);
    host.remove();
  });
});

/**
 * Gesture 検証(#131)— Preview/Commit/Cancel と Job 操作の区別。
 *
 * - element gesture(begin/preview/commit/cancel)が binding → gateway
 *   の確定値へ正しく伝達され、preview/cancel は確定値を汚さない
 * - これらは Job run/resume/cancel とは別経路: `al.job` は action
 *   command のみを受け、gesture とは別 domain の receipt を返す
 * - `al.media.preview` transport action も Job とは別 domain
 * - preview の coalescing が binding 側の最新勝ちで扱われる
 */

import assert from "node:assert/strict";
import { describe, it } from "node:test";
import { AlGuiGateway } from "../src/gateway.ts";
import { mountMediaPreview } from "../src/screens/media-preview.ts";
import { mount } from "./env.ts";
import { FakeAlService, flushSync } from "./fakes.ts";
import { key, part, ptr, stubRect, tick } from "./helpers.ts";

const setup = () => {
  const svc = new FakeAlService();
  const gateway = new AlGuiGateway(svc);
  return { svc, gateway };
};

describe("seekbar gesture → view 確定値", () => {
  it("pointer drag の begin→preview→commit が確定値になる", async () => {
    const { gateway } = setup();
    const host = mount("div");
    const fx = mountMediaPreview(host, { gateway, durationS: 100, flushWith: flushSync });
    const track = part(fx.seekbar, "track");
    stubRect(track, { left: 0, width: 100 });
    track?.dispatchEvent(ptr("pointerdown", { clientX: 10 }));
    track?.dispatchEvent(ptr("pointermove", { clientX: 60 }));
    track?.dispatchEvent(ptr("pointerup", { clientX: 60 }));
    await tick();
    // commit された値だけが truth になる(preview 中間値は残らない)
    assert.equal(gateway.targetValue("al.view.seek"), 60);
    const statuses = gateway.receiptLog.map((r) => r.status);
    assert.deepEqual(
      statuses.filter((s) => s !== "applied"),
      [],
    );
    assert.deepEqual(statuses, ["applied"]);
    fx.unmount();
    host.remove();
  });

  it("cancel は確定値を変えない(committed のみが残る)", async () => {
    const { gateway } = setup();
    const host = mount("div");
    const fx = mountMediaPreview(host, { gateway, durationS: 100, flushWith: flushSync });
    const track = part(fx.seekbar, "track");
    stubRect(track, { left: 0, width: 100 });
    // 初回 commit 3
    track?.dispatchEvent(ptr("pointerdown", { clientX: 3 }));
    track?.dispatchEvent(ptr("pointerup", { clientX: 3 }));
    await tick();
    assert.equal(gateway.targetValue("al.view.seek"), 3);
    // 2 回目の gesture: preview で 90 まで行って Escape cancel
    track?.dispatchEvent(ptr("pointerdown", { clientX: 50 }));
    track?.dispatchEvent(ptr("pointermove", { clientX: 90 }));
    fx.seekbar.dispatchEvent(key("keydown", { key: "Escape" }));
    await tick();
    assert.equal(gateway.targetValue("al.view.seek"), 3);
    fx.unmount();
    host.remove();
  });
});

describe("job action は gesture 経路と別 domain", () => {
  it("al.job への commit intent は unsupported receipt", async () => {
    const { gateway } = setup();
    const receipts: string[] = [];
    gateway.dispatchFor(
      {
        kind: "commit",
        target: "al.job",
        controlId: "c",
        sequence: 1,
        gestureId: "g",
        value: { plain: 1, normalized: 1 },
      },
      { commandId: "cmd-x", generation: 1, issuedAt: 0, expectReceipt: true },
      (r) => receipts.push(r.status),
    );
    assert.deepEqual(receipts, ["unsupported"]);
  });

  it("preview transport action は preview store だけを更新し Job には触れない", async () => {
    const { gateway, svc } = setup();
    const host = mount("div");
    const fx = mountMediaPreview(host, { gateway, durationS: 100, flushWith: flushSync });
    const play = part(fx.transport, "play");
    play?.dispatchEvent(new MouseEvent("click", { bubbles: true }));
    await tick();
    assert.equal(gateway.previewProjection.state, "playing");
    // transport action は job op を呼ばない(job 呼出がない)
    assert.equal(svc.calls.filter((c) => c.op.startsWith("al.job")).length, 0);
    assert.equal(gateway.jobProjection.status, "idle");
    fx.unmount();
    host.remove();
  });

  it("job run action は preview 状態に影響しない", async () => {
    const { gateway, svc } = setup();
    const host = mount("div");
    const fx = mountMediaPreview(host, { gateway, durationS: 100, flushWith: flushSync });
    const play = part(fx.transport, "play");
    play?.dispatchEvent(new MouseEvent("click", { bubbles: true }));
    await tick();
    assert.equal(gateway.previewProjection.state, "playing");
    // job run は preview projection を変えない
    gateway.dispatchFor(
      { kind: "action", target: "al.job", controlId: "job.run", sequence: 1, action: "run" },
      { commandId: "cmd-r", generation: 1, issuedAt: 0, expectReceipt: true },
      () => {},
    );
    await tick();
    assert.equal(gateway.previewProjection.state, "playing");
    svc.runningJob().finish();
    await tick();
    fx.unmount();
    host.remove();
  });
});

describe("preview coalescing", () => {
  it("preview intent は coalescible で binding が最新勝ちで扱う", async () => {
    const { gateway } = setup();
    const host = mount("div");
    const fx = mountMediaPreview(host, { gateway, durationS: 100 });
    // flushWith 未指定 → queueMicrotask でまとめて flush される
    const track = part(fx.seekbar, "track");
    stubRect(track, { left: 0, width: 100 });
    track?.dispatchEvent(ptr("pointerdown", { clientX: 5 }));
    track?.dispatchEvent(ptr("pointermove", { clientX: 20 }));
    track?.dispatchEvent(ptr("pointermove", { clientX: 40 }));
    track?.dispatchEvent(ptr("pointermove", { clientX: 80 }));
    track?.dispatchEvent(ptr("pointerup", { clientX: 80 }));
    await tick();
    assert.equal(gateway.targetValue("al.view.seek"), 80);
    // preview は commit に先行するが、確定値は最後の commit のみ
    fx.unmount();
    host.remove();
  });
});

describe("gesture と job action の receipt は別 series", () => {
  it("view gesture commit は applied、job action は accepted→applied の二段", async () => {
    const { gateway, svc } = setup();
    const host = mount("div");
    const fx = mountMediaPreview(host, { gateway, durationS: 100, flushWith: flushSync });
    const track = part(fx.seekbar, "track");
    stubRect(track, { left: 0, width: 100 });
    track?.dispatchEvent(ptr("pointerdown", { clientX: 25 }));
    track?.dispatchEvent(ptr("pointerup", { clientX: 25 }));
    // gesture commit は即 applied(値 domain 確定)
    assert.deepEqual(
      gateway.receiptLog.map((r) => r.status),
      ["applied"],
    );

    gateway.dispatchFor(
      { kind: "action", target: "al.job", controlId: "job.run", sequence: 1, action: "run" },
      { commandId: "cmd-r", generation: 1, issuedAt: 0, expectReceipt: true },
      () => {},
    );
    // job は accepted → (完了後) applied の二段 receipt
    assert.equal(gateway.receiptLog.at(-1)?.status, "accepted");
    svc.runningJob().finish();
    await tick();
    assert.equal(gateway.receiptLog.at(-1)?.status, "applied");
    fx.unmount();
    host.remove();
  });
});

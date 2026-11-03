/**
 * Honest states 検証(#131)— 失敗・遅延・順不同・不可・cancel を
 * 成功として偽装しないことの確認。
 *
 * - missing source / unsupported op / rejected / failed / cancelled /
 *   unavailable / timeout / delayed / reordered / duplicate の各状態が
 *   binding phase または receipt として正直に表現される
 * - reordered receipt(遅い applied が後から届く等)でも確定値は
 *   最後の authoritative 結果に従う
 * - stale projection(gateway revision より古い)は破棄される
 */

import assert from "node:assert/strict";
import { describe, it } from "node:test";
import { AlGuiGateway } from "../src/gateway.ts";
import { mountJobSettings } from "../src/screens/job-settings.ts";
import { mountSeekRange } from "../src/screens/seek-range.ts";
import {
  type BindingPort,
  CommandBinding,
  type CommandIntent,
  type CommandReceipt,
  type DispatchContext,
} from "../vendor/ruviform/ui/binding/src/index.ts";
import { mount } from "./env.ts";
import { FakeAlService, flushSync, ManualTimers } from "./fakes.ts";
import { tick } from "./helpers.ts";

const ctx = (id: string): DispatchContext => ({
  commandId: id,
  generation: 1,
  issuedAt: 0,
  expectReceipt: true,
});

describe("binding-level honest phases(非 gateway port で検証)", () => {
  // gateway ではなく最小 port で binding 機構の状態遷移を検証する
  const pendingPort = (): BindingPort & { intents: CommandIntent[] } => {
    const intents: CommandIntent[] = [];
    return {
      intents,
      dispatch(intent: CommandIntent, _c: DispatchContext) {
        intents.push(intent); // receipt を返さない → pending のまま
      },
    };
  };

  it("receipt 未到着は applied にならず timeout で明示", () => {
    const timers = new ManualTimers();
    const port = pendingPort();
    const b = new CommandBinding<number>({
      port,
      target: "x",
      flushWith: flushSync,
      schedule: timers.schedule,
      ackTimeoutMs: 100,
    });
    b.push({
      kind: "commit",
      target: "x",
      controlId: "c",
      sequence: 1,
      gestureId: "g",
      value: { plain: 5, normalized: 5 },
    });
    assert.equal(b.state.phase, "pending"); // 確定を偽装しない
    timers.runDue(100);
    assert.equal(b.state.phase, "timeout");
    b.detach();
  });

  it("遅延 receipt は届いた時点で state に反映される(delayed≠lost)", () => {
    let ack: ((r: CommandReceipt) => void) | undefined;
    const port: BindingPort = {
      dispatch(_i: CommandIntent, c: DispatchContext) {
        ack = (r) => b.ack({ ...r, commandId: c.commandId });
      },
    };
    const timers = new ManualTimers();
    const b = new CommandBinding<number>({
      port,
      target: "x",
      flushWith: flushSync,
      schedule: timers.schedule,
      ackTimeoutMs: 1000,
    });
    b.push({
      kind: "commit",
      target: "x",
      controlId: "c",
      sequence: 1,
      gestureId: "g",
      value: { plain: 5, normalized: 5 },
    });
    assert.equal(b.state.phase, "pending");
    // timeout 前では遅いが到着はする — 遅延は lost ではない
    timers.runDue(500);
    assert.equal(b.state.phase, "pending");
    ack?.({ commandId: "", status: "applied" });
    assert.equal(b.state.phase, "applied");
    b.detach();
  });
});

describe("reordered / duplicate / stale projection", () => {
  it("遅れて届いた古い revision の projection は guard で落ちる", () => {
    const gateway = new AlGuiGateway(new FakeAlService());
    gateway.declareTarget("al.view.seek", 0);
    let binding!: CommandBinding<number>;
    binding = new CommandBinding<number>({
      target: "al.view.seek",
      port: {
        dispatch: (i, c) => gateway.dispatchFor(i, c, (r) => binding.ack(r)),
      },
      flushWith: flushSync,
    });
    // subscribe の初回 push は現在 truth(rev0)を即座に届ける
    const off = gateway.subscribe<number>("al.view.seek", (p) => binding.observe(p), {
      viewId: "v1",
      generation: 1,
    });
    assert.equal(binding.state.lastObserved?.value, 0);
    gateway.applyExternal("al.view.seek", 10); // rev1 → observed
    assert.equal(binding.state.lastObserved?.value, 10);
    // rev0 の遅延 projection — 並び替え配信で後から届いても truth を
    // 上書きしない
    binding.observe({ revision: 0, value: 999 });
    assert.equal(binding.state.lastObserved?.value, 10);
    assert.equal(binding.diagnostics.staleProjections, 1);
    off?.();
    binding.detach();
  });

  it("同じ commandId の重複 receipt は binding 側で二度適用されない", () => {
    // 配信層が同じ receipt を二度届けるケース — 実 wire の
    // duplicate は binding.ack が dedupe する責務。
    let binding!: CommandBinding<number>;
    const gateway = new AlGuiGateway(new FakeAlService());
    gateway.declareTarget("al.view.seek", 0);
    binding = new CommandBinding<number>({
      target: "al.view.seek",
      port: {
        dispatch: (i, c) => gateway.dispatchFor(i, c, (r) => binding.ack(r)),
      },
      flushWith: flushSync,
    });
    binding.push({
      kind: "commit",
      target: "al.view.seek",
      controlId: "c",
      sequence: 1,
      gestureId: "g",
      value: { plain: 5, normalized: 5 },
    });
    assert.equal(binding.state.phase, "applied");
    const receipts = gateway.receiptLog;
    const last = receipts.at(-1);
    assert.ok(last !== undefined);
    // 同じ receipt を配送層経由で再度届けても phase が退行しない
    binding.ack({ ...last });
    assert.equal(binding.state.phase, "applied");
    binding.detach();
  });

  it("applied 後に遅れて届いた古い accepted は phase を退行させない", () => {
    let binding!: CommandBinding<number>;
    const gateway = new AlGuiGateway(new FakeAlService());
    gateway.declareTarget("al.view.seek", 0);
    let captured: ((r: CommandReceipt) => void) | undefined;
    binding = new CommandBinding<number>({
      target: "al.view.seek",
      port: {
        dispatch: (i, c) => {
          captured = (r) => binding.ack({ ...r, commandId: c.commandId });
          gateway.dispatchFor(i, c, (r) => {
            if (r.status === "applied") captured?.(r);
            // accepted は意図的に遅延させる(dispatch では届けない)
          });
        },
      },
      flushWith: flushSync,
    });
    binding.push({
      kind: "commit",
      target: "al.view.seek",
      controlId: "c",
      sequence: 1,
      gestureId: "g",
      value: { plain: 5, normalized: 5 },
    });
    assert.equal(binding.state.phase, "applied");
    // 並び替え配信: 後から古い accepted が届いても applied を保持
    captured?.({ commandId: "", status: "accepted" } satisfies CommandReceipt);
    assert.equal(binding.state.phase, "applied");
    binding.detach();
  });
});

describe("unavailable / unsupported / failed", () => {
  it("view target への action は unsupported receipt", () => {
    const gateway = new AlGuiGateway(new FakeAlService());
    const out: CommandReceipt[] = [];
    gateway.dispatchFor(
      { kind: "action", target: "al.view.seek", controlId: "c", sequence: 1, action: "play" },
      ctx("c1"),
      (r) => out.push(r),
    );
    assert.equal(out[0]?.status, "unsupported");
  });

  it("未知 target の commit は rejected receipt", () => {
    const gateway = new AlGuiGateway(new FakeAlService());
    const out: CommandReceipt[] = [];
    gateway.dispatchFor(
      {
        kind: "commit",
        target: "al.nothing",
        controlId: "c",
        sequence: 1,
        gestureId: "g",
        value: { plain: 1, normalized: 1 },
      },
      ctx("c1"),
      (r) => out.push(r),
    );
    assert.equal(out[0]?.status, "rejected");
  });

  it("dataset 未 attach の view query は error(成功を装わない)", async () => {
    const gateway = new AlGuiGateway(new FakeAlService());
    const host = mount("div");
    const fx = mountSeekRange(host, { gateway, durationS: 100, flushWith: flushSync });
    const out = await fx.query.request({
      sourceId: "waveform/1/in-01/0",
      sourceRevision: "fp-x",
      start: 0,
      end: 10,
    });
    assert.equal(out.ok, false);
    if (!out.ok) {
      assert.equal(out.error.kind, "error");
      assert.ok(String(out.error.detail).includes("session"));
    }
    fx.unmount();
    host.remove();
  });

  it("未知 stage の commit は rejected(RunSpec からも外れる)", () => {
    const gateway = new AlGuiGateway(new FakeAlService());
    const out: CommandReceipt[] = [];
    gateway.dispatchFor(
      {
        kind: "commit",
        target: "al.cfg.stages.expensvie_analysis",
        controlId: "c",
        sequence: 1,
        gestureId: "g",
        value: { plain: 1, normalized: 1 },
      },
      ctx("c1"),
      (r) => out.push(r),
    );
    assert.equal(out[0]?.status, "rejected");
    assert.ok(!gateway.draft.stages.has("expensvie_analysis"));
  });

  it("非 finite 値の commit は rejected(NaN/Infinity を truth にしない)", () => {
    const gateway = new AlGuiGateway(new FakeAlService());
    const out: CommandReceipt[] = [];
    gateway.dispatchFor(
      {
        kind: "commit",
        target: "al.view.seek",
        controlId: "c",
        sequence: 1,
        gestureId: "g",
        value: { plain: Number.NaN, normalized: Number.NaN },
      },
      ctx("c1"),
      (r) => out.push(r),
    );
    assert.equal(out[0]?.status, "rejected");
    assert.notEqual(gateway.targetValue("al.view.seek"), Number.NaN);
  });

  it("反転した range commit(low > high)は rejected", () => {
    const gateway = new AlGuiGateway(new FakeAlService());
    gateway.seedRange({ low: 0, high: 40 });
    const out: CommandReceipt[] = [];
    gateway.dispatchFor(
      {
        kind: "commit",
        target: "al.view.range",
        controlId: "sel-lo",
        sequence: 1,
        gestureId: "g",
        value: { plain: 50, normalized: 0.5 },
      },
      ctx("c1"),
      (r) => out.push(r),
    );
    assert.equal(out[0]?.status, "rejected");
    assert.deepEqual(gateway.targetValue("al.view.range"), { low: 0, high: 40 });
  });

  it("subscribe は未宣言 target でも後続 commit の projection を受け取る", () => {
    const gateway = new AlGuiGateway(new FakeAlService());
    const seen: unknown[] = [];
    const off = gateway.subscribe<unknown>("al.view.late", (p) => seen.push(p.value), {
      viewId: "v",
      generation: 1,
    });
    gateway.applyExternal("al.view.late", 7);
    assert.deepEqual(seen, [0, 7]); // 初期 push + commit 後 push
    off?.();
  });

  it("proto 系 config path の commit は rejected(Object.prototype を汚さない)", () => {
    const gateway = new AlGuiGateway(new FakeAlService());
    const out: CommandReceipt[] = [];
    gateway.dispatchFor(
      {
        kind: "commit",
        target: "al.cfg.a.__proto__.polluted",
        controlId: "c",
        sequence: 1,
        gestureId: "g",
        value: { plain: 1, normalized: 1 },
      },
      ctx("c1"),
      (r) => out.push(r),
    );
    assert.equal(out[0]?.status, "rejected");
    assert.equal(({} as Record<string, unknown>).polluted, undefined);
  });

  it("job 失敗の detail が receipt/projection に残る", async () => {
    const svc = new FakeAlService();
    const gateway = new AlGuiGateway(svc);
    const host = mount("div");
    const fx = mountJobSettings(host, { gateway, flushWith: flushSync });
    fx.setInputs([{ role: "capture_bug", path: "/in/a.mp4" }]);
    fx.buttons.run.click();
    await tick();
    const h = svc.runningJob();
    h.failAt = 1;
    h.advance();
    await tick();
    const last = gateway.receiptLog.at(-1);
    assert.equal(last?.status, "rejected");
    assert.ok(String(last?.detail).includes("ingest"));
    assert.equal(gateway.jobProjection.status, "failed");
    fx.unmount();
    host.remove();
  });
});

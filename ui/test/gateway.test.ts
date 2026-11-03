/**
 * `AlGuiGateway` の port レベル検証(#131)。
 *
 * - view/cfg commit が確定値・receipt・projection を正しく発行する
 * - `al.cfg.*` commit が job draft を構成し、`buildRunSpec` が
 *   明示的な stages/config/base_dir を持つ RunSpec になる
 * - unknown target / kind 不一致は正直な receipt になる
 * - subscribe は現在値を即座に push する(reattach で truth 再読)
 */

import assert from "node:assert/strict";
import { describe, it } from "node:test";
import { AlGuiGateway, fingerprint, nestConfig } from "../src/gateway.ts";
import type { CommandIntent, DispatchContext } from "../vendor/ruviform/ui/binding/src/index.ts";
import { FakeAlService } from "./fakes.ts";

const ctx = (id = "cmd-1", expectReceipt = true): DispatchContext => ({
  commandId: id,
  generation: 1,
  issuedAt: 0,
  expectReceipt,
});

const commit = (target: string, plain: number, controlId = "c1"): CommandIntent => ({
  kind: "commit",
  target,
  controlId,
  sequence: 1,
  gestureId: "g1",
  value: { plain, normalized: plain },
});

describe("scalar view/config dispatch", () => {
  it("commit → applied receipt + projection + store 更新", () => {
    const gateway = new AlGuiGateway(new FakeAlService());
    const receipts: string[] = [];
    gateway.dispatchFor(commit("al.view.seek", 12.5), ctx(), (r) => receipts.push(r.status));
    assert.deepEqual(receipts, ["applied"]);
    assert.equal(
      (gateway.targetValue("al.view.seek") as { toString(): string }).toString(),
      "12.5",
    );
  });

  it("begin/preview/cancel は確定値を変えない", () => {
    const gateway = new AlGuiGateway(new FakeAlService());
    gateway.dispatchFor(commit("al.view.seek", 3), ctx(), () => {});
    for (const kind of ["begin", "preview", "cancel"] as const) {
      const intent: CommandIntent =
        kind === "cancel"
          ? {
              kind: "cancel",
              target: "al.view.seek",
              controlId: "c1",
              sequence: 2,
              gestureId: "g2",
              reason: "explicit",
            }
          : kind === "preview"
            ? {
                kind: "preview",
                target: "al.view.seek",
                controlId: "c1",
                sequence: 2,
                gestureId: "g2",
                value: { plain: 9, normalized: 9 },
                coalescible: true,
              }
            : {
                kind: "begin",
                target: "al.view.seek",
                controlId: "c1",
                sequence: 2,
                gestureId: "g2",
                value: { plain: 9, normalized: 9 },
              };
      gateway.dispatchFor(intent, ctx(), () => {});
    }
    assert.equal(gateway.targetValue("al.view.seek"), 3);
    // receipt を期待しない kind には receipt を出さない
    assert.equal(gateway.receiptLog.length, 1);
  });

  it("view target への action は unsupported", () => {
    const gateway = new AlGuiGateway(new FakeAlService());
    const out: { status: string }[] = [];
    gateway.dispatchFor(
      { kind: "action", target: "al.view.seek", controlId: "c1", sequence: 1, action: "play" },
      ctx(),
      (r) => out.push(r),
    );
    assert.equal(out[0]?.status, "unsupported");
  });

  it("unknown target への commit は rejected", () => {
    const gateway = new AlGuiGateway(new FakeAlService());
    const out: { status: string; detail?: string }[] = [];
    gateway.dispatchFor(commit("al.bogus", 1), ctx(), (r) => out.push(r));
    assert.equal(out[0]?.status, "rejected");
  });

  it("subscribe は現在値を即 push し unsubscribe で止まる", () => {
    const gateway = new AlGuiGateway(new FakeAlService());
    gateway.declareTarget("al.view.seek", 0);
    gateway.dispatchFor(commit("al.view.seek", 7), ctx(), () => {});
    const seen: number[] = [];
    const off = gateway.subscribe<number>(
      "al.view.seek",
      (p) => {
        seen.push(p.value);
      },
      { viewId: "v1", generation: 1 },
    );
    assert.deepEqual(seen, [7]);
    gateway.dispatchFor(commit("al.view.seek", 9, "c1"), ctx("cmd-2"), () => {});
    assert.deepEqual(seen, [7, 9]);
    off?.();
    gateway.dispatchFor(commit("al.view.seek", 11, "c1"), ctx("cmd-3"), () => {});
    assert.deepEqual(seen, [7, 9]);
  });
});

describe("range dispatch", () => {
  it("-lo/-hi edge commit が {low,high} を構成する", () => {
    const gateway = new AlGuiGateway(new FakeAlService());
    gateway.seedRange({ low: 0, high: 100 });
    gateway.dispatchFor(commit("al.view.range", 10, "sel-lo"), ctx("c1"), () => {});
    gateway.dispatchFor(commit("al.view.range", 40, "sel-hi"), ctx("c2"), () => {});
    assert.deepEqual(gateway.targetValue("al.view.range"), { low: 10, high: 40 });
  });

  it("projection は {low,high} を保持する", () => {
    const gateway = new AlGuiGateway(new FakeAlService());
    gateway.seedRange({ low: 0, high: 100 });
    const seen: unknown[] = [];
    gateway.subscribe("al.view.range", (p) => seen.push(p.value), { viewId: "v", generation: 1 });
    gateway.dispatchFor(commit("al.view.range", 5, "sel-lo"), ctx("c1"), () => {});
    assert.deepEqual(seen.at(-1), { low: 5, high: 100 });
  });
});

describe("job draft / run spec", () => {
  it("al.cfg.stages.* と al.cfg.<path> が draft を構成する", () => {
    const gateway = new AlGuiGateway(new FakeAlService());
    gateway.dispatchFor(commit("al.cfg.stages.ingest", 1, "t1"), ctx("c1"), () => {});
    gateway.dispatchFor(commit("al.cfg.stages.dataset", 1, "t2"), ctx("c2"), () => {});
    gateway.dispatchFor(commit("al.cfg.stages.ingest", 0, "t1"), ctx("c3"), () => {});
    gateway.dispatchFor(
      commit("al.cfg.analysis.cheap.stft_window", 4096, "n1"),
      ctx("c4"),
      () => {},
    );
    gateway.draft.inputs = [{ role: "capture_bug", path: "/in/a.mp4" }];
    const spec = gateway.buildRunSpec() as unknown as Record<string, unknown>;
    assert.deepEqual(spec.stages, ["dataset"]);
    assert.deepEqual(spec.inputs, [{ role: "capture_bug", path: "/in/a.mp4" }]);
    assert.deepEqual(spec.config, { analysis: { cheap: { stft_window: 4096 } } });
  });

  it("stage 未選択なら stages を送らない(service 既定)", () => {
    const gateway = new AlGuiGateway(new FakeAlService());
    const spec = gateway.buildRunSpec() as unknown as Record<string, unknown>;
    assert.equal("stages" in spec, false);
  });

  it("configure(base_dir) が以降の spec に明示付与される", async () => {
    const svc = new FakeAlService();
    const gateway = new AlGuiGateway(svc);
    await gateway.configure("/base");
    const spec = gateway.buildRunSpec() as unknown as Record<string, unknown>;
    assert.equal(spec.base_dir, "/base");
    assert.ok(svc.calls.some((c) => c.op === "al.app.configure"));
  });
});

describe("helpers", () => {
  it("nestConfig が dotted path を nest する", () => {
    assert.deepEqual(
      nestConfig([
        ["analysis.cheap.stft_window", 2048],
        ["analysis.cheap.level_window_s", 0.02],
      ]),
      { analysis: { cheap: { stft_window: 2048, level_window_s: 0.02 } } },
    );
  });

  it("fingerprint は内容で変わり内容が同じなら同じ", () => {
    assert.equal(fingerprint({ a: 1 }), fingerprint({ a: 1 }));
    assert.notEqual(fingerprint({ a: 1 }), fingerprint({ a: 2 }));
  });
});

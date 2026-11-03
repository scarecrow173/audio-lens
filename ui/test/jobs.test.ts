/**
 * Job ライフサイクル検証(#131)。
 *
 * - `al.job` の `run`/`resume`/`cancel` action が `al.job.run`/
 *   `al.job.resume` op と IPC cancel を呼び、receipt と projection が
 *   job のライフサイクルを正直に反映する
 * - 成功: accepted → applied + `succeeded` projection(job_id・stages)
 * - cancel: run receipt は `rejected`+`cancelled` detail、projection は
 *   `cancelled` + 部分的 stage 状況(完了を偽装しない)
 * - 失敗: `rejected` + `failed` projection(stage error を保持)
 * - resume は job_id 未選択なら `rejected`、未知 action は `unsupported`
 */

import assert from "node:assert/strict";
import { describe, it } from "node:test";
import { AlGuiGateway } from "../src/gateway.ts";
import { mountJobSettings } from "../src/screens/job-settings.ts";
import { mount } from "./env.ts";
import { FakeAlService, flushSync } from "./fakes.ts";
import { tick } from "./helpers.ts";

const setup = () => {
  const svc = new FakeAlService();
  const gateway = new AlGuiGateway(svc);
  return { svc, gateway };
};

describe("al.job run", () => {
  it("Run → al.job.run op が明示 spec で呼ばれ succeeded projection になる", async () => {
    const { svc, gateway } = setup();
    const host = mount("div");
    const fx = mountJobSettings(host, { gateway, flushWith: flushSync });
    fx.setInputs([
      { role: "capture_bug", path: "/in/bug.mp4", input_id: "bug" },
      { role: "capture_reference", path: "/in/ref.mp4", input_id: "ref" },
    ]);
    fx.buttons.run.click();
    await tick();

    const run = svc.calls.find((c) => c.op === "al.job.run");
    assert.ok(run !== undefined);
    // explicit Run identity: inputs・stages・config が spec に乗る
    assert.deepEqual(run.params.inputs, [
      { role: "capture_bug", path: "/in/bug.mp4", input_id: "bug" },
      { role: "capture_reference", path: "/in/ref.mp4", input_id: "ref" },
    ]);
    assert.ok(Array.isArray(run.params.stages));
    assert.equal((run.params.config as Record<string, unknown>).analysis !== undefined, true);

    assert.equal(gateway.jobProjection.status, "running");
    svc.runningJob().finish();
    await tick();
    assert.equal(gateway.jobProjection.status, "succeeded");
    assert.ok(gateway.jobProjection.job_id !== undefined);
    const receipts = gateway.receiptLog.map((r) => r.status);
    assert.deepEqual(receipts, ["accepted", "applied"]);
    fx.unmount();
    host.remove();
  });

  it("stage toggle off で stages が spec から外れる(explicit stages)", async () => {
    const { svc, gateway } = setup();
    const host = mount("div");
    const fx = mountJobSettings(host, { gateway, flushWith: flushSync });
    fx.setInputs([{ role: "capture_bug", path: "/in/bug.mp4" }]);
    // export を off → stages から外れる
    const toggle = fx.stageToggles.get("export") as unknown as {
      dispatchInput(i: unknown): void;
    };
    toggle.dispatchInput({ type: "set", value: { plain: 0 }, source: "pointer" });
    fx.buttons.run.click();
    await tick();
    const run = svc.calls.find((c) => c.op === "al.job.run");
    const stages = run?.params.stages as string[];
    assert.ok(stages.includes("ingest"));
    assert.ok(!stages.includes("export"));
    svc.runningJob().finish();
    await tick();
    fx.unmount();
    host.remove();
  });

  it("cancel → run receipt は rejected(cancelled)、projection は cancelled で偽装しない", async () => {
    const { svc, gateway } = setup();
    const host = mount("div");
    const fx = mountJobSettings(host, { gateway, flushWith: flushSync });
    fx.setInputs([{ role: "capture_bug", path: "/in/bug.mp4" }]);
    fx.buttons.run.click();
    await tick();
    svc.runningJob().advance(); // 1 stage 完了
    await tick();
    fx.buttons.cancel.click();
    await tick();
    // cancel action 自身は accepted(要求受理 — 実結果は done が届ける)
    // run command は rejected + cancelled detail
    const statuses = gateway.receiptLog.map((r) => `${r.status}${r.detail ? `:${r.detail}` : ""}`);
    assert.deepEqual(statuses, [
      "accepted:al.job.run",
      "accepted:cancel requested",
      "rejected:cancelled",
    ]);
    // 実 ErrorBody は data を持たない — 分類 detail で stage 情報を
    // 保持する形が production と一致する(部分 stage 内訳は
    // service 側の `data` 拡充が必要な別 issue 領域)。
    assert.equal(gateway.jobProjection.status, "cancelled");
    assert.equal(gateway.jobProjection.detail, "job cancelled");
    fx.unmount();
    host.remove();
  });

  it("run 実行中の 2 個目の run は rejected(jobHandle/projection を壊さない)", async () => {
    const { svc, gateway } = setup();
    const host = mount("div");
    const fx = mountJobSettings(host, { gateway, flushWith: flushSync });
    fx.setInputs([{ role: "capture_bug", path: "/in/a.mp4" }]);
    fx.buttons.run.click();
    await tick();
    fx.buttons.run.click(); // double-click 相当
    await tick();
    const runs = svc.calls.filter((c) => c.op === "al.job.run");
    assert.equal(runs.length, 1); // service 側で job は 1 つだけ
    assert.ok(gateway.receiptLog.some((r) => r.detail === "a job is already running"));
    // 先の job が終わっても projection はその job の結果を示す
    svc.runningJob().finish();
    await tick();
    assert.equal(gateway.jobProjection.status, "succeeded");
    fx.unmount();
    host.remove();
  });

  it("全 stage off は `stages: []` を送る(省略→service 全 stage とは別)", async () => {
    const { svc, gateway } = setup();
    const host = mount("div");
    const fx = mountJobSettings(host, { gateway, flushWith: flushSync });
    fx.setInputs([{ role: "capture_bug", path: "/in/a.mp4" }]);
    for (const stage of fx.stageToggles.keys()) {
      const toggle = fx.stageToggles.get(stage) as unknown as {
        dispatchInput(i: unknown): void;
      };
      toggle.dispatchInput({ type: "set", value: { plain: 0 }, source: "pointer" });
    }
    fx.buttons.run.click();
    await tick();
    const run = svc.calls.find((c) => c.op === "al.job.run");
    assert.deepEqual(run?.params.stages, []);
    svc.runningJob().finish();
    await tick();
    fx.unmount();
    host.remove();
  });

  it("stage 失敗は rejected + failed projection(error を保持)", async () => {
    const { svc, gateway } = setup();
    const host = mount("div");
    const fx = mountJobSettings(host, { gateway, flushWith: flushSync });
    fx.setInputs([{ role: "capture_bug", path: "/in/bug.mp4" }]);
    fx.buttons.run.click();
    await tick();
    const h = svc.runningJob();
    h.failAt = 2;
    h.advance();
    h.advance();
    await tick();
    assert.equal(gateway.jobProjection.status, "failed");
    assert.ok(String(gateway.jobProjection.detail).includes("failed"));
    const last = gateway.receiptLog.at(-1);
    assert.equal(last?.status, "rejected");
    fx.unmount();
    host.remove();
  });
});

describe("al.job resume/cancel の前提", () => {
  it("job 未選択の resume は al.job.list で再発見し、候補なしなら rejected", async () => {
    const { svc, gateway } = setup();
    const receipts: string[] = [];
    gateway.dispatchFor(
      { kind: "action", target: "al.job", controlId: "job.resume", sequence: 1, action: "resume" },
      { commandId: "c1", generation: 1, issuedAt: 0, expectReceipt: true },
      (r) => receipts.push(r.status),
    );
    await tick();
    assert.deepEqual(receipts, ["rejected"]);
    // list は問い合わせたが resume 可能 job は無かった
    assert.ok(svc.calls.some((c) => c.op === "al.job.list"));
    assert.ok(!svc.calls.some((c) => c.op === "al.job.resume"));
  });

  it("cancel 済み manifest は al.job.list で再発見して resume できる", async () => {
    const { svc, gateway } = setup();
    svc.addJob({
      jobId: "job-0007",
      inputs: [{ role: "capture_bug", path: "/in/a.mp4" }],
      stages: [
        { stage: "ingest", status: "completed", error: null },
        { stage: "locate", status: "pending", error: null },
      ],
      datasetReady: false,
    });
    gateway.dispatchFor(
      { kind: "action", target: "al.job", controlId: "job.resume", sequence: 1, action: "resume" },
      { commandId: "c1", generation: 1, issuedAt: 0, expectReceipt: true },
      () => {},
    );
    await tick();
    const resume = svc.calls.find((c) => c.op === "al.job.resume");
    assert.equal(resume?.params.job_id, "job-0007");
    svc.runningJob().finish();
    await tick();
    assert.equal(gateway.jobProjection.status, "succeeded");
  });

  it("resume 可能 job が複数なら推測せず rejected", async () => {
    const { svc, gateway } = setup();
    for (const jobId of ["job-0011", "job-0012"]) {
      svc.addJob({
        jobId,
        inputs: [{ role: "capture_bug", path: "/in/a.mp4" }],
        stages: [{ stage: "ingest", status: "pending", error: null }],
        datasetReady: false,
      });
    }
    const receipts: string[] = [];
    gateway.dispatchFor(
      { kind: "action", target: "al.job", controlId: "job.resume", sequence: 1, action: "resume" },
      { commandId: "c1", generation: 1, issuedAt: 0, expectReceipt: true },
      (r) => receipts.push(r.status),
    );
    await tick();
    assert.deepEqual(receipts, ["rejected"]);
    assert.ok(!svc.calls.some((c) => c.op === "al.job.resume"));
  });

  it("running job が無いのに cancel は rejected", async () => {
    const { gateway } = setup();
    const receipts: string[] = [];
    gateway.dispatchFor(
      { kind: "action", target: "al.job", controlId: "job.cancel", sequence: 1, action: "cancel" },
      { commandId: "c1", generation: 1, issuedAt: 0, expectReceipt: true },
      (r) => receipts.push(r.status),
    );
    await tick();
    assert.deepEqual(receipts, ["rejected"]);
  });

  it("未知の job action は unsupported", async () => {
    const { gateway } = setup();
    const receipts: string[] = [];
    gateway.dispatchFor(
      { kind: "action", target: "al.job", controlId: "job.x", sequence: 1, action: "rewind" },
      { commandId: "c1", generation: 1, issuedAt: 0, expectReceipt: true },
      (r) => receipts.push(r.status),
    );
    await tick();
    assert.deepEqual(receipts, ["unsupported"]);
  });

  it("resume は setJobContext の job_id を explicit に使う", async () => {
    const { svc, gateway } = setup();
    svc.addJob({
      jobId: "job-0042",
      inputs: [{ role: "capture_bug", path: "/in/a.mp4" }],
      stages: [
        { stage: "ingest", status: "completed", error: null },
        { stage: "locate", status: "pending", error: null },
      ],
      datasetReady: false,
    });
    gateway.setJobContext({ jobId: "job-0042" });
    gateway.dispatchFor(
      { kind: "action", target: "al.job", controlId: "job.resume", sequence: 1, action: "resume" },
      { commandId: "c1", generation: 1, issuedAt: 0, expectReceipt: true },
      () => {},
    );
    await tick();
    const resume = svc.calls.find((c) => c.op === "al.job.resume");
    assert.equal(resume?.params.job_id, "job-0042");
    svc.runningJob().finish();
    await tick();
    assert.equal(gateway.jobProjection.status, "succeeded");
  });
});

/**
 * FeatureCatalog / featureSpectrumSource 検証(#132)。
 *
 * - `al.ds.features` の record は verbatim で返り、validity/
 *   confidence/support_count/lineage を落とさない
 * - `inspect` は `_artifact` path から `al.ds.artifact` 詳細と
 *   provenance を解決する(経路の無い record は undefined)
 * - 数値列 feature は `SpectrumDataSource` frame として供給できる
 *   (欠測・verbatim、軸は honest な bin index)
 */

import assert from "node:assert/strict";
import { describe, it } from "node:test";
import { FeatureCatalog, featureSpectrumSource } from "../src/viz/features.ts";
import { FakeAlService } from "./fakes.ts";

const RECORD_A = {
  feature_id: "level.rms",
  name: "rms",
  value: [0.5, 0.25, -0.75],
  unit: "dbfs",
  validity: { kind: "valid" },
  confidence: { level: "low", score: 0.4 },
  support_count: 12,
  invalid_reason: "low confidence span",
  algorithm_id: "cheap.rms",
  lineage: ["art-1", "art-0"],
  _role: "features/cheap",
  _artifact: "features/cheap/level.json",
};

const RECORD_B = {
  feature_id: "pitch.f0",
  name: "f0",
  value: 220,
  unit: "hz",
  validity: { kind: "valid" },
  confidence: { level: "high" },
  support_count: 1,
  _role: "features/reference",
};

const setup = () => {
  const svc = new FakeAlService();
  svc.addDataset({
    root: "ds-1",
    featureRecords: [RECORD_A, RECORD_B],
    artifactList: [
      {
        path: "features/cheap/level.json",
        role: "features/cheap",
        store: "json",
        provenance: { job_id: "j-1", stage: "cheap_analysis" },
      },
    ],
    artifactDetails: {
      "features/cheap/level.json": {
        kind: "json",
        path: "features/cheap/level.json",
        provenance: { job_id: "j-1", stage: "cheap_analysis" },
      },
    },
  });
  svc.sessions.set(1, "ds-1");
  return { svc, cat: new FeatureCatalog(svc, 1) };
};

describe("FeatureCatalog.search", () => {
  it("record は verbatim — validity/confidence/support/lineage を保持", async () => {
    const { cat } = setup();
    const r = await cat.search();
    assert.equal(r.records.length, 2);
    const rec = r.records.find((x) => x.feature_id === "level.rms");
    assert.ok(rec !== undefined);
    assert.deepEqual(rec.validity, { kind: "valid" });
    assert.deepEqual(rec.confidence, { level: "low", score: 0.4 });
    assert.equal(rec.support_count, 12);
    assert.deepEqual(rec.lineage, ["art-1", "art-0"]);
    assert.equal(rec.invalid_reason, "low confidence span");
    assert.equal(rec._artifact, "features/cheap/level.json");
  });

  it("roles/prefix/limit は op params として届く", async () => {
    const { svc, cat } = setup();
    const r = await cat.search({
      roles: ["features/cheap"],
      featureIdPrefix: "level.",
      limit: 5,
    });
    const call = svc.calls.find((c) => c.op === "al.ds.features");
    assert.ok(call !== undefined);
    assert.deepEqual(call.params.roles, ["features/cheap"]);
    assert.equal(call.params.feature_id_prefix, "level.");
    assert.equal(call.params.limit, 5);
    assert.equal(r.records.length, 1);
    assert.equal(r.records[0]?.feature_id, "level.rms");
  });

  it("truncated はそのまま保持する", async () => {
    const { cat } = setup();
    const r = await cat.search({ limit: 1 });
    assert.equal(r.truncated, true);
    assert.equal(r.records.length, 1);
  });
});

describe("FeatureCatalog.inspect", () => {
  it("`_artifact` path から detail + provenance を解決する", async () => {
    const { cat } = setup();
    const out = await cat.inspect({ ...RECORD_A });
    assert.ok(out !== undefined);
    assert.equal(out.artifactPath, "features/cheap/level.json");
    assert.deepEqual(out.detail, {
      kind: "json",
      path: "features/cheap/level.json",
      provenance: { job_id: "j-1", stage: "cheap_analysis" },
    });
    assert.equal(out.record.feature_id, "level.rms");
  });

  it("`_artifact` の無い record は undefined(経路を捏造しない)", async () => {
    const { cat } = setup();
    assert.equal(await cat.inspect({ feature_id: "x" }), undefined);
  });
});

describe("featureSpectrumSource", () => {
  const AXIS = { kind: "uniform", firstHz: 0, stepHz: 100 } as const;

  it("数値列 feature を Spectrum frame として供給する(axis 必須)", () => {
    const src = featureSpectrumSource(RECORD_A, AXIS);
    assert.ok(src !== undefined);
    const meta = src.meta();
    assert.equal(meta.bins, 3);
    assert.equal(meta.id, "level.rms");
    const frame = src.snapshot();
    assert.ok(frame !== null);
    assert.deepEqual(Array.from(frame.magnitudes), [0.5, 0.25, -0.75]);
  });

  it("axis が無ければ undefined(bin index を Hz と偽らない)", () => {
    // @ts-expect-error axis は必須引数
    assert.equal(featureSpectrumSource(RECORD_A), undefined);
  });

  it("数値列を持たない record は undefined(spectrum truth を捏造しない)", () => {
    assert.equal(featureSpectrumSource(RECORD_B, AXIS), undefined);
    assert.equal(featureSpectrumSource({ feature_id: "x" }, AXIS), undefined);
  });
});

/**
 * SyncTimeline モデル検証(#132)。
 *
 * - `al.ds.overview`/`al.ds.locate`/`al.ds.events` 応答から共有
 *   capture-time 軸を組み立てる
 * - alignment 行の `status`/`validity`/`confidence`/`agreeing_methods`/
 *   `capture_start`/`capture_end` を verbatim に保持する
 * - capture→source の写像は locate_detail の範囲情報(time_map が
 *   あれば piecewise 補間)を使い、写像不能は `undefined`
 * - manifest input にあって audio artifact の無い track は `missing`
 * - event marker は `rvf` overlays 形状で共有軸上の位置を保持する
 */

import assert from "node:assert/strict";
import { describe, it } from "node:test";
import { SyncTimeline, timeMapCaptureToSource, timeMapViewFromWire } from "../src/viz/timeline.ts";
import { FakeAlService } from "./fakes.ts";

const CAP_RANGE_END = 48000 * 60;

// locate/summary の実 wire 形 — validity/confidence は Cell::Str
// (snake_case 文字列)、capture_start/end は I64(stage.rs:324-339)。
const LOCATE_ROW = {
  source_input_id: "src-01",
  capture_input_id: "cap-01",
  alignment_id: "al-1",
  status: "accepted",
  validity: "valid",
  confidence: "high",
  agreeing_methods: 3,
  capture_start: 48000,
  capture_end: CAP_RANGE_END,
};

const REJECTED_ROW = {
  source_input_id: "src-01",
  capture_input_id: "cap-02",
  alignment_id: "al-2",
  status: "rejected",
  validity: "invalid",
  confidence: "low",
  agreeing_methods: 1,
  capture_start: 96000,
  capture_end: 96000 + 48000,
};

// `al.ds.locate_detail` の実 wire 形 — `locate/<src>/<cap>.json` は
// `{result: LocateResult, derived: {...}}` の envelope(al-locate
// stage.rs の persist_json)。
const DETAIL = {
  result: {
    alignment_id: "al-1",
    source_input_id: "src-01",
    capture_input_id: "cap-01",
    normalized_source_range: { start: 0, end: CAP_RANGE_END - 48000 },
    capture_range: { start: 48000, end: CAP_RANGE_END },
    source_range: { start: 0, end: CAP_RANGE_END - 48000 },
    source_pcm: { artifact_id: "a-1", sample_rate: 48000, channels: 2 },
    capture_pcm: { artifact_id: "a-2", sample_rate: 48000, channels: 2 },
    accepted: true,
    validity: "valid",
    confidence: "high",
  },
  derived: { position_spread: 0 },
};

const setup = async () => {
  const svc = new FakeAlService();
  svc.addDataset({
    root: "ds-1",
    inputs: [
      { input_id: "src-01", role: "source_asset" },
      { input_id: "cap-01", role: "capture_bug" },
      { input_id: "cap-02", role: "capture_reference" },
      { input_id: "gone-01", role: "capture_bug" },
    ],
    // gone-01 は manifest にはあるが audio artifact が無い(missing)
    timelineInputs: [
      { input_id: "src-01", frames: 48000 * 60, sample_rate: 48000, channels: 2, duration_s: 60 },
      { input_id: "cap-01", frames: 48000 * 90, sample_rate: 48000, channels: 2, duration_s: 90 },
      { input_id: "cap-02", frames: 48000 * 75, sample_rate: 48000, channels: 1, duration_s: 75 },
    ],
    durationS: 60,
    eventRows: [
      { start_s: 12.5, kind: "dropout", confidence: { level: "high" } },
      { start_s: 80.0, kind: "clip" },
      { kind: "no-time" },
    ],
    locateRows: [LOCATE_ROW, REJECTED_ROW],
    locateDetailPaths: [
      { path: "locate/src-01/cap-01.json", provenance: { job_id: "j-1" } },
      { path: "locate/src-01/cap-02.json", provenance: { job_id: "j-1" } },
    ],
    locateDetails: { "src-01/cap-01": DETAIL },
  });
  const session = 1;
  svc.sessions.set(1, "ds-1");
  const ov = (await svc.call("al.ds.overview", { session })) as {
    overview: unknown;
    timeline: unknown;
  };
  const ev = await svc.call("al.ds.events", { session });
  const loc = await svc.call("al.ds.locate", { session });
  const detail = await svc.call("al.ds.locate_detail", {
    session,
    source: "src-01",
    capture: "cap-01",
  });
  const tl = new SyncTimeline({
    overview: ov.overview,
    timeline: ov.timeline,
    locate: loc,
    locateDetails: [{ source: "src-01", capture: "cap-01", detail }],
    events: ev,
  });
  return { svc, tl };
};

describe("SyncTimeline tracks", () => {
  it("audio のある input は available、audio artifact 無しは missing", async () => {
    const { tl } = await setup();
    assert.equal(tl.track("src-01")?.state, "available");
    assert.equal(tl.track("cap-01")?.state, "available");
    assert.equal(tl.track("gone-01")?.state, "missing");
    assert.equal(tl.track("gone-01")?.durationS, undefined);
    // manifest の role を保持
    assert.equal(tl.track("cap-01")?.role, "capture_bug");
    assert.equal(tl.track("cap-02")?.channels, 1);
  });

  it("extent は capture 系 track の最大 duration(共有軸)", async () => {
    const { tl } = await setup();
    assert.equal(tl.extentS, 90);
  });
});

describe("SyncTimeline alignments(verbatim 保持)", () => {
  it("status/validity/confidence/agreeing_methods/capture range を保持", async () => {
    const { tl } = await setup();
    const a = tl.alignment("src-01", "cap-01");
    assert.equal(a?.status, "accepted");
    // wire の Cell::Str を verbatim 保持(object 化しない)
    assert.equal(a?.validity, "valid");
    assert.equal(a?.confidence, "high");
    assert.equal(a?.agreeingMethods, 3);
    assert.equal(a?.captureStart, 48000);
    assert.equal(a?.captureEnd, CAP_RANGE_END);
    assert.equal(a?.detailPath, "locate/src-01/cap-01.json");
    assert.equal(a?.detail, DETAIL);
    // rejected も消えずに残る(accepted に畳まない)
    const r = tl.alignment("src-01", "cap-02");
    assert.equal(r?.status, "rejected");
    assert.equal(r?.validity, "invalid");
    assert.equal(tl.alignments.length, 2);
  });
});

describe("capture→source 時間写像", () => {
  it("detail の normalized/capture range で定数オフセット写像する", async () => {
    const { tl } = await setup();
    const a = tl.alignment("src-01", "cap-01");
    assert.ok(a !== undefined);
    // capture 2.0s = sample 96000 → norm 48000 → source 1.0s
    assert.equal(tl.captureToSourceS(a, 2.0), 1.0);
    assert.equal(tl.captureToSourceS(a, 1.0), 0.0);
  });

  it("detail の無い alignment は undefined(写像を捏造しない)", async () => {
    const { tl } = await setup();
    const r = tl.alignment("src-01", "cap-02");
    assert.ok(r !== undefined);
    assert.equal(tl.captureToSourceS(r, 2.0), undefined);
  });

  it("time_map segment 補間は TimeMap::capture_to_source と同規約", () => {
    // `time_map.json` artifact の serde wire 形(snake_case)から変換する
    const map = timeMapViewFromWire({
      segments: [
        {
          kind: "continuous",
          anchors: [
            { source: 0, capture: 0, score: 1 },
            { source: 48000, capture: 48000, score: 1 },
          ],
          capture_range: [0, 48000],
          source_range: [0, 48000],
          rate: 48000,
        },
        {
          kind: "gap",
          anchors: [],
          capture_range: [48000, 96000],
        },
      ],
    });
    assert.ok(map !== undefined);
    assert.equal(map.segments[0]?.captureRange?.[1], 48000);
    assert.equal(timeMapCaptureToSource(map, 24000), 24000);
    assert.equal(timeMapCaptureToSource(map, 60000), undefined); // gap
    assert.equal(timeMapCaptureToSource(map, 200000), undefined); // 範囲外
    // wire 形が壊れていれば変換を捏造せず undefined
    assert.equal(timeMapViewFromWire({ segments: "x" }), undefined);
    assert.equal(timeMapViewFromWire("x"), undefined);
  });

  it("time_map 経路の rate は normalized rate(capture rate)を使う", async () => {
    // source 原本 44100 / capture 48000 — anchor.source は normalized
    // PCM 座標(resample 済み、rate = 48000)なので、割るのは capture
    // rate。原本 rate で割ると ~1.088 倍ずれる。
    const svc = new FakeAlService();
    svc.addDataset({
      root: "ds-2",
      inputs: [
        { input_id: "src-1", role: "source_asset" },
        { input_id: "cap-1", role: "capture_bug" },
      ],
      timelineInputs: [
        { input_id: "src-1", frames: 44100 * 60, sample_rate: 44100, channels: 2, duration_s: 60 },
        { input_id: "cap-1", frames: 48000 * 60, sample_rate: 48000, channels: 2, duration_s: 60 },
      ],
      durationS: 60,
      locateRows: [
        {
          source_input_id: "src-1",
          capture_input_id: "cap-1",
          alignment_id: "al-x",
          status: "accepted",
          capture_start: 0,
          capture_end: 48000,
        },
      ],
      locateDetailPaths: [{ path: "locate/src-1/cap-1.json", provenance: {} }],
      locateDetails: {
        "src-1/cap-1": {
          result: {
            source_pcm: { sample_rate: 48000 },
            capture_range: { start: 0, end: 48000 },
            normalized_source_range: { start: 0, end: 48000 },
          },
        },
      },
    });
    svc.sessions.set(1, "ds-2");
    const ov = (await svc.call("al.ds.overview", { session: 1 })) as {
      overview: unknown;
      timeline: unknown;
    };
    const loc = await svc.call("al.ds.locate", { session: 1 });
    const wire = timeMapViewFromWire({
      segments: [
        {
          kind: "continuous",
          anchors: [
            { source: 0, capture: 0 },
            { source: 48000, capture: 48000 },
          ],
          capture_range: [0, 48000],
        },
      ],
    });
    assert.ok(wire !== undefined);
    const tl = new SyncTimeline({
      overview: ov.overview,
      timeline: ov.timeline,
      locate: loc,
      timeMaps: new Map([["locate/src-1/cap-1/time_map.json", wire]]),
    });
    const a = tl.alignment("src-1", "cap-1");
    assert.ok(a !== undefined);
    // capture 0.5s → normalized pos 24000 → source 0.5s
    // (原本 rate 44100 で割る誤りだと ~0.544 になる)
    assert.equal(tl.captureToSourceS(a, 0.5), 0.5);
  });
});

describe("event marker", () => {
  it("共有軸上の event を overlays 形状で返す(位置の無い行は残す)", async () => {
    const { tl } = await setup();
    const markers = tl.markers();
    assert.equal(markers.length, 2);
    assert.deepEqual(markers[0], { at: 12.5, label: "dropout" });
    assert.deepEqual(markers[1], { at: 80.0, label: "clip" });
    // 時間フィルタ — markers(0,50)
    const m50 = tl.markers(0, 50);
    assert.equal(m50.length, 1);
    assert.equal(m50[0]?.at, 12.5);
    // verbatim rows も残る
    assert.equal(tl.eventRows.length, 3);
  });
});

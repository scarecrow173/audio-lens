/**
 * RVBW/RVST デコーダ検証(#132)。
 *
 * - `ruviform-viz` `encode.rs` の wire layout を厳密に復号する
 * - f64 の min/max/rms/count は bit そのまま保持(精度劣化なし)
 * - `NaN` は欠測として保持し、0 埋め・補間しない
 * - `u8` tile の `255` sentinel は再正規化せず wire のまま残す
 * - 破損(magic/version/length/dtype 不整合)は黙らず例外で失敗する
 */

import assert from "node:assert/strict";
import { describe, it } from "node:test";
import { decodeRvbw, decodeRvst } from "../src/viz/decode.ts";
import { encodeRvbw, encodeRvstF32 } from "./fakes.ts";

describe("decodeRvbw", () => {
  it("f64 bin 値を bit そのまま復元する", () => {
    // runtime 計算値(literal の f64 丸めを持ち込まない)でも
    // bit そのまま往復する
    const [p0, p1, p2] = [Math.PI / 1e9, Math.PI, -Math.PI / 100];
    const payload = encodeRvbw([
      { start: 0, end: 480, min: p0, max: p1, rms: p2, count: 480 },
      { start: 480, end: 960, min: -1.5, max: 2.5, rms: 1.25, count: 480 },
    ]);
    const v = decodeRvbw(payload);
    assert.equal(v.bins.length, 2);
    assert.equal(v.bins[0]?.min, p0);
    assert.equal(v.bins[0]?.max, p1);
    assert.equal(v.bins[0]?.rms, p2);
    assert.equal(v.bins[0]?.count, 480);
    assert.equal(v.bins[0]?.start, 0);
    assert.equal(v.bins[1]?.end, 960);
  });

  it("NaN 統計はそのまま残り count=0 の全欠測 bucket を偽装しない", () => {
    const payload = encodeRvbw([
      { start: 0, end: 10, min: Number.NaN, max: Number.NaN, rms: Number.NaN, count: 0 },
      { start: 10, end: 20, min: -0.5, max: 0.5, rms: 0.3, count: 10 },
    ]);
    const v = decodeRvbw(payload);
    assert.ok(Number.isNaN(v.bins[0]?.min));
    assert.ok(Number.isNaN(v.bins[0]?.max));
    assert.ok(Number.isNaN(v.bins[0]?.rms));
    assert.equal(v.bins[0]?.count, 0);
    // 隣接 bucket は影響を受けない(補間しない)
    assert.equal(v.bins[1]?.min, -0.5);
  });

  it("空の bin 列は正しい空 view を返す", () => {
    const v = decodeRvbw(encodeRvbw([]));
    assert.equal(v.bins.length, 0);
  });

  it("magic/version/長さの不整合は例外で失敗する", () => {
    const good = encodeRvbw([{ start: 0, end: 1, min: 0, max: 0, rms: 0, count: 1 }]);
    const badMagic = good.slice();
    badMagic[0] = 0x58; // "X"
    assert.throws(() => decodeRvbw(badMagic), /magic/);
    const badVer = good.slice();
    new DataView(badVer.buffer).setUint16(4, 2, true);
    assert.throws(() => decodeRvbw(badVer), /version/);
    assert.throws(() => decodeRvbw(good.slice(0, 20)), /length/);
    assert.throws(() => decodeRvbw(new Uint8Array(4)), /header/);
  });
});

describe("decodeRvst", () => {
  it("f32 tile を row-major のまま復元し row 0 が先頭", () => {
    const payload = encodeRvstF32(2, 3, 7, [0.5, -1.25, 3.75, 8.0, 16.5, -64.0]);
    const t = decodeRvst(payload);
    assert.equal(t.dtype, 1);
    assert.equal(t.rows, 2);
    assert.equal(t.cols, 3);
    assert.equal(t.frameStart, 7);
    assert.equal(t.values[0], Math.fround(0.5));
    assert.equal(t.values[5], Math.fround(-64.0));
  });

  it("f64 dtype は bit そのまま復元する", () => {
    const [v0, v1] = [Math.PI / 1e9, -Math.PI];
    const buf = new Uint8Array(24 + 16);
    const dv = new DataView(buf.buffer);
    buf.set([0x52, 0x56, 0x53, 0x54], 0);
    dv.setUint16(4, 1, true);
    dv.setUint16(6, 0, true); // f64
    dv.setUint32(8, 2, true);
    dv.setUint32(12, 1, true);
    dv.setBigUint64(16, 11n, true);
    dv.setFloat64(24, v0, true);
    dv.setFloat64(32, v1, true);
    const t = decodeRvst(buf);
    assert.equal(t.dtype, 0);
    assert.equal(t.values[0], v0);
    assert.equal(t.values[1], v1);
    assert.equal(t.frameStart, 11);
  });

  it("u8 dtype の 255 sentinel を wire のまま残す(逆正規化しない)", () => {
    const buf = new Uint8Array(24 + 4);
    const dv = new DataView(buf.buffer);
    buf.set([0x52, 0x56, 0x53, 0x54], 0);
    dv.setUint16(4, 1, true);
    dv.setUint16(6, 2, true); // u8
    dv.setUint32(8, 2, true);
    dv.setUint32(12, 2, true);
    buf.set([128, 255, 64, 0], 24);
    const t = decodeRvst(buf);
    assert.equal(t.dtype, 2);
    assert.equal(t.values[1], 255); // NaN sentinel — f64 値へ写像しない
    assert.equal(t.values[0], 128);
  });

  it("NaN (f32) は tile 内でそのまま残る", () => {
    const t = decodeRvst(encodeRvstF32(1, 2, 0, [Number.NaN, 0.25]));
    assert.ok(Number.isNaN(t.values[0]));
    assert.equal(t.values[1], Math.fround(0.25));
  });

  it("dtype/長さ不整合は例外で失敗する", () => {
    const good = encodeRvstF32(1, 2, 0, [1, 2]);
    const badDtype = good.slice();
    new DataView(badDtype.buffer).setUint16(6, 9, true);
    assert.throws(() => decodeRvst(badDtype), /dtype/);
    assert.throws(() => decodeRvst(good.slice(0, 24 + 4)), /length/);
    assert.throws(() => decodeRvst(new Uint8Array(8)), /header|magic/);
  });
});

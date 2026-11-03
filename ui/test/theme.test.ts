/**
 * Theme 検証(#131)。
 *
 * - `AUDIOLENS_THEME` は `resolveTheme` で diagnostics ゼロ — token は
 *   すべてスキーマ適合。
 * - Reference Skin(テーマ未指定)はそのまま動作し、製品テーマ適用は
 *   入力・コマンド・結果を一切変えない(theme はデータであって
 *   操作ロジックではない)。
 */

import assert from "node:assert/strict";
import { describe, it } from "node:test";
import { AlGuiGateway } from "../src/gateway.ts";
import { mountSeekRange } from "../src/screens/seek-range.ts";
import { AUDIOLENS_THEME } from "../src/theme.ts";
import { resolveTheme } from "../vendor/ruviform/ui/components/src/index.ts";
import { mount } from "./env.ts";
import { FakeAlService, flushSync } from "./fakes.ts";
import { part, ptr, stubRect } from "./helpers.ts";

describe("AUDIOLENS_THEME", () => {
  it("token がすべてスキーマ適合(diagnostics 空)", () => {
    const resolved = resolveTheme(AUDIOLENS_THEME);
    assert.deepEqual(resolved.diagnostics, []);
    assert.ok(resolved.tokens.interactive !== undefined);
    assert.ok(resolved.tokens["focus.ring"] !== undefined);
    assert.ok(resolved.tokens["viz.series.1"] !== undefined);
    assert.ok(resolved.tokens["status.error"] !== undefined);
  });

  it("base が dark で density が default", () => {
    assert.equal(AUDIOLENS_THEME.base, "dark");
    assert.equal(AUDIOLENS_THEME.density, "default");
  });
});

describe("theme parity — 操作・結果がテーマに依存しない", () => {
  it("Reference Skin でも製品テーマでも同一 gesture→同一確定値・同一 receipt", async () => {
    const run = async (theme: "none" | "product") => {
      const svc = new FakeAlService();
      svc.addDataset({ root: "ds-1" });
      const gateway = new AlGuiGateway(svc);
      const host = mount("div");
      const fx = mountSeekRange(host, {
        gateway,
        ...(theme === "none" ? { theme: null } : {}),
        durationS: 100,
        flushWith: flushSync,
      });
      const track = part(fx.selector, "track");
      stubRect(track, { left: 0, width: 100 });
      const lo = part(fx.selector, "thumb-lo");
      lo?.dispatchEvent(ptr("pointerdown", { clientX: 0 }));
      lo?.dispatchEvent(ptr("pointermove", { clientX: 30 }));
      lo?.dispatchEvent(ptr("pointerup", { clientX: 30 }));
      const range = fx.committedRange();
      const receipts = gateway.receiptLog.map((r) => r.status);
      fx.unmount();
      host.remove();
      return { range, receipts };
    };
    const a = await run("none");
    const b = await run("product");
    assert.deepEqual(a.range, b.range);
    assert.deepEqual(a.receipts, b.receipts);
    assert.equal(a.range.low, 30);
    assert.equal(a.range.high, 100);
    assert.deepEqual(a.receipts, ["applied"]);
  });
});

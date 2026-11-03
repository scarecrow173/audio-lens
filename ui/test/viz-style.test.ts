/**
 * AudioLens theme → VizStyle ブリッジ検証(#132)。
 *
 * - `AUDIOLENS_THEME` の `viz.*` token が `resolveVizStyle` に届く
 * - theme 切替は paint のみ変え、data/query には触れない
 *   (tokenMap は data を持たない)
 */

import assert from "node:assert/strict";
import { describe, it } from "node:test";
import { AUDIOLENS_THEME } from "../src/theme.ts";
import { resolveAlVizStyle, vizTokenMap } from "../src/viz/style.ts";

describe("vizTokenMap", () => {
  it("AudioLens theme の viz.* token を token 名で供給する", () => {
    const map = vizTokenMap();
    assert.equal(map["viz.series.1"], "#4fd1a5");
    assert.equal(map["viz.waveform.fill"], "#45b8b44d");
    assert.equal(map["viz.gap.fill"], "#93a1b01a");
    // base tokens も解決されている(default theme の全 token)
    assert.ok(typeof map["viz.grid"] === "string");
  });
});

describe("resolveAlVizStyle", () => {
  it("theme 既定で ResolvedVizStyle を解決する", () => {
    const { style, diagnostics } = resolveAlVizStyle();
    assert.ok(style.palette.includes("#4fd1a5"));
    assert.equal(style.waveform.fill, "#45b8b44d");
    assert.equal(style.state.gapFill, "#93a1b01a");
    assert.equal(diagnostics.length, 0);
  });

  it("theme 差替えは paint のみ変える(token 由来値の置換)", () => {
    const alt = {
      ...AUDIOLENS_THEME,
      tokens: { ...AUDIOLENS_THEME.tokens, "viz.series.1": "#ff0000" },
    };
    const a = resolveAlVizStyle(alt);
    const b = resolveAlVizStyle();
    // palette[i] ← viz.series.i
    assert.equal(b.style.palette[1], "#4fd1a5");
    assert.equal(a.style.palette[1], "#ff0000");
    // 未差替 token は AudioLens 値のまま
    assert.equal(a.style.palette[0], b.style.palette[0]);
    assert.equal(a.style.waveform.fill, "#45b8b44d");
  });
});

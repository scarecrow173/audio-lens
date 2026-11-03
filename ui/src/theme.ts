/**
 * AudioLens 製品テーマ(#131)。
 *
 * Ruviform Reference Skin の `ThemeInput` — トークンはデータであり、
 * 操作ロジック・gesture・command は一切変えない(base の reference
 * controls をそのまま使い、`rvf-theme` scope にこれを乗せるだけ)。
 * dark base 上に解析ツールらしい cool accent(teal)を持たせ、
 * status/error/warning/pending と viz 系列色を AudioLens 向けに
 * 調整する。`resolveTheme` の diagnostics が空であることがテーマの
 * 正当性条件(テストで担保)。
 */

import type { ThemeInput } from "../vendor/ruviform/ui/components/src/index.ts";

export const AUDIOLENS_THEME: ThemeInput = {
  base: "dark",
  density: "default",
  tokens: {
    // サーフェス — reference dark よりやや青みの強い解析画面用
    surface: "#161b20",
    "surface.raised": "#1e252c",
    text: "#e2e8ee",
    "text.dim": "#93a1b0",
    border: "#3a4652",

    // AudioLens accent(teal)— reference の青と区別できる製品色
    interactive: "#45b8b4",
    "interactive.hover": "#5cc8c4",
    "interactive.active": "#7ad8d4",
    "focus.ring": "#45b8b4",

    // semantic state — rejected/invalid/unsupported は色だけでなく
    // element 側の状態属性でも区別される(テーマは色だけ担当)
    "status.error": "#e07070",
    "status.warning": "#d9a44a",
    "status.pending": "#8fa6e8",

    // control tracks
    "knob.arc.track": "#39424d",
    "fader.track": "#39424d",
    "toggle.track": "#454f5c",
    "toggle.thumb": "#e2e8ee",
    "seekbar.track": "#39424d",
    "seekbar.buffered": "#556374",
    "range.track": "#39424d",

    // visualization tokens(viz element が消費する role)
    "viz.grid": "#2e3844",
    "viz.series.1": "#4fd1a5",
    "viz.series.2": "#e0a04f",
    "viz.series.3": "#c97ed0",
    "viz.series.4": "#e07070",
    "viz.series.5": "#8fa6e8",
    "viz.series.6": "#a8b545",
    "viz.series.7": "#5fb0c9",
    "viz.waveform.fill": "#45b8b44d",
    "viz.waveform.rms": "#45b8b466",
    "viz.meter.low": "#4fbf80",
    "viz.meter.mid": "#e0b44f",
    "viz.meter.high": "#e07060",
    "viz.meter.track": "#2e3844",
    "viz.spectrum.fill": "#45b8b433",
    "viz.selection.fill": "#45b8b42e",
    "viz.gap.fill": "#93a1b01a",
  },
};

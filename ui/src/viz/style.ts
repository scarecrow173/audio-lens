/**
 * AudioLens theme → `VizStyle` ブリッジ(#132)。
 *
 * `AUDIOLENS_THEME`(#131)の `viz.*` token を `resolveVizStyle` へ
 * そのまま渡す — token はデータであり、theme 切替は paint のみを
 * 変え、data query・再解析・source revision を一切起こさない
 * (ruviform viz の VizStyle 契約どおり)。
 */

import { resolveTheme, type ThemeInput } from "../../vendor/ruviform/ui/components/src/index.ts";
import {
  type ResolvedVizStyle,
  resolveVizStyle,
  type VizStyleDiagnostic,
  type VizStyleInput,
} from "../../vendor/ruviform/ui/viz/src/index.ts";
import { AUDIOLENS_THEME } from "../theme.ts";

/** theme を `resolveVizStyle` 第1引数(token 名 → css text)に写す。 */
export function vizTokenMap(theme: ThemeInput = AUDIOLENS_THEME): Record<string, string> {
  return { ...resolveTheme(theme).tokens };
}

/** AudioLens theme(+任意の instance override)で VizStyle を解決する。 */
export function resolveAlVizStyle(
  theme: ThemeInput = AUDIOLENS_THEME,
  input?: VizStyleInput,
): { style: ResolvedVizStyle; diagnostics: VizStyleDiagnostic[] } {
  return resolveVizStyle(vizTokenMap(theme), input);
}

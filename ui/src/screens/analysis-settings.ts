/**
 * Analysis settings 画面(#131)。
 *
 * `config.analysis.*` の draft config を編集するコントロール群 —
 * commit した値は job draft へ入り、明示的な `al.job` `run` のときに
 * だけ RunSpec の `config` としてサービスへ渡る。表示整形のみで
 * f64 値を丸め戻さない。
 *
 * - `rvf-fader` → `al.cfg.analysis.cheap.level_window_s`(連続値)
 * - `rvf-numeric-input` → `al.cfg.analysis.cheap.stft_window`,
 *   `al.cfg.analysis.cheap.clip_threshold`, `al.cfg.analysis.reference.ridge`,
 *   `al.cfg.analysis.cheap.ipd_max_lag_s`
 *
 * config の値語彙は numeric のみ(gateway 側も numeric のみ受理 —
 * boolean key はまだ対象にしない)。
 */

import type { CommandBinding } from "../../vendor/ruviform/ui/binding/src/index.ts";
import { registerRvfElements } from "../../vendor/ruviform/ui/components/src/index.ts";
import {
  bindScalarControl,
  makeLink,
  mountScope,
  type ScreenFixture,
  type ScreenMountOptions,
  Unbinds,
} from "./shared.ts";

interface ParamDef {
  readonly key: string;
  readonly element: "rvf-fader" | "rvf-numeric-input";
  readonly descriptor: {
    min: number;
    max: number;
    defaultValue: number;
    step?: number;
    unit?: string;
    mapping?: { kind: "linear" } | { kind: "log" };
  };
}

/** 実 config キーと対応する既定値(crates/al-analysis 実装と一致)。 */
const PARAMS: readonly ParamDef[] = [
  {
    key: "analysis.cheap.level_window_s",
    element: "rvf-fader",
    descriptor: { min: 0.005, max: 0.2, defaultValue: 0.02, step: 0.005, unit: "s" },
  },
  {
    key: "analysis.cheap.stft_window",
    element: "rvf-numeric-input",
    descriptor: { min: 256, max: 16384, defaultValue: 2048, step: 256, unit: "samples" },
  },
  {
    key: "analysis.cheap.clip_threshold",
    element: "rvf-numeric-input",
    descriptor: { min: 0.5, max: 1.0, defaultValue: 0.999, step: 0.001 },
  },
  {
    key: "analysis.reference.ridge",
    element: "rvf-numeric-input",
    descriptor: { min: 1e-9, max: 1e-3, defaultValue: 1e-6, mapping: { kind: "log" } },
  },
  {
    key: "analysis.cheap.ipd_max_lag_s",
    element: "rvf-numeric-input",
    descriptor: { min: 0.0001, max: 0.01, defaultValue: 0.001, step: 0.0001, unit: "s" },
  },
];

export interface AnalysisSettingsFixture extends ScreenFixture {
  readonly params: ReadonlyMap<string, HTMLElement>;
  readonly bindings: ReadonlyMap<string, CommandBinding<unknown>>;
}

export function mountAnalysisSettings(
  host: HTMLElement,
  opts: ScreenMountOptions,
): AnalysisSettingsFixture {
  registerRvfElements();
  const { gateway } = opts;
  const { root, scope } = mountScope(host, opts.theme);
  const link = makeLink(opts, opts.viewId ?? "al.screen.analysis-settings");
  const unbinds = new Unbinds();

  const params = new Map<string, HTMLElement>();
  const bindings = new Map<string, CommandBinding<unknown>>();
  for (const def of PARAMS) {
    const el = document.createElement(def.element) as HTMLElement & {
      controlId: string;
      target: string;
      descriptor: ParamDef["descriptor"];
      ports: { format: (p: number) => string };
      value: number;
    };
    el.controlId = `cfg-${def.key.replaceAll(".", "-")}`;
    el.target = `al.cfg.${def.key}`;
    el.descriptor = def.descriptor;
    el.ports = { format: (p) => `${Number(p.toPrecision(6))}` };
    el.value = def.descriptor.defaultValue;
    gateway.declareTarget(`al.cfg.${def.key}`, def.descriptor.defaultValue);
    // 既に commit 済みの値は remount で default に戻さない。
    if (!gateway.draft.config.has(def.key)) {
      gateway.draft.config.set(def.key, def.descriptor.defaultValue);
    }
    const { binding, unbind } = bindScalarControl(el, link, gateway, `al.cfg.${def.key}`, opts);
    unbinds.add(unbind);
    params.set(def.key, el);
    bindings.set(def.key, binding);
    scope.append(el);
  }

  link.attach(opts.windowId ?? "main");

  return {
    root,
    scope,
    link,
    params,
    bindings,
    unmount: () => {
      unbinds.run();
      link.detach("unmount");
      root.remove();
    },
  };
}

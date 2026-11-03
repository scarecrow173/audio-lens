/**
 * Seek / Range selection 画面(#131)。
 *
 * - `rvf-seekbar` → `al.view.seek`(scalar view 状態)
 * - `rvf-range-selector` → `al.view.range`(`-lo`/`-hi` edge が同一
 *   target に commit し、gateway が `{low, high}` を確定値として保持)
 * - `issueRangeQuery` … commit 済み range をそのまま
 *   `QueryBinding.request` の `start`/`end` に渡し、Source(session +
 *   inputId + channel)と `sourceRevision` を明示して bounded query
 *   を発行する。表示用丸めは range 値に戻さない。
 */

import type { QueryBinding, QueryOutcome } from "../../vendor/ruviform/ui/binding/src/index.ts";
import { registerRvfElements } from "../../vendor/ruviform/ui/components/src/index.ts";
import type { RangeValue } from "../gateway.ts";
import {
  bindRangeEdges,
  bindScalarControl,
  makeLink,
  mountScope,
  type ScreenFixture,
  type ScreenMountOptions,
  Unbinds,
} from "./shared.ts";

export interface SeekRangeFixture extends ScreenFixture {
  readonly seekbar: HTMLElement & { value: number };
  readonly selector: HTMLElement & { valueLow: number; valueHigh: number };
  readonly query: QueryBinding;
  /** commit 済みの `[low, high]` を explicit range として bounded
   *  query を発行する(range を再パース・丸めない)。 */
  issueRangeQuery(
    sourceId: string,
    sourceRevision: number | string,
    extras?: { buckets?: number; channels?: readonly number[]; maxBytes?: number },
  ): Promise<QueryOutcome>;
  /** 現在の確定 range。 */
  committedRange(): RangeValue;
}

export interface SeekRangeOptions extends ScreenMountOptions {
  readonly durationS?: number;
}

export function mountSeekRange(host: HTMLElement, opts: SeekRangeOptions): SeekRangeFixture {
  registerRvfElements();
  const { gateway } = opts;
  const { root, scope } = mountScope(host, opts.theme);
  const link = makeLink(opts, opts.viewId ?? "al.screen.seek-range");
  const unbinds = new Unbinds();
  const duration = opts.durationS ?? 60;

  const seekbar = document.createElement("rvf-seekbar") as HTMLElement & {
    controlId: string;
    target: string;
    descriptor: { min: number; max: number; defaultValue: number; unit: string };
    ports: { format: (p: number) => string };
    value: number;
  };
  seekbar.controlId = "seek";
  seekbar.target = "al.view.seek";
  seekbar.descriptor = { min: 0, max: duration, defaultValue: 0, unit: "s" };
  seekbar.ports = { format: (p) => `${p.toFixed(3)} s` };
  gateway.declareTarget("al.view.seek", 0);
  const seekCtl = bindScalarControl(seekbar, link, gateway, "al.view.seek", opts);
  unbinds.add(seekCtl.unbind);
  scope.append(seekbar);

  const selector = document.createElement("rvf-range-selector") as HTMLElement & {
    controlId: string;
    target: string;
    descriptor: { min: number; max: number; defaultValue: number; unit: string };
    ports: { format: (p: number) => string };
    valueLow: number;
    valueHigh: number;
    pending?: boolean;
    invalid?: boolean;
  };
  selector.controlId = "sel";
  selector.target = "al.view.range";
  selector.descriptor = { min: 0, max: duration, defaultValue: 0, unit: "s" };
  selector.ports = { format: (p) => `${p.toFixed(3)} s` };
  selector.valueLow = 0;
  selector.valueHigh = duration;
  // 確定値の初期 truth は「全範囲選択」— edge 一方だけ commit しても
  // もう一方が duration に残るよう gateway 側を先に seed する。
  // seedRange は初回のみ効く(remount で commit 済み値を潰さない)。
  gateway.seedRange({ low: 0, high: duration });
  const edges = bindRangeEdges(selector, link, gateway, "al.view.range", "sel", opts);
  unbinds.add(edges.unbind);
  scope.append(selector);

  const query = link.createQuery({
    revisionProbe: (s) => gateway.probeRevision(s),
  });

  link.attach(opts.windowId ?? "main");

  return {
    root,
    scope,
    link,
    seekbar,
    selector,
    query,
    issueRangeQuery: (sourceId, sourceRevision, extras) => {
      const range = gateway.targetValue("al.view.range") as RangeValue;
      return query.request({
        sourceId,
        sourceRevision,
        start: range.low,
        end: range.high,
        ...(extras?.buckets !== undefined ? { buckets: extras.buckets } : {}),
        ...(extras?.channels !== undefined ? { channels: extras.channels } : {}),
        ...(extras?.maxBytes !== undefined ? { maxBytes: extras.maxBytes } : {}),
      });
    },
    committedRange: () => gateway.targetValue("al.view.range") as RangeValue,
    unmount: () => {
      unbinds.run();
      link.detach("unmount");
      root.remove();
    },
  };
}

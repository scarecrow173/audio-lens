/**
 * Media preview 画面(#131)。
 *
 * - `rvf-transport` → `al.media.preview` binding(`bindTransportElement`
 *   が `rvf-transport` intent を non-droppable action command にする)
 * - `rvf-seekbar` → `al.view.seek`(preview 位置の scalar view 状態)
 *
 * preview は表示/ローカル再生の意図だけを持つ — Job run/resume/cancel
 * とは別 target・別 domain で、どちらの操作も他方に副作用を持たない。
 */

import {
  bindTransportElement,
  CommandBinding,
} from "../../vendor/ruviform/ui/binding/src/index.ts";
import { registerRvfElements } from "../../vendor/ruviform/ui/components/src/index.ts";
import {
  bindScalarControl,
  makeLink,
  mountScope,
  type ScreenFixture,
  type ScreenMountOptions,
  Unbinds,
} from "./shared.ts";

export interface MediaPreviewFixture extends ScreenFixture {
  readonly transport: HTMLElement & { state: string };
  readonly seekbar: HTMLElement & { value: number };
  readonly previewBinding: CommandBinding<unknown>;
  readonly seekBinding: CommandBinding<unknown>;
}

export interface MediaPreviewOptions extends ScreenMountOptions {
  /** 表示 duration(秒)— seekbar の descriptor max。 */
  readonly durationS?: number;
}

export function mountMediaPreview(
  host: HTMLElement,
  opts: MediaPreviewOptions,
): MediaPreviewFixture {
  registerRvfElements();
  const { gateway } = opts;
  const { root, scope } = mountScope(host, opts.theme);
  const link = makeLink(opts, opts.viewId ?? "al.screen.media-preview");
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
  // 表示用フォーマットは display 層のみ — 値へは戻さない
  // (f64 確定値に display 丸めを混入させない)。
  seekbar.ports = { format: (p) => `${p.toFixed(3)} s` };
  gateway.declareTarget("al.view.seek", 0);
  const seekCtl = bindScalarControl(seekbar, link, gateway, "al.view.seek", opts);
  unbinds.add(seekCtl.unbind);
  scope.append(seekbar);

  const transport = document.createElement("rvf-transport") as HTMLElement & {
    controlId: string;
    state: string;
  };
  transport.controlId = "preview";
  const previewBinding = new CommandBinding<unknown>({
    target: "al.media.preview",
    port: {
      dispatch: (i, c) => gateway.dispatchFor(i, c, (r) => previewBinding.ack(r)),
    },
    ...(opts.flushWith !== undefined ? { flushWith: opts.flushWith } : {}),
    ...(opts.schedule !== undefined ? { schedule: opts.schedule } : {}),
    ...(opts.ackTimeoutMs !== undefined ? { ackTimeoutMs: opts.ackTimeoutMs } : {}),
  });
  link.registerBinding(previewBinding);
  unbinds.add(bindTransportElement(transport, previewBinding));
  // preview 状態を transport の表示へ反映する(projection → state)。
  unbinds.add(
    previewBinding.addStateListener((s) => {
      const v = s.lastObserved?.value;
      if (typeof v === "object" && v !== null && "state" in v) {
        transport.state = String((v as { state?: unknown }).state ?? "stopped");
      }
    }),
  );
  scope.append(transport);

  link.attach(opts.windowId ?? "main");

  return {
    root,
    scope,
    link,
    transport,
    seekbar,
    previewBinding,
    seekBinding: seekCtl.binding,
    unmount: () => {
      unbinds.run();
      link.detach("unmount");
      root.remove();
    },
  };
}

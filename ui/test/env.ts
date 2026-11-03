/**
 * `node --test` 用 headless DOM(#131)。ruviform `ui/components` の
 * test/env.ts と同じ方針: happy-dom が Custom Elements/Shadow DOM/
 * composed CustomEvent を供給し、Node 自体の global(timer・process・
 * URL・structuredClone)は上書きしない。テスト専用依存であり、
 * 製品コードは標準 DOM API だけを使う。
 */

import { Window } from "happy-dom";

const window = new Window({ url: "http://localhost/" });

const GLOBAL_KEYS = [
  "window",
  "self",
  "document",
  "customElements",
  "getComputedStyle",
  "Document",
  "DocumentFragment",
  "Node",
  "Element",
  "HTMLElement",
  "HTMLInputElement",
  "HTMLButtonElement",
  "SVGElement",
  "ShadowRoot",
  "MutationObserver",
  "Event",
  "EventTarget",
  "CustomEvent",
  "MouseEvent",
  "PointerEvent",
  "KeyboardEvent",
  "WheelEvent",
  "InputEvent",
  "FocusEvent",
  "CompositionEvent",
] as const;

const w = window as unknown as Record<string, unknown>;
const g = globalThis as unknown as Record<string, unknown>;

for (const key of GLOBAL_KEYS) {
  if (w[key] !== undefined) g[key] = w[key];
}

g.PointerEvent ??= w.MouseEvent;
g.KeyboardEvent ??= w.Event;
g.CompositionEvent ??= w.Event;
g.FocusEvent ??= w.Event;
g.WheelEvent ??= w.Event;
g.InputEvent ??= w.Event;

export const dom = { window };

/** Fresh element attached to the document. */
export function mount(tag: string, attrs: Record<string, string> = {}): HTMLElement {
  const el = document.createElement(tag);
  for (const [k, v] of Object.entries(attrs)) el.setAttribute(k, v);
  document.body.append(el);
  return el;
}

export function unmount(el: Element): void {
  el.remove();
}

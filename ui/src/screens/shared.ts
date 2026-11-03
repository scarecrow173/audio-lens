/**
 * 画面構成 fixture の共通部品(#131)。
 *
 * 各 mount 関数は `<rvf-theme>` scope の下に共有 control を配置し、
 * ViewLink + CommandBinding(consumer 側 ack クロージャ)で
 * `AlGuiGateway` へ接続する。control の gesture/value 解釈は
 * `ui/core`+`ui/components` 側の機械に委ね、ここでは「どの要素に
 * どの target を割り当てるか」の構成だけを書く。
 */

import {
  type BindingPort,
  type BindingViewState,
  bindControlElement,
  CommandBinding,
  type ScheduleFn,
  type Unbind,
  ViewLink,
} from "../../vendor/ruviform/ui/binding/src/index.ts";
import type { ThemeInput } from "../../vendor/ruviform/ui/components/src/index.ts";
import type { AlGuiGateway } from "../gateway.ts";
import { AUDIOLENS_THEME } from "../theme.ts";

export interface ScreenMountOptions {
  readonly gateway: AlGuiGateway;
  /** 既存 link を再利用する場合(省略時は viewId で新規作成)。 */
  readonly link?: ViewLink;
  /** link を作るときの論理 view 識別子。 */
  readonly viewId?: string;
  /** `attach` する window 識別子(既定 `"main"`)。 */
  readonly windowId?: string;
  /** 省略時は製品テーマ。`undefined` を明示したい場合は `null`。 */
  readonly theme?: ThemeInput | null;
  /** binding の queue flush(テスト用に同期 flush を注入できる)。 */
  readonly flushWith?: (fn: () => void) => void;
  /** ack timeout タイマー(テスト用に手動タイマーを注入できる)。 */
  readonly schedule?: ScheduleFn;
  readonly ackTimeoutMs?: number;
}

export interface ScreenFixture {
  /** scope 要素を包む最外枠(`host` 直下に append 済み)。 */
  readonly root: HTMLElement;
  readonly scope: HTMLElement;
  readonly link: ViewLink;
  /** 全 listener/binding 解除 + DOM detach(unmount 相当)。 */
  unmount(): void;
}

/** mount 時に theme 未指定なら製品テーマを scope へ適用する。 */
export function mountScope(
  host: HTMLElement,
  theme: ThemeInput | null | undefined,
): { root: HTMLElement; scope: HTMLElement } {
  const root = document.createElement("div");
  const scope = document.createElement("rvf-theme") as HTMLElement & {
    theme?: ThemeInput;
  };
  if (theme !== null) scope.theme = theme ?? AUDIOLENS_THEME;
  root.append(scope);
  host.append(root);
  return { root, scope };
}

/** `new ViewLink` の既定 port は gateway 自身 — link 経由の
 *  subscribe/query はこの port へ届く。
 *
 * `opts.link` が既に attach 済みの場合は先に detach する: 上流の
 * `ViewLink.registerBinding` は attach 中に登録された binding を
 * 再 subscribe しない(`createBinding` のみ subscribe 分岐を持つ)
 * ため、同一 window への attach は idempotent early-return となり
 * 新規 binding が projection を受け取れない。detach→attach で全
 *  binding を正しく再 subscribe させる(ruviform 側の改善候補)。
 */
export function makeLink(opts: ScreenMountOptions, viewId: string): ViewLink {
  const link = opts.link ?? new ViewLink({ viewId, port: opts.gateway as BindingPort });
  if (link.attached) link.detach("remount");
  return link;
}

/**
 * binding を作って link へ登録し、control element へ結線する。
 * receipt はクロージャ経由でこの binding の `ack` にだけ届く
 * (commandId は binding ローカル採番のため routing ではなく所有者
 * 直接渡し)。
 */
export function bindScalarControl<P>(
  el: Parameters<typeof bindControlElement>[0],
  link: ViewLink,
  gateway: AlGuiGateway,
  target: string,
  opts: ScreenMountOptions,
  bindingOpts?: {
    controlId?: string;
    project?: (p: { revision: number | string; value: P }) => number | undefined;
  },
): { binding: CommandBinding<P>; unbind: Unbind } {
  const binding = new CommandBinding<P>({
    target,
    port: {
      dispatch: (i, c) => gateway.dispatchFor(i, c, (r) => binding.ack(r)),
    },
    ...(bindingOpts?.controlId !== undefined ? { controlId: bindingOpts.controlId } : {}),
    ...(opts.flushWith !== undefined ? { flushWith: opts.flushWith } : {}),
    ...(opts.schedule !== undefined ? { schedule: opts.schedule } : {}),
    ...(opts.ackTimeoutMs !== undefined ? { ackTimeoutMs: opts.ackTimeoutMs } : {}),
  });
  link.registerBinding(binding);
  const project = bindingOpts?.project;
  const unbind = bindControlElement(
    el,
    binding,
    project !== undefined ? { project: (p) => project(p) } : undefined,
  );
  return { binding, unbind };
}

/**
 * `<rvf-range-selector>` の 2 edge を `al.view.range` へ結線する。
 * element には `value` プロパティがないため `bindControlElement` の
 * 既定反映(value 書き込み)は使えない — ここでは同じ contract を
 * 手で配線する(gesture → binding.event、projection → valueLow/
 * valueHigh 反映、pending/invalid フラグ同期)。
 */
export function bindRangeEdges(
  el: HTMLElement & { valueLow: number; valueHigh: number; pending?: boolean; invalid?: boolean },
  link: ViewLink,
  gateway: AlGuiGateway,
  target: string,
  controlId: string,
  opts: ScreenMountOptions,
): { lo: CommandBinding<unknown>; hi: CommandBinding<unknown>; unbind: Unbind } {
  const mk = (edge: "lo" | "hi"): CommandBinding<unknown> => {
    const binding = new CommandBinding<unknown>({
      target,
      controlId: `${controlId}-${edge}`,
      port: {
        dispatch: (i, c) => gateway.dispatchFor(i, c, (r) => binding.ack(r)),
      },
      ...(opts.flushWith !== undefined ? { flushWith: opts.flushWith } : {}),
      ...(opts.schedule !== undefined ? { schedule: opts.schedule } : {}),
      ...(opts.ackTimeoutMs !== undefined ? { ackTimeoutMs: opts.ackTimeoutMs } : {}),
    });
    link.registerBinding(binding);
    return binding;
  };
  const lo = mk("lo");
  const hi = mk("hi");

  const onGesture = (e: Event): void => {
    const detail = (e as CustomEvent).detail;
    // 両 binding が自分の controlId だけ拾う(他方は binding 内で無視)。
    lo.event(detail);
    hi.event(detail);
  };
  el.addEventListener("rvf-gesture", onGesture);

  // edge ごとの pending/invalid を保持して OR する — `applied` は
  // 終端 phase であり pending ではない。また片 edge の state 変化が
  // もう片 edge の in-flight 表示を消さないよう別々に追跡する。
  const flags = {
    lo: { pending: false, invalid: false },
    hi: { pending: false, invalid: false },
  };
  const reflect = (edge: "lo" | "hi", key: "low" | "high") => (s: BindingViewState<unknown>) => {
    flags[edge].pending = s.phase === "pending" || s.phase === "accepted";
    flags[edge].invalid = s.phase === "rejected" || s.phase === "timeout";
    el.pending = flags.lo.pending || flags.hi.pending;
    el.invalid = flags.lo.invalid || flags.hi.invalid;
    const v = s.lastObserved?.value;
    if (typeof v === "object" && v !== null && key in v) {
      const n = (v as Record<string, unknown>)[key];
      if (typeof n === "number" && Number.isFinite(n)) {
        if (key === "low") el.valueLow = n;
        else el.valueHigh = n;
      }
    }
  };
  const offLo = lo.addStateListener(reflect("lo", "low"));
  const offHi = hi.addStateListener(reflect("hi", "high"));

  let live = true;
  const unbind = (): void => {
    if (!live) return;
    live = false;
    el.removeEventListener("rvf-gesture", onGesture);
    offLo();
    offHi();
    lo.detach("unmount");
    hi.detach("unmount");
  };
  return { lo, hi, unbind };
}

/** 画面の全配線をまとめて解除するための registry。 */
export class Unbinds {
  private readonly list: (() => void)[] = [];
  add(f: (() => void) | undefined): void {
    if (f !== undefined) this.list.push(f);
  }
  run(): void {
    for (const f of this.list.splice(0)) f();
  }
}

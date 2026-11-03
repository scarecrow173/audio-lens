/**
 * Job settings 画面(#131)。
 *
 * `al.job.run`/`al.job.resume`/`al.job.cancel` に対応する構成:
 * - stage 有効化トグル(`rvf-toggle` × 全 stage → `al.cfg.stages.<name>`)
 * - analysis パラメータ入力(`rvf-numeric-input` → `al.cfg.analysis.*`)
 * - job inputs 宣言(`updateJobDraft` — 構造的入力、gesture ではない)
 * - Run / Resume / Cancel ボタン → `al.job` binding の action command
 *
 * Job 操作は gesture(commit/cancel)とは別経路の non-droppable action
 * command — receipt の accepted/applied/rejected が job ライフサイクル
 * の正直な表示状態を作る。
 */

import { CommandBinding } from "../../vendor/ruviform/ui/binding/src/index.ts";
import { registerRvfElements } from "../../vendor/ruviform/ui/components/src/index.ts";
import { type AlRunSpecParams, STAGE_NAMES } from "../ops.ts";
import {
  bindScalarControl,
  makeLink,
  mountScope,
  type ScreenFixture,
  type ScreenMountOptions,
  Unbinds,
} from "./shared.ts";

export interface JobSettingsFixture extends ScreenFixture {
  /** stage 名 → toggle element。 */
  readonly stageToggles: ReadonlyMap<string, HTMLElement>;
  /** Run/Resume/Cancel ボタン。 */
  readonly buttons: {
    run: HTMLButtonElement;
    resume: HTMLButtonElement;
    cancel: HTMLButtonElement;
  };
  /** `al.job` binding(action 発行 + job projection 観測)。 */
  readonly jobBinding: CommandBinding<unknown>;
  /** analysis params の numeric element 群。 */
  readonly params: ReadonlyMap<string, HTMLElement>;
  /** job draft の inputs を設定する(gesture ではなく構造入力)。 */
  setInputs(inputs: AlRunSpecParams["inputs"]): void;
}

const STFT_WINDOW_DESCRIPTOR = {
  min: 256,
  max: 16384,
  defaultValue: 2048,
  step: 256,
  unit: "samples",
} as const;

const LEVEL_WINDOW_DESCRIPTOR = {
  min: 0.005,
  max: 0.2,
  defaultValue: 0.02,
  step: 0.005,
  unit: "s",
} as const;

export function mountJobSettings(host: HTMLElement, opts: ScreenMountOptions): JobSettingsFixture {
  registerRvfElements();
  const { gateway } = opts;
  const { root, scope } = mountScope(host, opts.theme);
  const link = makeLink(opts, opts.viewId ?? "al.screen.job-settings");
  const unbinds = new Unbinds();

  // ── stage 有効化トグル ──────────────────────────────────────────
  const stageToggles = new Map<string, HTMLElement>();
  const togglesWrap = document.createElement("div");
  togglesWrap.dataset.part = "stages";
  // draft の stage 集合 seed は初回 mount のみ — remount/dock 移動で
  // commit 済みの「off 選択」を全 on へ戻さない。seed した時点で
  // 集合は「明示確立」扱い(空へ全 off したら `stages: []` を送る)。
  const seedStages = gateway.draft.stages.size === 0 && !gateway.draft.stagesEdited;
  for (const stage of STAGE_NAMES) {
    const label = document.createElement("label");
    label.dataset.stage = stage;
    const el = document.createElement("rvf-toggle") as HTMLElement & {
      controlId: string;
      target: string;
      checked: boolean;
    };
    el.controlId = `stage-${stage}`;
    el.target = `al.cfg.stages.${stage}`;
    el.checked = true; // 既定は全 stage 有効(pipeline 既定どおり)
    label.append(el, document.createTextNode(stage));
    togglesWrap.append(label);
    stageToggles.set(stage, el);
    // 初期 truth は「有効」(toggle checked と一致)— subscribe の
    // 初回 push が element の checked と齟齬しないよう宣言する。
    gateway.declareTarget(`al.cfg.stages.${stage}`, 1);
    const { unbind } = bindScalarControl(el, link, gateway, `al.cfg.stages.${stage}`, opts);
    unbinds.add(unbind);
    // draft の stage 集合を初期状態(全有効)に合わせる。
    if (seedStages) gateway.draft.stages.add(stage);
  }
  if (seedStages) gateway.draft.stagesEdited = true;
  scope.append(togglesWrap);

  // ── analysis params(numeric) ────────────────────────────────────
  const params = new Map<string, HTMLElement>();
  const paramsWrap = document.createElement("div");
  paramsWrap.dataset.part = "params";
  const paramDefs: readonly {
    key: string;
    descriptor: { min: number; max: number; defaultValue: number; step?: number; unit?: string };
  }[] = [
    { key: "analysis.cheap.stft_window", descriptor: STFT_WINDOW_DESCRIPTOR },
    { key: "analysis.cheap.level_window_s", descriptor: LEVEL_WINDOW_DESCRIPTOR },
  ];
  for (const { key, descriptor } of paramDefs) {
    const el = document.createElement("rvf-numeric-input") as HTMLElement & {
      controlId: string;
      target: string;
      descriptor: typeof descriptor;
      value: number;
    };
    el.controlId = `cfg-${key.replaceAll(".", "-")}`;
    el.target = `al.cfg.${key}`;
    el.descriptor = descriptor;
    el.value = descriptor.defaultValue;
    gateway.declareTarget(`al.cfg.${key}`, descriptor.defaultValue);
    // 既に commit 済みの値は remount で default に戻さない。
    if (!gateway.draft.config.has(key)) gateway.draft.config.set(key, descriptor.defaultValue);
    const { unbind } = bindScalarControl(el, link, gateway, `al.cfg.${key}`, opts);
    unbinds.add(unbind);
    paramsWrap.append(el);
    params.set(key, el);
  }
  scope.append(paramsWrap);

  // ── job actions ──────────────────────────────────────────────────
  const jobBinding = new CommandBinding<unknown>({
    target: "al.job",
    port: {
      dispatch: (i, c) => gateway.dispatchFor(i, c, (r) => jobBinding.ack(r)),
    },
    ...(opts.flushWith !== undefined ? { flushWith: opts.flushWith } : {}),
    ...(opts.schedule !== undefined ? { schedule: opts.schedule } : {}),
    ...(opts.ackTimeoutMs !== undefined ? { ackTimeoutMs: opts.ackTimeoutMs } : {}),
  });
  link.registerBinding(jobBinding);

  const btnWrap = document.createElement("div");
  btnWrap.dataset.part = "job-actions";
  const makeBtn = (id: string, text: string): HTMLButtonElement => {
    const b = document.createElement("button");
    b.type = "button";
    b.dataset.action = id;
    b.textContent = text;
    btnWrap.append(b);
    return b;
  };
  const runBtn = makeBtn("run", "Run");
  const resumeBtn = makeBtn("resume", "Resume");
  const cancelBtn = makeBtn("cancel", "Cancel");
  const onRun = (): void => jobBinding.action("run", { controlId: "job.run" });
  const onResume = (): void => jobBinding.action("resume", { controlId: "job.resume" });
  const onCancel = (): void => jobBinding.action("cancel", { controlId: "job.cancel" });
  runBtn.addEventListener("click", onRun);
  resumeBtn.addEventListener("click", onResume);
  cancelBtn.addEventListener("click", onCancel);
  unbinds.add(() => {
    runBtn.removeEventListener("click", onRun);
    resumeBtn.removeEventListener("click", onResume);
    cancelBtn.removeEventListener("click", onCancel);
  });
  scope.append(btnWrap);

  link.attach(opts.windowId ?? "main");

  return {
    root,
    scope,
    link,
    stageToggles,
    buttons: { run: runBtn, resume: resumeBtn, cancel: cancelBtn },
    jobBinding,
    params,
    setInputs: (inputs) => {
      gateway.draft.inputs = [...inputs];
    },
    unmount: () => {
      unbinds.run();
      link.detach("unmount");
      root.remove();
    },
  };
}

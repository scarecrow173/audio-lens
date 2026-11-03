/**
 * revision 付き view 状態ストア(#131)。
 *
 * Command/Query binding の projection 経路が要求する「authoritative
 * truth + 単調 revision」の最小実装。`al.view.*`(seek/range/channel)
 * と `al.cfg.*`(job draft config)は UI 側が truth を持つ view 状態 —
 * commit だけが revision を進め、preview/cancel は状態を変えない。
 * `subscribe` は即座に現在値を push する(reattach で古い表示を
 * 信頼しないための契約)。
 */

import type { StateProjection } from "../vendor/ruviform/ui/binding/src/index.ts";

export class RevisionedStore<P> {
  private value_: P;
  private revision_ = 0;
  private readonly listeners = new Set<(p: StateProjection<P>) => void>();

  constructor(initial: P) {
    this.value_ = initial;
  }

  get revision(): number {
    return this.revision_;
  }

  get value(): P {
    return this.value_;
  }

  /** 確定値の書き込み — revision を進めて全 listener に通知する。 */
  commit(value: P): number {
    this.value_ = value;
    this.revision_ += 1;
    this.publish();
    return this.revision_;
  }

  /** listener を登録し、現在 projection を即 push する。 */
  subscribe(listener: (p: StateProjection<P>) => void): () => void {
    this.listeners.add(listener);
    listener({ revision: this.revision_, value: this.value_ });
    return () => this.listeners.delete(listener);
  }

  private publish(): void {
    const p: StateProjection<P> = {
      revision: this.revision_,
      value: this.value_,
    };
    for (const fn of [...this.listeners]) fn(p);
  }
}

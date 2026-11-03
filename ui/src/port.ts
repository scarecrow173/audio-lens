/**
 * AudioLens サービス port(#131)。
 *
 * `AlGuiService`(crates/al-gui/src/service.rs)の op 呼出を、UI 層が
 * 依存する最小の形に写したインターフェース。実 WebView 経路では
 * `ruviform_ui::ipc` の dispatch/job チャンネルへ橋渡しし、headless
 * 検証では `test/fakes.ts` の FakeAlService が実装する。どちらの実装も
 * 同じ op 名・params・結果 JSON・エラー分類を持つ — この層は
 * AlGuiService の契約を越える語彙を新設しない。
 *
 * - `call` は一回応答の op(`al.ds.*`・`al.view.*`・`al.job.list/inspect`)。
 * - `job` は長時間 op(`al.job.run`/`al.job.resume`)で、rvf `CallOutcome::Job`
 *   に相当する進捗・キャンセル付きハンドルを返す。
 */

/** op 失敗のエラー本体(`ruviform_ui::ipc::ErrorBody` 相当)。 */
export interface AlServiceError {
  /** IPC エラーコード(`codes::INVALID_PARAMS`/`CANCELLED` または
   *  AudioLens 分類 `"not_found"`/`"io"`/`"stage"` 等)。 */
  readonly code: string;
  readonly message: string;
  /** 失敗に付随する構造化情報(任意)。 */
  readonly data?: unknown;
}

export function isServiceError(e: unknown): e is AlServiceError {
  return (
    typeof e === "object" &&
    e !== null &&
    typeof (e as AlServiceError).code === "string" &&
    typeof (e as AlServiceError).message === "string"
  );
}

/**
 * 長時間 job ハンドル(`CallOutcome::Job` 相当)。
 * `done` は `al.job.*` op の結果 JSON(`{job_id, stages}`)で resolve し、
 * 失敗・キャンセル時は {@link AlServiceError} で reject する。
 * cancel された run の manifest は service 側で残存するが、op は
 * job_id を返さない — 再開には `al.job.list` で再発見する。
 */
export interface AlJobHandle {
  /** IPC 側の job ハンドル識別子(cancel の対象)。 */
  readonly id: string;
  /** stage 進捗通知(done/total — `StageEvent` index ベース)。 */
  onProgress(cb: (done: number, total: number) => void): void;
  /** 実行中断を要求する(`ctx.is_cancelled` 経路)。 */
  cancel(): void;
  readonly done: Promise<unknown>;
}

/** サービス port 本体。 */
export interface AlServicePort {
  /** 一回応答 op の呼出。失敗は {@link AlServiceError} で reject。 */
  call(op: string, params?: Record<string, unknown>): Promise<unknown>;
  /** 長時間 job op の起動。 */
  job(op: "al.job.run" | "al.job.resume", params: Record<string, unknown>): AlJobHandle;
  /**
   * `rvf://data` 相当の範囲 pull(#132)。`al.view.*` op が返す
   * `{source, bytes, meta}` の `source` 名を指定し、`offset` から最大
   * `limit` バイトを返す — 実 WebView 経路では `rvf://data` の
   * range GET(`DataSource::read`)に対応する。short read(EOF)を許し、
   * `limit` を超える応答は契約違反として扱う。
   */
  pull(source: string, offset: number, limit: number): Promise<Uint8Array>;
  /** query 用の明示キャンセル経路(ホストが持つ場合のみ)。 */
  cancelQuery?(queryId: string): void;
}

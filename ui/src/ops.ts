/**
 * AudioLens op 名と語彙(#131)。
 *
 * `AlGuiService::OPS`(crates/al-gui/src/service.rs)と 1:1 に対応する。
 * op の追加・改名は Rust 側を正本としてここへ転記する — 逆方向には
 * 新しい語彙を作らない。stage 名と input role は `al-core` の serde
 * snake_case 表現と一致する。
 */

/** `AlGuiService::OPS` の op 名定数。 */
export const AL_OPS = {
  appInfo: "al.app.info",
  appConfigure: "al.app.configure",
  jobList: "al.job.list",
  jobInspect: "al.job.inspect",
  jobRun: "al.job.run",
  jobResume: "al.job.resume",
  dsOpen: "al.ds.open",
  dsClose: "al.ds.close",
  dsOverview: "al.ds.overview",
  dsEvents: "al.ds.events",
  dsLocate: "al.ds.locate",
  dsLocateDetail: "al.ds.locate_detail",
  dsFeatures: "al.ds.features",
  dsArtifacts: "al.ds.artifacts",
  dsArtifact: "al.ds.artifact",
  dsRead: "al.ds.read",
  viewWaveform: "al.view.waveform",
  viewSpectrogram: "al.view.spectrogram",
} as const;

export type AlOp = (typeof AL_OPS)[keyof typeof AL_OPS];

/** `StageKind` の serde snake_case(pipeline order)。 */
export const STAGE_NAMES = [
  "ingest",
  "locate",
  "cheap_analysis",
  "reference_analysis",
  "expensive_analysis",
  "dataset",
  "export",
] as const;
export type StageName = (typeof STAGE_NAMES)[number];

/** `InputRole` の serde snake_case。 */
export const INPUT_ROLES = ["capture_bug", "capture_reference", "source_asset", "other"] as const;
export type InputRole = (typeof INPUT_ROLES)[number];

/** `al.job.run`/`al.job.resume` が返す job 要約(JSON)。 */
export interface AlJobResult {
  readonly job_id: string;
  readonly stages: readonly {
    readonly stage: StageName | (string & {});
    readonly status: string;
    readonly error?: string | null;
  }[];
}

/** `al.job.run` へ渡す RunSpec 相当の op params。 */
export interface AlRunSpecParams {
  readonly inputs: readonly {
    readonly role: InputRole | (string & {});
    readonly path: string;
    readonly input_id?: string;
  }[];
  readonly stages?: readonly StageName[] | null;
  readonly config?: Record<string, unknown>;
}

/** `al.ds.open` 結果(`{session, root}` — service.rs 実装と一致)。 */
export interface AlDsOpenResult {
  readonly session: number;
  readonly root: string;
}

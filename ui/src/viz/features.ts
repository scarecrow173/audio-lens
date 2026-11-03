/**
 * Feature 閲覧モデル(#132) — `al.ds.features`/`al.ds.artifacts`/
 * `al.ds.artifact` の薄い adapter。`FeatureRecord` の
 * `validity`/`confidence`/`support_count`/`lineage` 等は record の
 * verbatim JSON をそのまま保持し、AudioLens 側で診断結論を生成しない
 * (検索・詳細解決・Spectrum 用の magnitude 供給のみ行う)。
 */

import {
  type BinAxis,
  ManualSnapshotSource,
  type SourceMeta,
  type SpectrumDataSource,
  type SpectrumFrame,
  type SpectrumInfo,
} from "../../vendor/ruviform/ui/viz/src/index.ts";
import { AL_OPS } from "../ops.ts";
import type { AlServicePort } from "../port.ts";

/** `al.ds.features` 応答の検索条件(service 側 FeatureQuery と同じ語彙)。 */
export interface FeatureSearchQuery {
  readonly roles?: readonly string[];
  readonly namePrefix?: string;
  readonly featureIdPrefix?: string;
  readonly limit?: number;
}

/** `al.ds.features` 応答の verbatim 化(record 群 + truncated 等)。 */
export interface FeatureSearchResult {
  readonly records: readonly Record<string, unknown>[];
  readonly truncated: boolean;
  readonly limit?: number;
}

function num(v: unknown): number | undefined {
  return typeof v === "number" && Number.isFinite(v) ? v : undefined;
}
function str(v: unknown): string | undefined {
  return typeof v === "string" ? v : undefined;
}
function isRecord(v: unknown): v is Record<string, unknown> {
  return typeof v === "object" && v !== null;
}

/**
 * Feature の検索 + Inspector 詳細解決。`record` は service が返した
 * JSON をそのまま保持する(`_artifact` path は service が付与)。
 */
export class FeatureCatalog {
  private readonly port: AlServicePort;
  private readonly session: number;
  constructor(port: AlServicePort, session: number) {
    this.port = port;
    this.session = session;
  }

  /** `al.ds.features` — record は verbatim(truncated も保持)。 */
  async search(q: FeatureSearchQuery = {}): Promise<FeatureSearchResult> {
    const r = await this.port.call(AL_OPS.dsFeatures, {
      session: this.session,
      ...(q.roles !== undefined ? { roles: [...q.roles] } : {}),
      ...(q.namePrefix !== undefined ? { name_prefix: q.namePrefix } : {}),
      ...(q.featureIdPrefix !== undefined ? { feature_id_prefix: q.featureIdPrefix } : {}),
      ...(q.limit !== undefined ? { limit: q.limit } : {}),
    });
    if (!isRecord(r)) return { records: [], truncated: false };
    const records = Array.isArray(r.records) ? r.records.filter(isRecord) : [];
    const limit = num(r.limit);
    return {
      records,
      truncated: r.truncated === true,
      ...(limit !== undefined ? { limit } : {}),
    };
  }

  /** `al.ds.artifacts` — provenance 付き artifact 一覧(verbatim)。 */
  async artifacts(role?: string): Promise<readonly Record<string, unknown>[]> {
    const r = await this.port.call(AL_OPS.dsArtifacts, {
      session: this.session,
      ...(role !== undefined ? { role } : {}),
    });
    if (!isRecord(r) || !Array.isArray(r.artifacts)) return [];
    return r.artifacts.filter(isRecord);
  }

  /** `al.ds.artifact` — `<role>/<name>` の詳細(verbatim)。 */
  async artifactDetail(path: string): Promise<unknown> {
    return this.port.call(AL_OPS.dsArtifact, { session: this.session, path });
  }

  /**
   * record の Inspector 詳細:record(verbatim)+ record が載る
   * artifact(`_artifact` path)の `al.ds.artifact` 詳細 + その
   * provenance。`_artifact` の無い record は `undefined` を返す
   * (detail 経路が無い record を偽造しない)。
   */
  async inspect(record: Record<string, unknown>): Promise<
    | {
        record: Record<string, unknown>;
        artifactPath: string;
        detail: unknown;
      }
    | undefined
  > {
    const path = str(record._artifact);
    if (path === undefined) return undefined;
    const detail = await this.artifactDetail(path);
    return { record, artifactPath: path, detail };
  }
}

/** feature record の magnitudes 供給用 Spectrum port。 */
class FeatureSpectrumSource implements SpectrumDataSource {
  private readonly inner: ManualSnapshotSource<SpectrumFrame>;
  private readonly info: SpectrumInfo;
  constructor(meta: SourceMeta, info: SpectrumInfo) {
    this.inner = new ManualSnapshotSource<SpectrumFrame>(meta);
    this.info = info;
  }
  meta(): SourceMeta & SpectrumInfo {
    return { ...this.inner.meta(), ...this.info };
  }
  subscribe(onUpdate: () => void): () => void {
    return this.inner.subscribe(onUpdate);
  }
  snapshot(): SpectrumFrame | null {
    return this.inner.snapshot();
  }
  push(f: SpectrumFrame): void {
    this.inner.push(f);
  }
}

/**
 * feature record の数値列(`value`/`values` が number[])を
 * `SpectrumDataSource` の 1 frame として供給する。欠測(NaN や
 * non-finite)はそのまま残す。record が数値列を持たなければ
 * `undefined`(feature を spectrum として描ける truth が無い)。
 *
 * `axis` は必須 — vendored `SpectrumInfo.axis` は Hz 型のみで、
 * bin index を `hz` へ流すと非周波数軸を Hz と偽ることになる。
 * 周波数軸の truth が無い record に spectrum を捏造しない
 * (非 Hz 軸は Ruviform `BinAxis` 側の責務 — 要 upstream 拡張)。
 */
export function featureSpectrumSource(
  record: Record<string, unknown>,
  axis: BinAxis,
): SpectrumDataSource | undefined {
  const v = isRecord(record)
    ? Array.isArray(record.value)
      ? record.value
      : record.values
    : undefined;
  if (!Array.isArray(v) || v.length === 0 || !v.every((x) => typeof x === "number")) {
    return undefined;
  }
  if (axis === undefined) return undefined; // non-TS 呼出も黙って描かない
  const bins = v.length;
  const src = new FeatureSpectrumSource(
    { id: str(record.feature_id) ?? "feature", revision: 0 },
    { bins, axis },
  );
  src.push({ sequence: 1, magnitudes: [...v] });
  return src;
}

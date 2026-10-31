//! 永続 Analysis Dataset への query port。GUI・CLI が共有する
//! 「読む側」の入口。全ての読み出しは `DatasetReader` の range
//! API 経由で、dataset 全体の全量ロードをしない。

use std::path::{Path, PathBuf};
use std::sync::Arc;

use al_dataset::manifest::{DatasetArtifact, StoreKind};
use al_dataset::reader::DatasetReader;
use al_dataset::table::Cell;
use ruviform_viz::source::{SpectrumSource, WaveformSource};
use serde_json::{Value, json};

use crate::error::ServiceError;
use crate::source::{DatasetSpectrum, DatasetWaveform};

/// features 検索条件。`None` は無制限。
#[derive(Debug, Clone, Default)]
pub struct FeatureQuery {
    /// 検索対象の feature role(`features/cheap` 等)。`None` で
    /// `features/` プレフィックスを持つ全 role。
    pub roles: Option<Vec<String>>,
    /// artifact 名の input_id / pair フィルタ(前方一致)。
    pub name_prefix: Option<String>,
    /// `feature_id` の前方一致フィルタ。
    pub feature_id_prefix: Option<String>,
    /// 返す最大 record 数(超過時 `truncated: true`)。
    pub limit: Option<usize>,
}

/// 開いている dataset への typed query セッション。
pub struct DatasetSession {
    root: PathBuf,
    reader: Arc<DatasetReader>,
}

impl DatasetSession {
    /// `root` が dataset ルート(`manifest.json` を持つ)として開く。
    /// schema 不一致・破損 manifest は `DatasetReader::open` が
    /// 明示的に失敗させる。
    pub fn open(root: impl AsRef<Path>) -> Result<Self, ServiceError> {
        let root = root.as_ref().to_path_buf();
        let reader = DatasetReader::open(&root).map_err(|e| {
            if e.kind() == std::io::ErrorKind::NotFound {
                ServiceError::not_found(format!("no dataset at {}", root.display()))
            } else {
                ServiceError::from(e)
            }
        })?;
        Ok(Self {
            root,
            reader: Arc::new(reader),
        })
    }

    /// dataset ルート。
    pub fn root(&self) -> &Path {
        &self.root
    }

    /// 内部 reader(source 構築・低レベル read 用)。
    pub fn reader(&self) -> &Arc<DatasetReader> {
        &self.reader
    }

    /// dataset 概要: 入力・role 状態・artifact 総数・config 概要。
    pub fn overview(&self) -> Result<Value, ServiceError> {
        let manifest = self.reader.manifest();
        let roles: serde_json::Map<String, Value> = manifest
            .roles
            .iter()
            .map(|(name, entry)| {
                (
                    name.clone(),
                    json!({
                        "status": entry.status,
                        "artifact_count": entry.artifact_count,
                    }),
                )
            })
            .collect();
        Ok(json!({
            "root": self.root.display().to_string(),
            "schema_version": manifest.schema_version,
            "audio_format": manifest.audio_format,
            "inputs": manifest.inputs,
            "roles": Value::Object(roles),
            "artifact_count": manifest.artifacts.len(),
        }))
    }

    /// タイムライン基礎情報: 入力別 audio 情報(frames・rate・
    /// channels・duration)とイベント/locate 件数。波形の時間軸は
    /// 最初に audio を持つ入力の sample_rate を使う。
    pub fn timeline(&self) -> Result<Value, ServiceError> {
        let mut inputs = Vec::new();
        for artifact in self.reader.artifacts("inputs") {
            if artifact.store != StoreKind::Audio {
                continue;
            }
            let name = artifact
                .path
                .strip_prefix("inputs/")
                .unwrap_or(&artifact.path)
                .trim_end_matches(".wav")
                .to_string();
            let frames = self.reader.audio_frames("inputs", &name).unwrap_or(0);
            let meta = self
                .reader
                .read_json("inputs", &format!("{name}.metadata"))
                .unwrap_or(Value::Null);
            let audio = &meta["audio"];
            let sample_rate = audio["sample_rate"].as_u64().unwrap_or(0);
            inputs.push(json!({
                "input_id": name,
                "frames": frames,
                "sample_rate": sample_rate,
                "channels": audio["channels"].as_u64().unwrap_or(0),
                "duration_s": if sample_rate > 0 { frames as f64 / sample_rate as f64 } else { 0.0 },
            }));
        }
        let event_count = self
            .reader
            .open_table("events", "candidates")
            .map(|t| t.row_count())
            .unwrap_or(0);
        let locate_count = self
            .reader
            .open_table("locate", "summary")
            .map(|t| t.row_count())
            .unwrap_or(0);
        Ok(json!({
            "inputs": inputs,
            "event_count": event_count,
            "locate_count": locate_count,
        }))
    }

    /// `events/candidates` を行範囲 + 任意の時間範囲で読む。
    /// 各行を `{列名: 値}` の JSON として返す(測定の意味を保存する
    /// ため列の丸め・再解釈はしない)。
    pub fn events(
        &self,
        row_start: u64,
        row_end: u64,
        start_s: Option<f64>,
        end_s: Option<f64>,
    ) -> Result<Value, ServiceError> {
        let table = self
            .reader
            .open_table("events", "candidates")
            .map_err(|e| {
                if e.kind() == std::io::ErrorKind::InvalidData {
                    ServiceError::not_found("events/candidates is not produced")
                } else {
                    ServiceError::from(e)
                }
            })?;
        let total = table.row_count();
        let end = row_end.min(total).max(row_start);
        let cols = table.read_range(row_start.min(end), end, None)?;
        let schema = table.schema();
        let mut rows = Vec::new();
        for i in 0..cols.first().map_or(0, Vec::len) {
            let row = row_to_json(schema, &cols, i);
            if let (Some(s), Some(e)) = (start_s, row["start_s"].as_f64())
                && e < s
            {
                continue;
            }
            if let (Some(e), Some(s)) = (end_s, row["start_s"].as_f64())
                && s >= e
            {
                continue;
            }
            rows.push(row);
        }
        Ok(json!({
            "row_count": total,
            "rows": rows,
        }))
    }

    /// `locate/summary` テーブル + `locate/` JSON artifact 一覧。
    pub fn locate(&self) -> Result<Value, ServiceError> {
        let mut alignments = Vec::new();
        if let Ok(table) = self.reader.open_table("locate", "summary") {
            let cols = table.read_range(0, table.row_count(), None)?;
            let schema = table.schema();
            for i in 0..cols.first().map_or(0, Vec::len) {
                alignments.push(row_to_json(schema, &cols, i));
            }
        }
        let details: Vec<Value> = self
            .reader
            .artifacts("locate")
            .into_iter()
            .filter(|a| a.store == StoreKind::Json)
            .map(|a| json!({"path": a.path, "provenance": a.provenance}))
            .collect();
        Ok(json!({
            "alignments": alignments,
            "detail_paths": details,
        }))
    }

    /// `locate/<src>/<cap>.json` の詳細(result + evidence)。
    pub fn locate_detail(&self, source: &str, capture: &str) -> Result<Value, ServiceError> {
        self.reader
            .read_json("locate", &format!("{source}/{capture}"))
            .map_err(ServiceError::from)
    }

    /// `features/**` の JSON artifact から `FeatureRecord` を検索。
    /// record は素の JSON をそのまま返す(表現ドリフト防止)。
    pub fn features(&self, query: &FeatureQuery) -> Result<Value, ServiceError> {
        let limit = query.limit.unwrap_or(500);
        let mut records = Vec::new();
        let mut truncated = false;
        'outer: for artifact in self.feature_artifacts(query) {
            let name = artifact
                .path
                .strip_prefix(&format!("{}/", artifact.role))
                .unwrap_or(&artifact.path)
                .trim_end_matches(".json")
                .to_string();
            if let Some(prefix) = &query.name_prefix
                && !name.starts_with(prefix.as_str())
            {
                continue;
            }
            let body = match self.reader.read_json(&artifact.role, &name) {
                Ok(body) => body,
                Err(_) => continue, // 壊れた artifact は検索対象外
            };
            let list = body["records"].as_array().cloned().unwrap_or_default();
            for mut record in list {
                if let Some(prefix) = &query.feature_id_prefix {
                    let id = record["feature_id"].as_str().unwrap_or("");
                    if !id.starts_with(prefix.as_str()) {
                        continue;
                    }
                }
                record["_artifact"] = json!(artifact.path);
                records.push(record);
                if records.len() >= limit {
                    truncated = true;
                    break 'outer;
                }
            }
        }
        Ok(json!({
            "records": records,
            "truncated": truncated,
            "limit": limit,
        }))
    }

    /// feature artifact 一覧を role フィルタ付きで返す。
    fn feature_artifacts(&self, query: &FeatureQuery) -> Vec<&DatasetArtifact> {
        self.reader
            .manifest()
            .artifacts
            .iter()
            .filter(|a| {
                a.store == StoreKind::Json
                    && a.role.starts_with("features/")
                    && query
                        .roles
                        .as_ref()
                        .is_none_or(|roles| roles.contains(&a.role))
            })
            .collect()
    }

    /// 全 artifact を role フィルタ付きで列挙(provenance 付き)。
    pub fn artifacts(&self, role: Option<&str>) -> Vec<Value> {
        self.reader
            .manifest()
            .artifacts
            .iter()
            .filter(|a| role.is_none_or(|r| a.role == r))
            .map(|a| serde_json::to_value(a).unwrap_or(Value::Null))
            .collect()
    }

    /// 1 artifact の詳細: store 種別に応じた schema/shape/meta +
    /// provenance。`path` は `<role>/<name>` 形式。
    pub fn artifact_detail(&self, path: &str) -> Result<Value, ServiceError> {
        let artifact = self
            .reader
            .manifest()
            .artifacts
            .iter()
            .find(|a| a.path == path)
            .ok_or_else(|| ServiceError::not_found(format!("no artifact {path}")))?;
        // role は `features/cheap` のように `/` を含み得るため、
        // manifest 側の role を正本として name を剥がす。
        let role = artifact.role.as_str();
        let name = artifact
            .path
            .strip_prefix(&format!("{role}/"))
            .ok_or_else(|| {
                ServiceError::invalid(format!(
                    "artifact path {path} does not start with role {role}"
                ))
            })?;
        let store = match artifact.store {
            StoreKind::Table => {
                let t = self.reader.open_table(role, name)?;
                json!({
                    "kind": "table",
                    "schema": t.schema(),
                    "row_count": t.row_count(),
                })
            }
            StoreKind::Tensor => {
                let t = self.reader.open_tensor(role, name)?;
                json!({
                    "kind": "tensor",
                    "shape": t.meta().shape,
                    "chunk_shape": t.meta().chunk_shape,
                })
            }
            StoreKind::Audio => json!({
                "kind": "audio",
                "frames": self.reader.audio_frames(role, name)?,
            }),
            StoreKind::Json => json!({"kind": "json"}),
        };
        Ok(json!({
            "artifact": artifact,
            "store": store,
        }))
    }

    /// table の行範囲を `{列名: 値}` JSON 配列として読む。
    /// `columns` は列 index 選択。
    pub fn table_rows(
        &self,
        role: &str,
        name: &str,
        row_start: u64,
        row_end: u64,
        columns: Option<&[usize]>,
    ) -> Result<Value, ServiceError> {
        let table = self.reader.open_table(role, name)?;
        let cols = table.read_range(row_start, row_end, columns)?;
        let schema = table.schema();
        let selected: Vec<usize> = columns
            .map(|c| c.to_vec())
            .unwrap_or_else(|| (0..schema.len()).collect());
        let mut rows = Vec::new();
        // 行 index は複数列 vec をまたいで使うため range loop が自然
        // (al-locate の mixer と同じ判断)。
        #[allow(clippy::needless_range_loop)]
        for i in 0..cols.first().map_or(0, Vec::len) {
            let mut obj = serde_json::Map::new();
            for (out_col, &schema_col) in selected.iter().enumerate() {
                obj.insert(
                    schema[schema_col].name.clone(),
                    cell_to_json(&cols[out_col][i]),
                );
            }
            rows.push(Value::Object(obj));
        }
        Ok(json!({
            "row_count": table.row_count(),
            "rows": rows,
        }))
    }

    /// tensor 領域を読む。`region` は dim 毎の `[start, end)`。
    /// 読み出し要素数を `max_elements` で制限する(表示用有界 read)。
    pub fn tensor_region(
        &self,
        role: &str,
        name: &str,
        region: &[[u64; 2]],
        max_elements: u64,
    ) -> Result<Value, ServiceError> {
        let tensor = self.reader.open_tensor(role, name)?;
        let elements: u64 = region.iter().map(|r| r[1].saturating_sub(r[0])).product();
        if elements > max_elements {
            return Err(ServiceError::invalid(format!(
                "region has {elements} elements (max {max_elements}); narrow the range"
            )));
        }
        let (values, shape) = tensor.read_range(region)?;
        Ok(json!({
            "shape": shape,
            "values": values,
        }))
    }

    /// 汎用 JSON artifact read(`<role>/<name>`)。
    pub fn json_artifact(&self, role: &str, name: &str) -> Result<Value, ServiceError> {
        self.reader
            .read_json(role, name)
            .map_err(ServiceError::from)
    }

    /// `inputs/<input_id>` の波形 source を開く。
    pub fn waveform(&self, input_id: &str) -> Result<DatasetWaveform, ServiceError> {
        DatasetWaveform::open(Arc::clone(&self.reader), input_id).map_err(ServiceError::from)
    }

    /// `intermediate/<input_id>/ch<channel>` のスペクトル source を開く。
    pub fn spectrum(
        &self,
        input_id: &str,
        channel: usize,
    ) -> Result<DatasetSpectrum, ServiceError> {
        DatasetSpectrum::open(Arc::clone(&self.reader), input_id, channel)
            .map_err(ServiceError::from)
    }

    /// 波形ビューを `bins` 解像度で計算し `RVBW` エンコードして返す。
    /// `start_s`/`end_s` は秒。`WaveformSeries::on_demand` で表示に
    /// 必要な範囲のみ読む。
    pub fn waveform_view(
        &self,
        input_id: &str,
        channel: usize,
        start_s: f64,
        end_s: f64,
        bins: usize,
    ) -> Result<(ruviform_viz::LodBytes, Value), ServiceError> {
        let source = Arc::new(self.waveform(input_id)?);
        let info = source.info();
        if channel >= info.channels {
            return Err(ServiceError::invalid(format!(
                "channel {channel} out of {}",
                info.channels
            )));
        }
        let series = ruviform_viz::WaveformSeries::on_demand(source.clone());
        let rate = info.sample_rate as f64;
        let start = (start_s.max(0.0) * rate) as u64;
        let end = ((end_s * rate) as u64).min(info.frames).max(start);
        let view = series
            .view(channel, start, end.max(start + 1), bins.max(1))
            .map_err(|e| ServiceError::invalid(e.to_string()))?;
        let lod = ruviform_viz::encode_buckets(&view);
        let meta = json!({
            "input_id": input_id,
            "channel": channel,
            "sample_rate": info.sample_rate,
            "start": lod.start,
            "end": lod.end,
            "bins": lod.bins,
            "format": "RVBW",
        });
        Ok((lod, meta))
    }

    /// スペクトログラムタイルを `RVST` エンコードして返す。
    /// `frames` は source frame 範囲、`rows`/`cols` は出力解像度。
    /// 表示正規化(dB range)は `db_min`/`db_max` で指定し、
    /// `Quantization::F32` で magnitude をそのまま送る(フロントで
    /// colormap 適用)。
    pub fn spectrogram_tile(
        &self,
        input_id: &str,
        channel: usize,
        frames: std::ops::Range<u64>,
        rows: usize,
        cols: usize,
    ) -> Result<(ruviform_viz::Bytes, Value), ServiceError> {
        let source = Arc::new(self.spectrum(input_id, channel)?);
        let info = source.info();
        let spec = ruviform_viz::Spectrogram::new(source.clone());
        let tile = spec
            .tile(&ruviform_viz::TileRequest {
                frames,
                rows: ruviform_viz::RowSpec::uniform(info.bins, rows),
                cols: cols.max(1),
                aggregate: ruviform_viz::Aggregate::Max,
            })
            .map_err(|e| ServiceError::invalid(e.to_string()))?;
        let bytes = ruviform_viz::encode_tile(&tile, ruviform_viz::Quantization::F32);
        let meta = json!({
            "input_id": input_id,
            "channel": channel,
            "cols": tile.cols,
            "rows": tile.rows,
            "frame_start": tile.frames.start,
            "frame_end": tile.frames.end,
            "sample_rate": info.sample_rate,
            "hop": info.grid.hop,
            "window": info.grid.window,
            "bins": info.bins,
            "format": "RVST-f32",
        });
        Ok((bytes, meta))
    }
}

/// `Cell` を JSON 値へ。`Utf8` に格納された JSON 文字列はそのまま
/// 文字列として返す(再解釈しない)。
fn cell_to_json(cell: &Cell) -> Value {
    match cell {
        Cell::Null => Value::Null,
        Cell::I64(v) => json!(v),
        Cell::F64(v) => json!(v),
        Cell::Str(s) => json!(s),
    }
}

/// table 行を `{列名: 値}` の JSON object へ。
fn row_to_json(schema: &[al_dataset::table::Column], cols: &[Vec<Cell>], index: usize) -> Value {
    let mut obj = serde_json::Map::new();
    for (c, col) in schema.iter().enumerate() {
        obj.insert(col.name.clone(), cell_to_json(&cols[c][index]));
    }
    Value::Object(obj)
}

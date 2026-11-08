//! `AlGuiService`: `rvf.ipc.v1` 上の AudioLens op 実装。
//! op → `al-service` 呼出の薄い adapter で、永続・解析・view 計算の
//! ロジックは持たない(設計: `docs/designs/gui-app-architecture.md`)。

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;

use al_service::dataset::{DatasetSession, FeatureQuery};
use al_service::jobs::{JobService, RunControl, RunSpec};
use al_service::{ServiceError, parse_input_spec, parse_stage};
use ruviform_ui::VecDataSource;
use ruviform_ui::ipc::{ErrorBody, codes};
use ruviform_ui::service::{CallOutcome, JobSpec, OpInfo, Service, ServiceContext};
use ruviform_viz::source::SpectrumSource;
use serde_json::{Value, json};

use crate::command;
use crate::view::desktop::TAB_COUNT;

const OPS: &[OpInfo] = &[
    OpInfo {
        name: "al.app.info",
        summary: "GUI/service 情報(backend・既定 base_dir)",
    },
    OpInfo {
        name: "al.app.configure",
        summary: "既定 base_dir を設定する",
    },
    OpInfo {
        name: "al.job.list",
        summary: "base_dir 配下の job 一覧",
    },
    OpInfo {
        name: "al.job.inspect",
        summary: "job manifest + dataset 状態",
    },
    OpInfo {
        name: "al.job.run",
        summary: "新規 job 作成・全 stage 実行(キャンセル可)",
    },
    OpInfo {
        name: "al.job.resume",
        summary: "既存 job の再開(キャンセル可)",
    },
    OpInfo {
        name: "al.ds.open",
        summary: "dataset root または job の analysis/ を開く",
    },
    OpInfo {
        name: "al.ds.close",
        summary: "dataset session を閉じる",
    },
    OpInfo {
        name: "al.ds.overview",
        summary: "dataset 概要 + timeline",
    },
    OpInfo {
        name: "al.ds.events",
        summary: "events/candidates の範囲読み出し",
    },
    OpInfo {
        name: "al.ds.locate",
        summary: "locate summary + detail paths",
    },
    OpInfo {
        name: "al.ds.locate_detail",
        summary: "locate/<src>/<cap>.json の詳細",
    },
    OpInfo {
        name: "al.ds.features",
        summary: "FeatureRecord 検索(feature_id/input/role)",
    },
    OpInfo {
        name: "al.ds.artifacts",
        summary: "dataset artifact 一覧(role フィルタ可)",
    },
    OpInfo {
        name: "al.ds.artifact",
        summary: "artifact 詳細(schema/shape/provenance)",
    },
    OpInfo {
        name: "al.ds.read",
        summary: "table/tensor/json/audio の汎用 range 読み出し",
    },
    OpInfo {
        name: "al.view.waveform",
        summary: "波形 view を RVBW で発行する",
    },
    OpInfo {
        name: "al.view.spectrogram",
        summary: "スペクトログラムタイルを RVST で発行する",
    },
    OpInfo {
        name: command::COMMAND_INTENT_OP,
        summary: "view の command intent(job 選択・再開・tab 選択)を解釈する",
    },
];

/// AudioLens GUI サービス。`al-service` の JobService/DatasetSession
/// を op 経路へ橋渡しする。dataset session は `ds.open` が発行する
/// ハンドルで管理し、payload は `ctx.data()` の pull path へ載せる。
pub struct AlGuiService {
    /// `al.app.configure` またはコンストラクタで設定した既定
    /// base_dir。各 op の `base_dir` パラメータが優先される。
    base_dir: Option<PathBuf>,
    sessions: HashMap<u64, DatasetSession>,
    next_session: u64,
    next_payload: u64,
}

impl AlGuiService {
    /// 既定 base_dir なしで作る(`al.app.configure` か各 op の
    /// `base_dir` パラメータで後から指定する)。
    pub fn new() -> Self {
        Self {
            base_dir: None,
            sessions: HashMap::new(),
            next_session: 0,
            next_payload: 0,
        }
    }

    /// 既定 base_dir を最初から持つサービスを作る。
    pub fn with_base_dir(base_dir: impl Into<PathBuf>) -> Self {
        let mut service = Self::new();
        service.base_dir = Some(base_dir.into());
        service
    }

    /// op パラメータまたは既定から JobService を解決する。
    fn job_service(&self, params: &Value) -> Result<JobService, ErrorBody> {
        let base_dir = params
            .get("base_dir")
            .and_then(Value::as_str)
            .map(PathBuf::from)
            .or_else(|| self.base_dir.clone());
        let Some(base_dir) = base_dir else {
            return Err(invalid("base_dir is required (param or al.app.configure)"));
        };
        Ok(JobService::new(base_dir))
    }

    /// session ハンドルを解決する。
    fn session(&self, params: &Value) -> Result<&DatasetSession, ErrorBody> {
        let id = params
            .get("session")
            .and_then(Value::as_u64)
            .ok_or_else(|| invalid("session (u64) is required"))?;
        self.sessions
            .get(&id)
            .ok_or_else(|| invalid(format!("no dataset session {id}")))
    }

    /// payload を `rvf://data` へ登録し、`{source, bytes}` を返す。
    fn publish(
        &mut self,
        ctx: &mut ServiceContext<'_>,
        tag: &str,
        bytes: Vec<u8>,
        meta: Value,
    ) -> Result<Value, ErrorBody> {
        self.next_payload += 1;
        let name = format!("al/{tag}/{}", self.next_payload);
        let len = bytes.len() as u64;
        ctx.data
            .register(&name, Arc::new(VecDataSource::new(bytes)));
        Ok(json!({
            "source": name,
            "bytes": len,
            "meta": meta,
        }))
    }

    fn map_err(e: ServiceError) -> ErrorBody {
        // 標準コードは ruviform-ui の定数、AudioLens 固有分類
        // (not_found/io/stage)はそのまま文字列コードとして返す。
        let code = match e.code() {
            "invalid" => codes::INVALID_PARAMS,
            "cancelled" => codes::CANCELLED,
            other => other,
        };
        ErrorBody::new(code, e.to_string())
    }

    /// RunSpec を op パラメータから構築する。
    fn run_spec(params: &Value) -> Result<RunSpec, ErrorBody> {
        let raw_inputs = params
            .get("inputs")
            .and_then(Value::as_array)
            .ok_or_else(|| invalid("inputs (array) is required"))?;
        let mut inputs = Vec::new();
        for raw in raw_inputs {
            let role = raw
                .get("role")
                .and_then(Value::as_str)
                .ok_or_else(|| invalid("input requires role"))?;
            let path = raw
                .get("path")
                .and_then(Value::as_str)
                .ok_or_else(|| invalid("input requires path"))?;
            let mut input = parse_input_spec(&format!("{role}:{path}")).map_err(invalid)?;
            if let Some(id) = raw.get("input_id").and_then(Value::as_str) {
                input.input_id = al_core::InputId::parse(id)
                    .map_err(|e| invalid(format!("invalid input_id {id:?}: {e}")))?;
            }
            inputs.push(input);
        }
        let stages = match params.get("stages").and_then(Value::as_array) {
            Some(names) => Some(
                names
                    .iter()
                    .map(|n| {
                        n.as_str()
                            .ok_or_else(|| invalid("stage names must be strings"))
                            .and_then(|s| parse_stage(s).map_err(invalid))
                    })
                    .collect::<Result<Vec<_>, _>>()?,
            ),
            None => None,
        };
        // config は JSON オブジェクト直指定かファイルパス。
        let config = match params.get("config") {
            Some(v) if v.is_object() => v.clone(),
            _ => match params.get("config_path").and_then(Value::as_str) {
                Some(path) => {
                    let text = std::fs::read_to_string(path)
                        .map_err(|e| invalid(format!("cannot read config {path}: {e}")))?;
                    serde_json::from_str(&text)
                        .map_err(|e| invalid(format!("invalid config JSON {path}: {e}")))?
                }
                None => json!({}),
            },
        };
        Ok(RunSpec {
            inputs,
            stages,
            config,
        })
    }

    fn command(
        &mut self,
        params: &Value,
        ctx: &mut ServiceContext<'_>,
    ) -> Result<CallOutcome, ErrorBody> {
        let action = command::decode_action(params)
            .ok_or_else(|| invalid("command must be an action intent"))?;
        match (action.target.as_str(), action.action.as_str()) {
            (command::JOB_TARGET, command::ACTION_SELECT) => {
                let jobs = self
                    .job_service(&Value::Null)?
                    .list()
                    .map_err(Self::map_err)?;
                let row = action
                    .params
                    .as_ref()
                    .and_then(|p| p.get("row"))
                    .and_then(Value::as_u64)
                    .and_then(|r| usize::try_from(r).ok())
                    .ok_or_else(|| invalid("select requires row (u64)"))?;
                let job = jobs
                    .get(row)
                    .ok_or_else(|| invalid(format!("row {row} is outside {} jobs", jobs.len())))?;
                Ok(CallOutcome::Result(
                    json!({"status": "applied", "job_id": job.job_id.as_str()}),
                ))
            }
            (command::JOB_TARGET, command::ACTION_RESUME) => {
                let params = action.params.unwrap_or(Value::Null);
                self.call_internal("al.job.resume", &params, ctx)
            }
            (command::DESKTOP_TARGET, command::ACTION_TAB_SELECT) => {
                let index = action
                    .params
                    .as_ref()
                    .and_then(|p| p.get("index"))
                    .and_then(Value::as_u64)
                    .and_then(|i| usize::try_from(i).ok())
                    .filter(|&i| i < TAB_COUNT)
                    .ok_or_else(|| invalid(format!("tab-select requires index < {TAB_COUNT}")))?;
                Ok(CallOutcome::Result(
                    json!({"status": "applied", "index": index}),
                ))
            }
            (target, other) => Err(invalid(format!("unknown command {target:?} {other:?}"))),
        }
    }

    fn call_internal(
        &mut self,
        op: &str,
        params: &Value,
        ctx: &mut ServiceContext<'_>,
    ) -> Result<CallOutcome, ErrorBody> {
        match op {
            "al.app.info" => Ok(CallOutcome::Result(json!({
                "name": "al-gui",
                "version": env!("CARGO_PKG_VERSION"),
                "base_dir": self.base_dir.as_ref().map(|p| p.display().to_string()),
            }))),
            "al.app.configure" => {
                let base_dir = params
                    .get("base_dir")
                    .and_then(Value::as_str)
                    .ok_or_else(|| invalid("base_dir is required"))?;
                self.base_dir = Some(PathBuf::from(base_dir));
                Ok(CallOutcome::Result(json!({"base_dir": base_dir})))
            }
            "al.job.list" => {
                let service = self.job_service(params)?;
                let jobs = service.list().map_err(Self::map_err)?;
                let out: Vec<Value> = jobs
                    .iter()
                    .map(|j| {
                        json!({
                            "job_id": j.job_id.as_str(),
                            "inputs": j.inputs.iter().map(|i| json!({
                                "input_id": i.input_id.as_str(),
                                "role": i.role,
                                "path": i.path.display().to_string(),
                            })).collect::<Vec<_>>(),
                            "stages": j.stages.iter().map(|(k, s, e)| json!({
                                "stage": k,
                                "status": s,
                                "error": e,
                            })).collect::<Vec<_>>(),
                            "dataset_ready": j.dataset_ready,
                        })
                    })
                    .collect();
                Ok(CallOutcome::Result(json!({"jobs": out})))
            }
            "al.job.inspect" => {
                let service = self.job_service(params)?;
                let job_id = required_str(params, "job_id")?;
                let job_id = al_core::JobId::parse(job_id)
                    .map_err(|e| invalid(format!("invalid job_id: {e}")))?;
                let insp = service.inspect(&job_id).map_err(Self::map_err)?;
                Ok(CallOutcome::Result(json!({
                    "job_id": insp.manifest.job_id.as_str(),
                    "manifest": insp.manifest,
                    "dataset_ready": insp.dataset_ready,
                    "dataset_artifacts": insp.dataset_artifacts,
                })))
            }
            "al.job.run" => {
                let service = self.job_service(params)?;
                let spec = Self::run_spec(params)?;
                Ok(CallOutcome::Job(JobSpec::new("al.job.run", move |ctx| {
                    let cancelled = || ctx.is_cancelled();
                    let on_stage = |ev: al_job::StageEvent| {
                        let done = match ev.kind {
                            al_job::StageEventKind::Started => ev.index as u64,
                            al_job::StageEventKind::Completed { .. }
                            | al_job::StageEventKind::Reused
                            | al_job::StageEventKind::Failed => ev.index as u64 + 1,
                        };
                        ctx.progress(done, ev.total as u64);
                    };
                    let ctl = RunControl {
                        is_cancelled: Some(&cancelled),
                        on_stage: Some(&on_stage),
                    };
                    service
                        .run(spec, &ctl)
                        .map(|(job_id, manifest)| {
                            json!({
                                "job_id": job_id.as_str(),
                                "stages": stage_statuses(&manifest),
                            })
                        })
                        .map_err(AlGuiService::map_err)
                })))
            }
            "al.job.resume" => {
                let service = self.job_service(params)?;
                let job_id = required_str(params, "job_id")?;
                let job_id = al_core::JobId::parse(job_id)
                    .map_err(|e| invalid(format!("invalid job_id: {e}")))?;
                Ok(CallOutcome::Job(JobSpec::new(
                    "al.job.resume",
                    move |ctx| {
                        let cancelled = || ctx.is_cancelled();
                        let on_stage = |ev: al_job::StageEvent| {
                            let done = match ev.kind {
                                al_job::StageEventKind::Started => ev.index as u64,
                                al_job::StageEventKind::Completed { .. }
                                | al_job::StageEventKind::Reused
                                | al_job::StageEventKind::Failed => ev.index as u64 + 1,
                            };
                            ctx.progress(done, ev.total as u64);
                        };
                        let ctl = RunControl {
                            is_cancelled: Some(&cancelled),
                            on_stage: Some(&on_stage),
                        };
                        service
                            .resume(&job_id, &ctl)
                            .map(|manifest| {
                                json!({
                                    "job_id": manifest.job_id.as_str(),
                                    "stages": stage_statuses(&manifest),
                                })
                            })
                            .map_err(AlGuiService::map_err)
                    },
                )))
            }
            command::COMMAND_INTENT_OP => self.command(params, ctx),
            "al.ds.open" => {
                let session = if let Some(root) = params.get("root").and_then(Value::as_str) {
                    DatasetSession::open(root).map_err(Self::map_err)?
                } else {
                    let service = self.job_service(params)?;
                    let job_id = required_str(params, "job_id")?;
                    let job_id = al_core::JobId::parse(job_id)
                        .map_err(|e| invalid(format!("invalid job_id: {e}")))?;
                    service.open_dataset(&job_id).map_err(Self::map_err)?
                };
                self.next_session += 1;
                let id = self.next_session;
                let root = session.root().display().to_string();
                self.sessions.insert(id, session);
                Ok(CallOutcome::Result(json!({
                    "session": id,
                    "root": root,
                })))
            }
            "al.ds.close" => {
                let id = params
                    .get("session")
                    .and_then(Value::as_u64)
                    .ok_or_else(|| invalid("session is required"))?;
                self.sessions.remove(&id);
                Ok(CallOutcome::Result(json!({"closed": id})))
            }
            "al.ds.overview" => {
                let session = self.session(params)?;
                let overview = session.overview().map_err(Self::map_err)?;
                let timeline = session.timeline().map_err(Self::map_err)?;
                Ok(CallOutcome::Result(json!({
                    "overview": overview,
                    "timeline": timeline,
                })))
            }
            "al.ds.events" => {
                let session = self.session(params)?;
                let out = session
                    .events(
                        params.get("row_start").and_then(Value::as_u64).unwrap_or(0),
                        params
                            .get("row_end")
                            .and_then(Value::as_u64)
                            .unwrap_or(u64::MAX),
                        params.get("start_s").and_then(Value::as_f64),
                        params.get("end_s").and_then(Value::as_f64),
                    )
                    .map_err(Self::map_err)?;
                Ok(CallOutcome::Result(out))
            }
            "al.ds.locate" => {
                let out = self.session(params)?.locate().map_err(Self::map_err)?;
                Ok(CallOutcome::Result(out))
            }
            "al.ds.locate_detail" => {
                let session = self.session(params)?;
                let src = required_str(params, "source")?;
                let cap = required_str(params, "capture")?;
                let out = session.locate_detail(src, cap).map_err(Self::map_err)?;
                Ok(CallOutcome::Result(out))
            }
            "al.ds.features" => {
                let session = self.session(params)?;
                let roles = params.get("roles").and_then(Value::as_array).map(|a| {
                    a.iter()
                        .filter_map(Value::as_str)
                        .map(str::to_owned)
                        .collect()
                });
                let query = FeatureQuery {
                    roles,
                    name_prefix: params
                        .get("name_prefix")
                        .and_then(Value::as_str)
                        .map(str::to_owned),
                    feature_id_prefix: params
                        .get("feature_id_prefix")
                        .and_then(Value::as_str)
                        .map(str::to_owned),
                    limit: params
                        .get("limit")
                        .and_then(Value::as_u64)
                        .map(|v| v as usize),
                };
                let out = session.features(&query).map_err(Self::map_err)?;
                Ok(CallOutcome::Result(out))
            }
            "al.ds.artifacts" => {
                let session = self.session(params)?;
                let out = session.artifacts(params.get("role").and_then(Value::as_str));
                Ok(CallOutcome::Result(json!({"artifacts": out})))
            }
            "al.ds.artifact" => {
                let session = self.session(params)?;
                let path = required_str(params, "path")?;
                let out = session.artifact_detail(path).map_err(Self::map_err)?;
                Ok(CallOutcome::Result(out))
            }
            "al.ds.read" => {
                let session = self.session(params)?;
                let kind = required_str(params, "kind")?;
                let role = required_str(params, "role")?;
                let name = required_str(params, "name")?;
                let out = match kind {
                    "table" => {
                        let columns = params.get("columns").and_then(Value::as_array).map(|a| {
                            a.iter()
                                .filter_map(Value::as_u64)
                                .map(|v| v as usize)
                                .collect::<Vec<_>>()
                        });
                        session
                            .table_rows(
                                role,
                                name,
                                params.get("row_start").and_then(Value::as_u64).unwrap_or(0),
                                params
                                    .get("row_end")
                                    .and_then(Value::as_u64)
                                    .unwrap_or(u64::MAX),
                                columns.as_deref(),
                            )
                            .map_err(Self::map_err)?
                    }
                    "tensor" => {
                        let region: Vec<[u64; 2]> = params
                            .get("region")
                            .and_then(Value::as_array)
                            .ok_or_else(|| invalid("region (array of [start,end]) required"))?
                            .iter()
                            .map(|v| {
                                let pair = v
                                    .as_array()
                                    .ok_or_else(|| invalid("region dim must be [start,end]"))?;
                                if pair.len() != 2 {
                                    return Err(invalid("region dim must be [start,end]"));
                                }
                                Ok([
                                    pair[0]
                                        .as_u64()
                                        .ok_or_else(|| invalid("region bounds must be u64"))?,
                                    pair[1]
                                        .as_u64()
                                        .ok_or_else(|| invalid("region bounds must be u64"))?,
                                ])
                            })
                            .collect::<Result<_, ErrorBody>>()?;
                        session
                            .tensor_region(
                                role,
                                name,
                                &region,
                                params
                                    .get("max_elements")
                                    .and_then(Value::as_u64)
                                    .unwrap_or(256 * 1024),
                            )
                            .map_err(Self::map_err)?
                    }
                    "json" => session.json_artifact(role, name).map_err(Self::map_err)?,
                    "audio" => {
                        let frames = session
                            .reader()
                            .audio_frames(role, name)
                            .map_err(|e| Self::map_err(ServiceError::from(e)))?;
                        json!({"frames": frames})
                    }
                    _ => return Err(invalid(format!("unknown read kind {kind:?}"))),
                };
                Ok(CallOutcome::Result(out))
            }
            "al.view.waveform" => {
                let session = self.session(params)?;
                let input_id = required_str(params, "input_id")?;
                let channel = params.get("channel").and_then(Value::as_u64).unwrap_or(0) as usize;
                let start_s = params.get("start_s").and_then(Value::as_f64).unwrap_or(0.0);
                let end_s = params
                    .get("end_s")
                    .and_then(Value::as_f64)
                    .unwrap_or(f64::MAX / 2.0);
                let bins = params.get("bins").and_then(Value::as_u64).unwrap_or(1024) as usize;
                let (lod, meta) = session
                    .waveform_view(input_id, channel, start_s, end_s, bins)
                    .map_err(Self::map_err)?;
                let out = self.publish(ctx, "waveform", lod.bytes.0, meta)?;
                Ok(CallOutcome::Result(out))
            }
            "al.view.spectrogram" => {
                let session = self.session(params)?;
                let input_id = required_str(params, "input_id")?;
                let channel = params.get("channel").and_then(Value::as_u64).unwrap_or(0) as usize;
                // frame 指定が無ければ秒範囲から STFT hop で換算する
                // (frontend は時間軸しか知らない)。
                let frames = if let (Some(fs), Some(fe)) = (
                    params.get("frame_start").and_then(Value::as_u64),
                    params.get("frame_end").and_then(Value::as_u64),
                ) {
                    fs..fe
                } else {
                    let info = session
                        .spectrum(input_id, channel)
                        .map_err(Self::map_err)?
                        .info();
                    let hop = info.grid.hop.max(1) as f64;
                    let rate = info.sample_rate as f64;
                    let s0 = params.get("start_s").and_then(Value::as_f64).unwrap_or(0.0);
                    let s1 = params
                        .get("end_s")
                        .and_then(Value::as_f64)
                        .unwrap_or(f64::MAX / 2.0);
                    ((s0.max(0.0) * rate / hop) as u64)..((s1 * rate / hop) as u64)
                };
                let rows = params.get("rows").and_then(Value::as_u64).unwrap_or(256) as usize;
                let cols = params.get("cols").and_then(Value::as_u64).unwrap_or(1024) as usize;
                let (bytes, meta) = session
                    .spectrogram_tile(input_id, channel, frames, rows, cols)
                    .map_err(Self::map_err)?;
                let out = self.publish(ctx, "spectrogram", bytes.0, meta)?;
                Ok(CallOutcome::Result(out))
            }
            _ => Err(ErrorBody::new(
                codes::UNKNOWN_OP,
                format!("unregistered op {op:?}"),
            )),
        }
    }
}

impl Default for AlGuiService {
    fn default() -> Self {
        Self::new()
    }
}

impl Service for AlGuiService {
    fn ops(&self) -> &'static [OpInfo] {
        OPS
    }

    fn call(
        &mut self,
        op: &str,
        params: &Value,
        ctx: &mut ServiceContext<'_>,
    ) -> Result<CallOutcome, ErrorBody> {
        // panic は runtime が job 化するのでここでは通常の Result を返す。
        self.call_internal(op, params, ctx)
    }
}

fn invalid(message: impl Into<String>) -> ErrorBody {
    ErrorBody::new(codes::INVALID_PARAMS, message)
}

fn required_str<'a>(params: &'a Value, key: &str) -> Result<&'a str, ErrorBody> {
    params
        .get(key)
        .and_then(Value::as_str)
        .ok_or_else(|| invalid(format!("{key} (string) is required")))
}

/// manifest の stage 状態を JSON 配列にする(job run/resume 結果)。
fn stage_statuses(manifest: &al_job::Manifest) -> Vec<Value> {
    manifest
        .stages
        .iter()
        .map(|r| {
            json!({
                "stage": r.stage,
                "status": r.status,
                "duration_ms": r.duration_ms,
                "error": r.error,
            })
        })
        .collect()
}

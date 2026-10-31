//! Job 操作サービス。`al-cli`/`al-gui` 共通の run/resume/inspect/
//! list/cancel 入口。`StageRunner` の所有境界はこの層に限定する。

use std::path::{Path, PathBuf};

use al_core::{InputId, JobId, StageKind, StageStatus};
use al_job::job::{JobDefinition, JobInput};
use al_job::manifest::Manifest;
use al_job::runner::{RunHooks, StageEvent, StageRunner};
use al_job::stage::Stage;

use crate::error::ServiceError;
use crate::pipeline::{default_stage_kinds, make_stage, resolve_config};

/// `run` の入力仕様。CLI の `--input/--stages/--config` に相当する
/// 概念を型で表す。
#[derive(Debug, Clone)]
pub struct RunSpec {
    /// 宣言入力(role・path・任意 input_id)。`JobDefinition::create`
    /// が絶対パス化・SHA-256 pin を行う。
    pub inputs: Vec<JobInput>,
    /// 実行 stage 集合。`None` は [`default_stage_kinds`]。
    pub stages: Option<Vec<StageKind>>,
    /// Job config 本体。`resolve_config` による dictionary スナップ
    /// ショットは run 内で適用される。
    pub config: serde_json::Value,
}

/// run/resume 実行の観測・中断制御。GUI は `JobContext` の
/// cancel/progress をここへ橋渡しする。`Default` は観測なし。
#[derive(Default)]
pub struct RunControl<'a> {
    /// stage 境界で発火する進捗イベント。
    pub on_stage: Option<&'a dyn Fn(StageEvent)>,
    /// キャンセル確認。`true` を返した時点で次の stage は起動せず、
    /// manifest を Pending のまま保存して中断する。
    pub is_cancelled: Option<&'a dyn Fn() -> bool>,
}

impl<'a> RunControl<'a> {
    fn hooks(&'a self) -> RunHooks<'a> {
        RunHooks {
            on_stage: self.on_stage,
            is_cancelled: self.is_cancelled,
        }
    }
}

/// `inspect` の結果: manifest 本体 + dataset 派生状態。
#[derive(Debug, Clone)]
pub struct JobInspection {
    /// Job manifest(stage 状態・artifact lineage を含む)。
    pub manifest: Manifest,
    /// `analysis/` dataset が開ける状態か。
    pub dataset_ready: bool,
    /// dataset 内 artifact 数(`dataset_ready` 時のみ)。
    pub dataset_artifacts: Option<u64>,
}

/// `list` が返す job の要約。
#[derive(Debug, Clone)]
pub struct JobSummary {
    /// Job 識別子。
    pub job_id: JobId,
    /// 宣言入力(input_id・role・source path)。
    pub inputs: Vec<JobInput>,
    /// stage 状態一覧(plan order)。
    pub stages: Vec<(StageKind, StageStatus, Option<String>)>,
    /// `analysis/` dataset が存在するか。
    pub dataset_ready: bool,
}

/// base_dir 配下の job を管理するサービス。
pub struct JobService {
    base_dir: PathBuf,
}

impl JobService {
    /// `base_dir` 配下に job ディレクトリを作るサービスを作る。
    pub fn new(base_dir: impl Into<PathBuf>) -> Self {
        Self {
            base_dir: base_dir.into(),
        }
    }

    /// base_dir。
    pub fn base_dir(&self) -> &Path {
        &self.base_dir
    }

    /// job ディレクトリ(`<base>/<job-id>`)。
    pub fn job_dir(&self, job_id: &JobId) -> PathBuf {
        StageRunner::new(&self.base_dir).job_dir(job_id)
    }

    /// job の persisted dataset ルート(`<job>/analysis`)。
    pub fn dataset_dir(&self, job_id: &JobId) -> PathBuf {
        self.job_dir(job_id).join("analysis")
    }

    /// 新規 job を作成して全 stage を実行する。キャンセル要求時は
    /// `Err(ServiceError::cancelled())` — job ディレクトリと
    /// manifest は残り、`resume` で続行できる。
    pub fn run(
        &self,
        spec: RunSpec,
        ctl: &RunControl<'_>,
    ) -> Result<(JobId, Manifest), ServiceError> {
        let mut config = spec.config;
        resolve_config(&mut config).map_err(ServiceError::invalid)?;
        let stages_kinds = spec.stages.unwrap_or_else(default_stage_kinds);
        let def = JobDefinition::create(spec.inputs, config.clone(), stages_kinds)
            .map_err(|e| ServiceError::invalid(format!("cannot create job: {e}")))?;
        let mut stages = Self::build_stages(&def)?;
        let runner = StageRunner::new(&self.base_dir);
        let manifest = runner
            .run_with_hooks(&def, &mut stages, &ctl.hooks())
            .map_err(|e| Self::map_run_error(e, ctl))?;
        Ok((def.job_id, manifest))
    }

    /// 既存 job を manifest 記録どおりに再開する。`run` と同じ
    /// stage 構築を使い、完了済み stage は runner 側で再利用される。
    pub fn resume(&self, job_id: &JobId, ctl: &RunControl<'_>) -> Result<Manifest, ServiceError> {
        let def = self.load_definition(job_id)?;
        let mut stages = Self::build_stages(&def)?;
        let runner = StageRunner::new(&self.base_dir);
        runner
            .resume_with_hooks(&def, &mut stages, &ctl.hooks())
            .map_err(|e| Self::map_run_error(e, ctl))
    }

    /// 既存 job の可搬 export パッケージを生成する(`al export` の実体)。
    /// 実行計画に export stage が無い job(dataset までの plan)では
    /// manifest に record を追加してから実行する。完了済みの export は
    /// runner 側で再利用される。依存 stage が未完了なら stage 失敗と
    /// して伝播し、先に `resume` するよう求める。
    pub fn export(&self, job_id: &JobId, ctl: &RunControl<'_>) -> Result<Manifest, ServiceError> {
        let def = self.load_definition(job_id)?;
        let mut stages =
            vec![make_stage(StageKind::Export, &def.config).map_err(ServiceError::invalid)?];
        let runner = StageRunner::new(&self.base_dir);
        runner
            .run_stage_with_hooks(job_id, StageKind::Export, &mut stages, &ctl.hooks())
            .map_err(|e| Self::map_run_error(e, ctl))
    }

    /// job の現状を読む(manifest + dataset 有無)。
    pub fn inspect(&self, job_id: &JobId) -> Result<JobInspection, ServiceError> {
        let job_dir = self.job_dir(job_id);
        let manifest = Manifest::load(&job_dir).map_err(|e| {
            if e.kind() == std::io::ErrorKind::NotFound {
                ServiceError::not_found(format!("job {job_id} not found"))
            } else {
                ServiceError::from(e)
            }
        })?;
        let (dataset_ready, dataset_artifacts) =
            match al_dataset::reader::DatasetReader::open(&job_dir.join("analysis")) {
                Ok(reader) => (true, Some(reader.manifest().artifacts.len() as u64)),
                Err(_) => (false, None),
            };
        Ok(JobInspection {
            manifest,
            dataset_ready,
            dataset_artifacts,
        })
    }

    /// manifest が存在する全 job を列挙する。壊れた manifest の job
    /// は `stages` 空の要約として残す(一覧で不可視にしない)。
    pub fn list(&self) -> Result<Vec<JobSummary>, ServiceError> {
        let mut out = Vec::new();
        let entries = match std::fs::read_dir(&self.base_dir) {
            Ok(entries) => entries,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(out),
            Err(e) => return Err(e.into()),
        };
        for entry in entries {
            let entry = entry?;
            let job_dir = entry.path();
            if !job_dir.is_dir() {
                continue;
            }
            let Some(name) = job_dir.file_name().and_then(|n| n.to_str()) else {
                continue;
            };
            let Ok(job_id) = JobId::parse(name) else {
                continue;
            };
            let (inputs, stages) = match Manifest::load(&job_dir) {
                Ok(manifest) => (
                    manifest.inputs.clone(),
                    manifest
                        .stages
                        .iter()
                        .map(|r| (r.stage, r.status, r.error.clone()))
                        .collect(),
                ),
                Err(_) => (Vec::new(), Vec::new()),
            };
            out.push(JobSummary {
                job_id,
                inputs,
                stages,
                dataset_ready: job_dir.join("analysis").join("manifest.json").is_file(),
            });
        }
        out.sort_by(|a, b| a.job_id.as_str().cmp(b.job_id.as_str()));
        Ok(out)
    }

    /// manifest から `JobDefinition` を復元する(resume 用)。
    pub fn load_definition(&self, job_id: &JobId) -> Result<JobDefinition, ServiceError> {
        let job_dir = self.job_dir(job_id);
        let manifest = Manifest::load(&job_dir).map_err(|e| {
            if e.kind() == std::io::ErrorKind::NotFound {
                ServiceError::not_found(format!("job {job_id} not found"))
            } else {
                ServiceError::from(e)
            }
        })?;
        Ok(JobDefinition {
            job_id: job_id.clone(),
            inputs: manifest.inputs.clone(),
            config: manifest.config.clone(),
            stages: manifest.stages.iter().map(|s| s.stage).collect(),
        })
    }

    /// job の persisted dataset session を開く(`<job>/analysis`)。
    pub fn open_dataset(
        &self,
        job_id: &JobId,
    ) -> Result<crate::dataset::DatasetSession, ServiceError> {
        crate::dataset::DatasetSession::open(self.dataset_dir(job_id))
    }

    /// input 引数の `input_id` 指定(`--input-ids` 相当)を適用する。
    /// GUI/CLI で同じ規則: 個数一致・一意。
    pub fn assign_input_ids(
        inputs: &mut [JobInput],
        ids: Vec<InputId>,
    ) -> Result<(), ServiceError> {
        if ids.len() != inputs.len() {
            return Err(ServiceError::invalid(
                "input_ids must provide one ID per input entry",
            ));
        }
        let mut seen = std::collections::HashSet::new();
        for (input, id) in inputs.iter_mut().zip(ids) {
            if !seen.insert(id.clone()) {
                return Err(ServiceError::invalid("input_ids must be unique"));
            }
            input.input_id = id;
        }
        Ok(())
    }

    fn build_stages(def: &JobDefinition) -> Result<Vec<Box<dyn Stage>>, ServiceError> {
        def.stages
            .iter()
            .map(|kind| make_stage(*kind, &def.config))
            .collect::<Result<Vec<_>, _>>()
            .map_err(ServiceError::invalid)
    }

    /// 実行失敗の分類: cancel flag が立っていれば Cancelled、
    /// それ以外は stage 失敗として伝播する。
    fn map_run_error(err: al_job::StageError, ctl: &RunControl<'_>) -> ServiceError {
        if ctl.is_cancelled.is_some_and(|f| f()) {
            ServiceError::cancelled()
        } else {
            ServiceError::from_stage(err)
        }
    }
}

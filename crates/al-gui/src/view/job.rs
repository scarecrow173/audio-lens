//! Job/Run の view model — `JobSummary`/`JobInspection` の typed 写し。

use al_core::{StageKind, StageStatus};
use al_job::manifest::Manifest;
use al_service::jobs::{JobInspection, JobSummary};

use super::{ViewError, tag};

/// job の input 行(`role` は `InputRole` の表示名)。
#[derive(Debug, Clone, PartialEq)]
pub struct InputRow {
    /// Input id。
    pub input_id: String,
    /// role 名(source/capture_bug 等)。
    pub role: String,
    /// 入力 file path。
    pub path: String,
}

/// stage 1 件の状態。
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct StageRow {
    /// stage 種別。
    pub kind: StageKind,
    /// 状態。
    pub status: StageStatus,
}

/// `al.job.list` 1 行分。
#[derive(Debug, Clone, PartialEq)]
pub struct JobRow {
    /// job id。
    pub job_id: String,
    /// 入力一覧。
    pub inputs: Vec<InputRow>,
    /// stage 状態(error は `errors` 側へ)。
    pub stages: Vec<StageRow>,
    /// stage error の typed 写し(stage index 順)。
    pub errors: Vec<Option<String>>,
    /// dataset が揃っているか。
    pub dataset_ready: bool,
}

impl JobRow {
    /// `JobSummary` から写す。`stages` の `(StageKind, StageStatus,
    /// Option<String>)` を型安全に分離する。
    #[must_use]
    pub fn from_summary(s: &JobSummary) -> Self {
        let mut stages = Vec::with_capacity(s.stages.len());
        let mut errors = Vec::with_capacity(s.stages.len());
        for (kind, status, err) in &s.stages {
            stages.push(StageRow {
                kind: *kind,
                status: *status,
            });
            errors.push(err.clone());
        }
        Self {
            job_id: s.job_id.as_str().to_string(),
            inputs: s
                .inputs
                .iter()
                .map(|i| InputRow {
                    input_id: i.input_id.as_str().to_string(),
                    role: tag(&i.role),
                    path: i.path.display().to_string(),
                })
                .collect(),
            stages,
            errors,
            dataset_ready: s.dataset_ready,
        }
    }
}

/// `al.job.inspect` の写し。manifest は typed のまま保持し、
/// GUI 側で再構築しない。
#[derive(Debug)]
pub struct JobDetail {
    /// job id。
    pub job_id: String,
    /// manifest artifact 数。
    pub artifact_count: usize,
    /// dataset が揃っているか。
    pub dataset_ready: bool,
    /// dataset artifact 数(`dataset_ready` 時のみ)。
    pub dataset_artifacts: Option<u64>,
}

impl JobDetail {
    /// `JobInspection` から写す。
    #[must_use]
    pub fn from_inspection(i: &JobInspection) -> Self {
        Self {
            job_id: i.manifest.job_id.as_str().to_string(),
            artifact_count: i.manifest.artifacts.len(),
            dataset_ready: i.dataset_ready,
            dataset_artifacts: i.dataset_artifacts,
        }
    }
}

/// `al.job.run`/`resume` の完了結果の写し(stage 状態 + 失敗理由)。
/// `stages` は `{stage,status,duration_ms,error}` を emit する
/// (al-gui `stage_statuses`) — `failed`/`skipped` stage の理由を
/// 落とさないよう `errors`/`durations_ms` を index 対応で保持する。
#[derive(Debug, Clone, PartialEq)]
pub struct RunOutcome {
    /// job id。
    pub job_id: String,
    /// stage 状態一覧。
    pub stages: Vec<StageRow>,
    /// stage error(`stages` と index 対応)。
    pub errors: Vec<Option<String>>,
    /// stage 実行時間 ms(`stages` と index 対応、unknown は `None`)。
    pub durations_ms: Vec<Option<u64>>,
}

impl RunOutcome {
    /// job の永続 manifest(最新の stage 記録)から写す。
    #[must_use]
    pub fn from_manifest(m: &Manifest) -> Self {
        Self {
            job_id: m.job_id.as_str().to_string(),
            stages: m
                .stages
                .iter()
                .map(|r| StageRow {
                    kind: r.stage,
                    status: r.status,
                })
                .collect(),
            errors: m.stages.iter().map(|r| r.error.clone()).collect(),
            durations_ms: m.stages.iter().map(|r| r.duration_ms).collect(),
        }
    }

    /// op 結果 JSON `{"job_id", "stages":[{stage,status,..}]}` から写す。
    ///
    /// # Errors
    /// `job_id`/`stages` が期待形でない場合。
    pub fn from_json(v: &serde_json::Value) -> Result<Self, ViewError> {
        let job_id = super::required(v, "job_id")?.to_string();
        let stages_raw = v
            .get("stages")
            .and_then(serde_json::Value::as_array)
            .ok_or_else(|| ViewError::Invalid("result lacks stages".into()))?;
        let mut stages = Vec::with_capacity(stages_raw.len());
        let mut errors = Vec::with_capacity(stages_raw.len());
        let mut durations_ms = Vec::with_capacity(stages_raw.len());
        for row in stages_raw {
            let kind = al_service::parse_stage(super::required(row, "stage")?)
                .map_err(ViewError::Invalid)?;
            let status = super::parse_tag(row, "status")?;
            stages.push(StageRow { kind, status });
            errors.push(
                row.get("error")
                    .and_then(serde_json::Value::as_str)
                    .map(str::to_string),
            );
            durations_ms.push(super::u64_or(row, "duration_ms"));
        }
        Ok(Self {
            job_id,
            stages,
            errors,
            durations_ms,
        })
    }
}

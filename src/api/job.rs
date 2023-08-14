// License: GNU Affero General Public License v3 or later
// A copy of GNU AGPL v3 should have been included in this software package in LICENSE.txt.

use std::convert::TryFrom;

use axum::{
    extract,
    routing::{get, post},
    Extension, Json, Router,
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sqlx::PgPool;
use uuid::Uuid;

use crate::jobs::blast::BlastInput;
use crate::jobs::clusterblast::ClusterBlast;
use crate::jobs::comparippson::CompaRiPPson;
use crate::jobs::ping::Ping;
use crate::models::job::{JobEntry, JobStatus, JobType};
use crate::Result;

pub fn routes() -> Router {
    Router::new()
        .route("/api/jobs/clusterblast", post(create_clusterblast))
        .route("/api/jobs/comparippson", post(create_comparippson))
        .route("/api/jobs/ping", post(create_ping))
        .route("/api/job/:job_id", get(get_job_info))
}

async fn create_clusterblast(
    Extension(pool): Extension<PgPool>,
    extract::Json(input): extract::Json<BlastInput>,
) -> Result<Json<Value>> {
    let mut job = JobEntry::new(JobType::ClusterBlast(ClusterBlast::from_blast(input)));
    job.commit(&pool).await?;

    let info = JobInfo::try_from(job)?;
    Ok(Json(json!(info)))
}

async fn create_comparippson(
    Extension(pool): Extension<PgPool>,
    extract::Json(input): extract::Json<BlastInput>,
) -> Result<Json<Value>> {
    let mut job = JobEntry::new(JobType::CompaRiPPson(CompaRiPPson::from_blast(input)));
    job.commit(&pool).await?;

    let info = JobInfo::try_from(job)?;
    Ok(Json(json!(info)))
}

#[derive(Debug, Deserialize, Serialize)]
struct PingRequest {
    pub greeting: String,
}

async fn create_ping(
    Extension(pool): Extension<PgPool>,
    extract::Json(req): extract::Json<PingRequest>,
) -> Result<Json<Value>> {
    let mut job = JobEntry::new(JobType::Ping(Ping::new(&req.greeting)));
    job.commit(&pool).await?;

    let info = JobInfo::try_from(job)?;
    Ok(Json(json!(info)))
}

async fn get_job_info(
    Extension(pool): Extension<PgPool>,
    extract::Path(job_id): extract::Path<Uuid>,
) -> Result<Json<Value>> {
    let id = job_id.to_string();
    let job = JobEntry::from_db(&pool, &id).await?;
    let info = JobInfo::try_from(job)?;
    Ok(Json(json!(info)))
}

#[derive(Debug, Deserialize, Serialize)]
pub struct JobInfo {
    pub id: String,
    pub jobtype: String,
    pub status: String,
    pub submitted: DateTime<Utc>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub next: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub results: Option<Value>,
}

impl TryFrom<JobEntry> for JobInfo {
    type Error = crate::Error;
    fn try_from(value: JobEntry) -> std::result::Result<Self, Self::Error> {
        let mut info = Self {
            id: value.id,
            jobtype: value.jobtype.to_string(),
            status: value.status.to_string(),
            submitted: value.submitted_date,
            next: None,
            results: None,
        };
        match value.status {
            JobStatus::Error | JobStatus::Delete => {} // do nothing
            JobStatus::Pending | JobStatus::Running => {
                info.next = Some(format!("/api/job/{}", info.id))
            }
            JobStatus::Done => {
                let val = match value.jobtype {
                    JobType::ClusterBlast(cb) => serde_json::to_value(cb.results)?,
                    JobType::CompaRiPPson(cr) => serde_json::to_value(cr.results)?,
                    JobType::Ping(ping) => serde_json::to_value(ping.reply)?,
                    JobType::StoredQuery(q) => serde_json::to_value(q.filename)?,
                };
                info.results = Some(val);
            }
        };
        Ok(info)
    }
}

// License: GNU Affero General Public License v3 or later
// A copy of GNU AGPL v3 should have been included in this software package in LICENSE.txt.

use std::convert::TryFrom;
use std::str::FromStr;
use std::string::ToString;

use chrono::prelude::*;
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use uuid::Uuid;

use crate::jobs::{blast, clusterblast, comparippson, ping, stored_query};
use crate::{Error, Result};

#[derive(Debug, Deserialize, Serialize, Clone, strum::Display)]
#[strum(serialize_all = "lowercase")]
#[serde(rename_all = "lowercase")]
pub enum JobType {
    ClusterBlast(clusterblast::ClusterBlast),
    CompaRiPPson(comparippson::CompaRiPPson),
    Ping(ping::Ping),
    StoredQuery(stored_query::StoredQuery),
}

#[derive(
    Debug, Deserialize, Serialize, Clone, strum::Display, strum::AsRefStr, strum::EnumString,
)]
#[strum(serialize_all = "lowercase")]
#[serde(rename_all = "lowercase")]
pub enum JobStatus {
    Pending,
    Running,
    Done,
    Error,
    Delete,
}

#[derive(Debug)]
pub struct JobEntry {
    pub id: String,
    pub jobtype: JobType,
    pub status: JobStatus,
    pub runner: String,
    pub submitted_date: DateTime<Utc>,
    version: i32,
}

impl JobEntry {
    pub fn new(jobtype: JobType) -> Self {
        let id = Uuid::new_v4().to_string();
        Self {
            id,
            jobtype,
            status: JobStatus::Pending,
            runner: "".to_owned(),
            submitted_date: Utc::now(),
            version: 0,
        }
    }

    pub async fn from_db(pool: &PgPool, id: &str) -> Result<Self> {
        let job = sqlx::query_as!(
            DbJob,
            r#"
            SELECT * FROM asdb_jobs.jobs
                WHERE id = $1"#,
            id,
        )
        .fetch_one(pool)
        .await?;

        Ok(job.try_into()?)
    }

    pub async fn next_pending(pool: &PgPool) -> Result<Option<Self>> {
        let job_opt = sqlx::query_as!(
            DbJob,
            r#"
            SELECT * FROM asdb_jobs.jobs
                WHERE status = 'pending'
                ORDER BY submitted_date
                LIMIT 1"#,
        )
        .fetch_optional(pool)
        .await?;

        if let Some(job) = job_opt {
            return Ok(Some(JobEntry::try_from(job)?));
        }

        Ok(None)
    }

    pub async fn next_to_clean(pool: &PgPool, days: f64) -> Result<Option<Self>> {
        let job_opt = sqlx::query_as!(
            DbJob,
            r#"
            SELECT * FROM asdb_jobs.jobs
                WHERE submitted_date < now() - interval '1 day' * $1 OR status = 'delete'
                ORDER BY submitted_date
                LIMIT 1"#,
            days,
        )
        .fetch_optional(pool)
        .await?;

        if let Some(job) = job_opt {
            return Ok(Some(JobEntry::try_from(job)?));
        }

        Ok(None)
    }

    pub async fn fetch(&mut self, pool: &PgPool) -> Result<&mut Self> {
        let job: JobEntry = sqlx::query_as!(
            DbJob,
            r#"
            SELECT * FROM asdb_jobs.jobs
                WHERE id = $1"#,
            self.id,
        )
        .fetch_one(pool)
        .await?
        .try_into()?;

        self.jobtype = job.jobtype;
        self.status = job.status;
        Ok(self)
    }

    pub async fn commit(&mut self, pool: &PgPool) -> Result<&mut Self> {
        let tx = pool.begin().await?;
        // get the non-mutable pointer
        let db_job = DbJob::try_from(&*self)?;
        let count = sqlx::query!(
            r#"
            SELECT COUNT(*) FROM asdb_jobs.jobs
                WHERE id = $1
            "#,
            self.id,
        )
        .fetch_one(pool)
        .await?
        .count
        .unwrap_or_default();
        if count == 0 {
            sqlx::query!(
                r#"
                INSERT INTO asdb_jobs.jobs (id, jobtype, status, runner, submitted_date, data, results, version)
                VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
            "#,
            db_job.id,
            db_job.jobtype,
            db_job.status,
            db_job.runner,
            db_job.submitted_date,
            db_job.data,
            db_job.results,
            db_job.version
            )
            .execute(pool)
            .await?;
            tx.commit().await?;
            return Ok(self);
        }
        let new_version = sqlx::query!(
            r#"
            UPDATE asdb_jobs.jobs SET
                status = $3,
                runner = $4,
                data = $5,
                results = $6,
                version = ($2 + 1)
            WHERE id = $1 AND version = $2
            RETURNING version
            "#,
            db_job.id,
            db_job.version,
            db_job.status,
            db_job.runner,
            db_job.data,
            db_job.results,
        )
        .fetch_one(pool)
        .await?
        .version;
        tx.commit().await?;
        self.version = new_version;

        Ok(self)
    }

    pub async fn delete(&self, pool: &PgPool) -> Result<()> {
        let tx = pool.begin().await?;
        sqlx::query!("DELETE FROM asdb_jobs.jobs WHERE id = $1", self.id)
            .execute(pool)
            .await?;
        self.update_stats(pool).await?;
        tx.commit().await?;
        Ok(())
    }

    async fn update_stats(&self, pool: &PgPool) -> Result<()> {
        sqlx::query!(
            r#"UPDATE asdb_jobs.counters SET value = value + 1 WHERE name = 'total_jobs'"#,
        )
        .execute(pool)
        .await?;
        sqlx::query!(
            r#"
            INSERT INTO asdb_jobs.counters(name, value) VALUES ($1, 1)
            ON CONFLICT (name) DO UPDATE SET value = counters.value + 1
            "#,
            format!("{}_jobs", self.jobtype)
        )
        .execute(pool)
        .await?;
        Ok(())
    }
}

impl TryFrom<DbJob> for JobEntry {
    type Error = Error;

    fn try_from(value: DbJob) -> std::result::Result<Self, Self::Error> {
        let jobtype = match value.jobtype.as_ref() {
            "clusterblast" => {
                let input: blast::BlastInput = serde_json::from_value(value.data)?;
                let results: clusterblast::ClusterBlastResults =
                    serde_json::from_value(value.results)?;
                JobType::ClusterBlast(clusterblast::ClusterBlast { input, results })
            }
            "comparippson" => {
                let input: blast::BlastInput = serde_json::from_value(value.data)?;
                let results: comparippson::CompaRiPPsonResults =
                    serde_json::from_value(value.results)?;
                JobType::CompaRiPPson(comparippson::CompaRiPPson { input, results })
            }
            "ping" => {
                let greeting: String = serde_json::from_value(value.data)?;
                let reply: Option<String> = serde_json::from_value(value.results).ok();
                JobType::Ping(ping::Ping { greeting, reply })
            }
            "storedquery" => {
                let input: stored_query::StoredQueryInput = serde_json::from_value(value.data)?;
                let filename: Option<String> = serde_json::from_value(value.results).ok();
                JobType::StoredQuery(stored_query::StoredQuery { input, filename })
            }
            _ => {
                return Err(Error::InvalidRequest(format!(
                    "Invalid jobtype {}",
                    value.jobtype
                )))
            }
        };
        Ok(Self {
            id: value.id,
            jobtype,
            status: JobStatus::from_str(&value.status).or(Err(Error::ParserError))?,
            runner: value.runner.unwrap_or_default(),
            submitted_date: value.submitted_date.and_utc(),
            version: value.version,
        })
    }
}

#[derive(Debug, Deserialize, Serialize)]
struct DbJob {
    pub id: String,
    pub jobtype: String,
    pub status: String,
    pub runner: Option<String>,
    pub submitted_date: NaiveDateTime,
    pub data: sqlx::types::JsonValue,
    pub results: sqlx::types::JsonValue,
    pub version: i32,
}

impl TryFrom<&JobEntry> for DbJob {
    type Error = Error;

    fn try_from(value: &JobEntry) -> std::result::Result<Self, Self::Error> {
        let (jobtype, data, results) = match value.jobtype.clone() {
            JobType::Ping(ping) => (
                "ping".to_string(),
                serde_json::to_value(ping.greeting)?,
                serde_json::to_value(ping.reply)?,
            ),
            JobType::CompaRiPPson(cr) => (
                "comparippson".to_string(),
                serde_json::to_value(cr.input)?,
                serde_json::to_value(cr.results)?,
            ),
            JobType::ClusterBlast(cb) => (
                "clusterblast".to_string(),
                serde_json::to_value(cb.input)?,
                serde_json::to_value(cb.results)?,
            ),
            JobType::StoredQuery(q) => (
                "storedquery".to_string(),
                serde_json::to_value(q.input)?,
                serde_json::to_value(q.filename)?,
            ),
        };

        Ok(Self {
            id: value.id.to_owned(),
            jobtype,
            status: value.status.to_string(),
            runner: Some(value.runner.to_owned()),
            submitted_date: value.submitted_date.naive_utc(),
            data,
            results,
            version: value.version,
        })
    }
}

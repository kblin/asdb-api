// License: GNU Affero General Public License v3 or later
// A copy of GNU AGPL v3 should have been included in this software package in LICENSE.txt.

use std::path::PathBuf;

use git_version::git_version;
use sqlx::PgPool;
use tokio::time::{sleep, Duration, Instant};

use crate::models::{
    control::Control,
    job::{JobEntry, JobStatus, JobType},
};
use crate::Result;

pub mod blast;
pub mod clusterblast;
pub mod comparippson;
pub mod ping;
pub mod stored_query;

const VERSION: &str = git_version!(cargo_prefix = "cargo:", fallback = "unknown");

pub async fn dispatch(pool: PgPool, config: RunConfig) -> Result<()> {
    let mut control = Control::new(&pool, &config.name, "running", false, VERSION)
        .commit()
        .await
        .expect("whoops");
    eprintln!("->> Starting loop");
    loop {
        if let Some(mut job) = JobEntry::next_pending(&pool).await? {
            job.runner = config.name.to_owned();
            job.status = JobStatus::Running;
            job.commit(&pool).await?;
            let start = Instant::now();
            job = run(job, &pool, &config).await?;
            let duration = start.elapsed();
            eprintln!("->> Processing job {} took {duration:?}", &job.id);
        }

        control.fetch().await?;
        if control.stop_scheduled {
            eprintln!("->> shutting down");
            return Ok(());
        }

        sleep(Duration::from_secs(1)).await;
    }
}

async fn run(mut job: JobEntry, pool: &PgPool, config: &RunConfig) -> Result<JobEntry> {
    match job.jobtype.clone() {
        JobType::ClusterBlast(cb) => {
            let completed = clusterblast::run(cb, config).await?;
            job.jobtype = JobType::ClusterBlast(completed);
        }
        JobType::CompaRiPPson(cr) => {
            let completed = comparippson::run(cr, config).await?;
            job.jobtype = JobType::CompaRiPPson(completed);
        }
        JobType::Ping(p) => {
            let completed_p = ping::run(p).await?;
            job.jobtype = JobType::Ping(completed_p);
        }
        JobType::StoredQuery(q) => {
            let completed_q = stored_query::run(q, pool, config).await?;
            job.jobtype = JobType::StoredQuery(completed_q);
        }
    }
    job.status = JobStatus::Done;
    job.commit(pool).await?;
    Ok(job)
}

#[derive(Debug, Clone)]
pub struct RunConfig {
    pub comparippson_config: comparippson::CompaRiPPsonConfig,
    pub dbdir: PathBuf,
    pub jobdir: PathBuf,
    pub outdir: Option<PathBuf>,
    pub name: String,
    pub urlroot: String,
}

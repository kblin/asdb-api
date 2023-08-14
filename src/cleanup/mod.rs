// License: GNU Affero General Public License v3 or later
// A copy of GNU AGPL v3 should have been included in this software package in LICENSE.txt.

use std::fs::remove_dir_all;
use std::path::PathBuf;

use sqlx::PgPool;

use crate::models::job::JobEntry;
use crate::Result;

pub async fn run(pool: &PgPool, job_base_dir: &PathBuf, days: f64) -> Result<()> {
    loop {
        let Some(job) = JobEntry::next_to_clean(pool, days).await? else {
            break;
        };

        let mut jobdir = job_base_dir.clone();
        jobdir.push(&job.id);

        if jobdir.exists() {
            eprintln!("Removing {jobdir:?}");
            remove_dir_all(jobdir)?;
        }

        eprintln!("Deleting job {}", job.id);
        job.delete(pool).await?;
    }

    eprintln!("Vacuuming the jobs table");
    sqlx::query!("VACUUM asdb_jobs.jobs").execute(pool).await?;
    eprintln!("Vacuuming the controls table");
    sqlx::query!("VACUUM asdb_jobs.controls")
        .execute(pool)
        .await?;

    Ok(())
}

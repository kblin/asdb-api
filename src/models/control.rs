// License: GNU Affero General Public License v3 or later
// A copy of GNU AGPL v3 should have been included in this software package in LICENSE.txt.

use sqlx::PgPool;

use crate::Result;

#[derive(Debug)]
pub struct Control<'a> {
    pool: &'a PgPool,
    pub name: String,
    pub status: String,
    pub stop_scheduled: bool,
    pub version: String,
}

impl<'a> Control<'a> {
    pub fn new(
        pool: &'a PgPool,
        name: &str,
        status: &str,
        stop_scheduled: bool,
        version: &str,
    ) -> Self {
        Self {
            pool,
            name: name.to_owned(),
            status: status.to_owned(),
            stop_scheduled,
            version: version.to_owned(),
        }
    }

    pub async fn from_db(pool: &'a PgPool, name: &str) -> Result<Control<'a>> {
        let row = sqlx::query!(
            r#"
        SELECT *  FROM asdb_jobs.controls
            WHERE name = $1"#,
            name,
        )
        .fetch_one(pool)
        .await?;

        Ok(Self {
            pool,
            name: row.name.to_owned(),
            status: row.status.to_owned(),
            stop_scheduled: row.stop_scheduled,
            version: row.version.to_owned(),
        })
    }

    pub async fn fetch(&mut self) -> Result<&mut Control<'a>> {
        let row = sqlx::query!(
            r#"
        SELECT status, stop_scheduled FROM asdb_jobs.controls
            WHERE name = $1"#,
            self.name,
        )
        .fetch_one(self.pool)
        .await?;
        self.status = row.status.to_owned();
        self.stop_scheduled = row.stop_scheduled;
        Ok(self)
    }

    pub async fn commit(self) -> Result<Control<'a>> {
        sqlx::query!(
            r#"
        INSERT INTO asdb_jobs.controls (name, status, stop_scheduled, version)
            VALUES ($1, $2, $3, $4)
        ON CONFLICT (name)
        DO UPDATE
            SET status = $2, stop_scheduled = $3, version = $4"#,
            self.name,
            self.status,
            self.stop_scheduled,
            self.version
        )
        .execute(self.pool)
        .await?;
        Ok(self)
    }

    pub async fn delete(self) -> Result<()> {
        sqlx::query!("DELETE FROM asdb_jobs.controls WHERE name = $1", self.name)
            .fetch_one(self.pool)
            .await?;
        Ok(())
    }
}

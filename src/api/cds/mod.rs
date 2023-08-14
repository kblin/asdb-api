// License: GNU Affero General Public License v3 or later
// A copy of GNU AGPL v3 should have been included in this software package in LICENSE.txt.

use serde::{Deserialize, Serialize};
use sqlx::PgPool;

use crate::Result;

pub struct CdsId {
    pub cds_id: i32,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct Cds {
    pub cds_id: i32,
    pub locus_tag: Option<String>,
    pub translation: Option<String>,
    pub accession: String,
    pub location: String,
}

impl Cds {
    pub fn csv_header() -> &'static str {
        return "#Locus tag\tAccession\tLocation\ttranslation";
    }

    pub fn to_csv(self) -> String {
        let parts = [
            self.locus_tag.unwrap_or_default(),
            self.accession,
            self.location,
            self.translation.unwrap_or_default(),
        ];
        parts.join("\t").to_string()
    }
}

pub async fn ids_to_genes(pool: &PgPool, ids: &[i32]) -> Result<Vec<Cds>> {
    let genes = sqlx::query_as!(
        Cds,
        r#"
    SELECT cds_id, locus_tag, translation, accession, c.location FROM antismash.cdss AS c
    JOIN antismash.regions USING (region_id)
    WHERE cds_id = ANY($1)
        "#,
        ids,
    )
    .fetch_all(pool)
    .await?;

    Ok(genes)
}

pub async fn ids_to_faa(pool: &PgPool, ids: &[i32]) -> Result<Vec<String>> {
    let mut fastas = Vec::with_capacity(ids.len());
    let rows = sqlx::query!(
        r#"
    SELECT cds_id, locus_tag, translation, accession, c.location FROM antismash.cdss AS c
    JOIN antismash.regions USING (region_id)
    WHERE cds_id = ANY($1)
        "#,
        ids,
    )
    .fetch_all(pool)
    .await?;

    for row in rows {
        fastas.push(format!(
            ">{}|{}|{}\n{}",
            row.locus_tag.unwrap_or("unknown_id".to_string()),
            row.accession,
            row.location,
            row.translation.unwrap_or_default(),
        ))
    }

    Ok(fastas)
}

pub async fn ids_to_fna(_pool: &PgPool, _ids: &[i32]) -> Result<Vec<String>> {
    // TODO: Implement this once antismash.cdss has start and end coordinates
    todo!()
}

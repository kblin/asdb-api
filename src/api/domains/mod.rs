// License: GNU Affero General Public License v3 or later
// A copy of GNU AGPL v3 should have been included in this software package in LICENSE.txt.

use serde::{Deserialize, Serialize};
use sqlx::PgPool;

use crate::Result;

pub struct DomainId {
    pub as_domain_id: i32,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct Domain {
    pub as_domain_id: i32,
    pub locus_tag: Option<String>,
    pub name: String,
    pub accession: String,
    pub version: Option<i32>,
    pub location: String,
    pub translation: Option<String>,
}

impl Domain {
    pub fn csv_header() -> &'static str {
        "#Locus tag\tDomain type\tAccession\tLocation\tSequence"
    }
    pub fn to_csv(self) -> String {
        let parts = [
            self.locus_tag.unwrap_or("Unknown locus tag".to_string()),
            self.name,
            format!("{}.{}", self.accession, self.version.unwrap_or(1)),
            self.location,
            self.translation.unwrap_or_default(),
        ];
        parts.join("\t")
    }
}

pub async fn ids_to_domains(pool: &PgPool, ids: &[i32]) -> Result<Vec<Domain>> {
    let domains = sqlx::query_as!(
        Domain,
        r#"
        SELECT as_domain_id, locus_tag, p.name, d.location, d.translation, accession, version FROM antismash.as_domains AS d
        JOIN antismash.cdss USING (cds_id)
        JOIN antismash.regions USING (region_id)
        JOIN antismash.dna_sequences USING (accession)
        JOIN antismash.as_domain_profiles AS p USING (as_domain_profile_id)
        WHERE as_domain_id = ANY($1)
        "#,
        ids
    ).fetch_all(pool).await?;

    Ok(domains)
}

pub async fn ids_to_faa(pool: &PgPool, ids: &[i32]) -> Result<Vec<String>> {
    let mut fastas = Vec::with_capacity(ids.len());

    let domains = ids_to_domains(pool, ids).await?;
    for domain in domains {
        fastas.push(format!(
            ">{}|{}|{}.{}|{}\n{}",
            domain.locus_tag.unwrap_or("unknown_locus_tag".to_string()),
            domain.name,
            domain.accession,
            domain.version.unwrap_or(1),
            domain.location,
            domain.translation.unwrap_or_default(),
        ));
    }

    Ok(fastas)
}

pub async fn ids_to_fna(_pool: &PgPool, _ids: &[i32]) -> Result<Vec<String>> {
    // TODO: Implement this once antismash.cdss has start and end coordinates
    todo!()
}

// License: GNU Affero General Public License v3 or later
// A copy of GNU AGPL v3 should have been included in this software package in LICENSE.txt.

use axum::{extract, response::Redirect, routing::get, Extension, Router};
use regex::Regex;
use sqlx::PgPool;

use crate::Result;

pub fn routes() -> Router {
    Router::new()
        .route("/api/goto/:identifier", get(goto))
        .route("/go/:identifier", get(goto))
        .route("/api/goto/:identifier/:region", get(goto_region))
        .route("/go/:identifier/:region", get(goto_region))
}

async fn canonical_id(pool: &PgPool, raw: String) -> Result<String> {
    let identifier = sanitise_id(&raw);

    // TODO: The old API had an "is it a v1 accession" check here
    // might need a more generic solution now that we're on v4

    // try the exact match first
    if let Ok(res) = sqlx::query!(
        r#"
    SELECT assembly_id FROM antismash.genomes
    WHERE assembly_id = $1"#,
        &identifier,
    )
    .fetch_one(pool)
    .await
    {
        return Ok(res.assembly_id);
    }

    // Maybe identifier lacks a version number?
    if let Ok(res) = sqlx::query!(
        r#"
    SELECT assembly_id FROM antismash.genomes
    WHERE assembly_id ILIKE $1
    ORDER BY assembly_id LIMIT 1"#,
        format!("{}%", &identifier),
    )
    .fetch_one(pool)
    .await
    {
        return Ok(res.assembly_id);
    }

    // Maybe it's a sequence accession, not an assembly ID

    // we store accessions and versions separately in dna_sequences
    if let Some((acc, ver)) = identifier.split_once('.') {
        let version: i32 = ver.parse().unwrap_or(1);
        if let Ok(res) = sqlx::query!(
            r#"
        SELECT assembly_id FROM antismash.genomes
        JOIN antismash.dna_sequences USING (genome_id)
        WHERE accession = $1 AND version = $2"#,
            acc,
            version,
        )
        .fetch_one(pool)
        .await
        {
            return Ok(res.assembly_id);
        }
    }

    if let Ok(res) = sqlx::query!(
        r#"
    SELECT assembly_id FROM antismash.genomes
    JOIN antismash.dna_sequences USING (genome_id)
    WHERE accession = $1"#,
        identifier,
    )
    .fetch_one(pool)
    .await
    {
        return Ok(res.assembly_id);
    }

    Err(crate::Error::NotFound)
}

async fn goto(
    Extension(pool): Extension<PgPool>,
    extract::Path(identifier): extract::Path<String>,
) -> Result<Redirect> {
    let id = canonical_id(&pool, identifier).await?;
    Ok(Redirect::to(&format!("/output/{id}/index.html")))
}

async fn goto_region(
    Extension(pool): Extension<PgPool>,
    extract::Path((identifier, region_raw)): extract::Path<(String, String)>,
) -> Result<Redirect> {
    let id = canonical_id(&pool, identifier).await?;
    let region = sanitise_region(&region_raw);
    eprintln!("->> {region_raw} -> {region}");
    Ok(Redirect::to(&format!("/output/{id}/index.html#{region}")))
}

fn sanitise_region(raw: &str) -> String {
    // we could do something fancier with a regex for rNcN and capture groups but this works
    Regex::new(r"[^cr0-9]")
        .unwrap()
        .replace_all(raw, "")
        .to_string()
}

pub fn sanitise_id(raw: &str) -> String {
    let safe_pattern = Regex::new(r"[^A-Za-z0-9_.]+").unwrap();
    safe_pattern.replace_all(raw, "").to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sanitise_region() {
        let tests = [
            ("r1c1", "r1c1".to_string()),
            ("bobr1c1eve", "r1c1".to_string()),
            ("bobr17alice23", "r17c23".to_string()),
        ];

        for (input, expected) in tests {
            let res = sanitise_region(input);
            assert_eq!(res, expected);
        }
    }
}

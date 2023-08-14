// License: GNU Affero General Public License v3 or later
// A copy of GNU AGPL v3 should have been included in this software package in LICENSE.txt.

use axum::{routing::get, Extension, Json, Router};
use serde::Serialize;
use serde_json::{json, Value};
use sqlx::PgPool;

use crate::Result;

pub fn routes() -> Router {
    Router::new()
        .route("/api/stats", get(stats))
        .route("/api/v2.0/stats", get(stats))
}

#[derive(Debug, Serialize)]
struct Stats {
    num_clusters: i64,
    num_genomes: i64,
    num_sequences: i64,
    top_seq_taxon: i32,
    top_seq_taxon_count: i64,
    top_seq_species: String,
    top_secmet_taxon: i32,
    top_secmet_taxon_count: f64,
    top_secmet_species: String,
    top_secmet_assembly_id: String,
    clusters: Vec<StatCluster>,
}

#[derive(Debug, Serialize)]
struct StatCluster {
    name: String,
    description: String,
    count: i64,
    category: String,
}

async fn stats(Extension(pool): Extension<PgPool>) -> Result<Json<Value>> {
    let num_clusters =
        sqlx::query!("SELECT COUNT(*) FROM antismash.regions WHERE contig_edge IS FALSE;")
            .fetch_one(&pool)
            .await?
            .count
            .unwrap_or(0);

    let num_genomes = sqlx::query!("SELECT COUNT(*) FROM antismash.genomes;")
        .fetch_one(&pool)
        .await?
        .count
        .unwrap_or(0);

    let num_sequences = sqlx::query!("SELECT COUNT(*) FROM antismash.dna_sequences")
        .fetch_one(&pool)
        .await?
        .count
        .unwrap_or(0);

    let top_seq_info = sqlx::query!(
        r#"
        SELECT tax_id, genus, species, strain, COUNT(accession) as tax_count
        FROM antismash.taxa
        JOIN antismash.genomes USING (tax_id)
        JOIN antismash.dna_sequences USING (genome_id)
        GROUP BY tax_id
        ORDER BY tax_count DESC
        LIMIT 1;
    "#
    )
    .fetch_one(&pool)
    .await?;

    let top_seq_taxon = top_seq_info.tax_id;
    let top_seq_taxon_count = top_seq_info.tax_count.unwrap_or(0);
    let top_seq_species = format!(
        "{} {} {}",
        top_seq_info.genus.unwrap_or_default(),
        top_seq_info.species.unwrap_or_default(),
        top_seq_info.strain.unwrap_or_default()
    );

    let secmet_info = sqlx::query!(
        r#"
        SELECT tax_id, genus, species, strain, assembly_id,
            (COUNT(DISTINCT region_number)::float / COUNT(DISTINCT assembly_id)) AS clusters_per_seq
        FROM antismash.taxa
        JOIN antismash.genomes USING (tax_id)
        JOIN antismash.dna_sequences USING (genome_id)
        JOIN antismash.regions USING (accession)
        GROUP BY tax_id, assembly_id
        ORDER BY clusters_per_seq DESC
        LIMIT 1;
    "#
    )
    .fetch_one(&pool)
    .await?;

    let top_secmet_taxon = secmet_info.tax_id;
    let top_secmet_species = format!(
        "{} {} {}",
        secmet_info.genus.unwrap_or_default(),
        secmet_info.species.unwrap_or_default(),
        secmet_info.strain.unwrap_or_default()
    );
    let top_secmet_taxon_count = secmet_info.clusters_per_seq.unwrap_or_default().round();
    let top_secmet_assembly_id = secmet_info.assembly_id;

    let clusters = sqlx::query!(
        r#"
        SELECT term, description, category, sub.count
            FROM antismash.bgc_types
            JOIN (
                SELECT bgc_type_id, COUNT(1) AS count
                FROM antismash.rel_regions_types GROUP BY bgc_type_id
            ) AS sub
            USING (bgc_type_id)
            ORDER BY sub.count DESC, term, category;"#
    )
    .fetch_all(&pool)
    .await?
    .iter()
    .map(|row| StatCluster {
        name: row.term.clone(),
        description: row.description.clone(),
        count: row.count.unwrap_or(0),
        category: row.category.clone(),
    })
    .collect();

    let stats: Stats = Stats {
        num_clusters,
        num_genomes,
        num_sequences,
        top_seq_taxon,
        top_seq_taxon_count,
        top_seq_species,
        top_secmet_taxon,
        top_secmet_taxon_count,
        top_secmet_species,
        top_secmet_assembly_id,
        clusters,
    };

    let body = Json(json!(stats));
    Ok(body)
}

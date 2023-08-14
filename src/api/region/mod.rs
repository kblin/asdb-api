// License: GNU Affero General Public License v3 or later
// A copy of GNU AGPL v3 should have been included in this software package in LICENSE.txt.

use std::collections::HashSet;

use async_recursion::async_recursion;
use axum::{extract, routing::get, Extension, Json, Router};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sqlx::PgPool;

use crate::api::go::sanitise_id;
use crate::query::{Operation, Operator, Query, ReturnType, Term};
use crate::Result;

pub mod area;
pub mod data;
pub mod expression;
pub mod modules;

pub use area::area;
pub use data::{DbRegion, Region};
pub use expression::handle_expression;

pub fn routes() -> Router {
    Router::new()
        .route("/api/assembly/:identifier", get(show_assembly))
        .route("/api/genome/:identifier", get(show_acc))
        .route("/api/area/:record/:location", get(area))
}

#[derive(Debug, Deserialize, Serialize)]
struct Reply {
    pub regions: Vec<Region>,
    pub offset: usize,
    pub paginate: usize,
    pub total: usize,
}

pub async fn search(
    pool: &PgPool,
    query: &Query,
    paginate: usize,
    offset: usize,
) -> Result<Json<Value>> {
    let value = match &query.return_type {
        ReturnType::Json => {
            let (total, all_regions) = core_search(pool, query).await?;

            let regions: Vec<Region>;
            if paginate > 0 {
                regions = Vec::from(&all_regions[offset..offset + paginate]);
            } else {
                regions = Vec::from(&all_regions[offset..]);
            }

            json!(Reply {
                regions,
                offset: 0,
                paginate: total,
                total
            })
        }
        _other => {
            let _ids = handle_term(pool, &query.terms).await?;
            todo!()
        }
    };
    Ok(Json(value))
}

async fn show_assembly(
    Extension(pool): Extension<PgPool>,
    extract::Path(identifier): extract::Path<String>,
) -> Result<Json<Value>> {
    let id = sanitise_id(&identifier);
    let query = Query::from_str(&format!("{{[assembly|{id}]}}"))?;
    let (_, regions) = core_search(&pool, &query).await?;

    Ok(Json(json!(regions)))
}

async fn show_acc(
    Extension(pool): Extension<PgPool>,
    extract::Path(identifier): extract::Path<String>,
) -> Result<Json<Value>> {
    let id = sanitise_id(&identifier);
    let query = Query::from_str(&format!("{{[acc|{id}]}}"))?;
    let (_, regions) = core_search(&pool, &query).await?;

    Ok(Json(json!(regions)))
}

pub struct RegionId {
    pub region_id: i32,
}

pub async fn core_search(pool: &PgPool, query: &Query) -> Result<(usize, Vec<Region>)> {
    let ids: Vec<i32> = handle_term(&pool, &query.terms).await?;
    let total = ids.len();
    let regions = ids_to_regions(pool, &ids).await?;
    Ok((total, regions))
}

pub async fn ids_to_regions(pool: &PgPool, ids: &[i32]) -> Result<Vec<Region>> {
    let regions = sqlx::query_as!(
            DbRegion,
            r#"
        SELECT region_id, region_number, record_number, start_pos, end_pos,
            accession, assembly_id, version, contig_edge, genus, species, strain,
            best_mibig_hit_similarity, best_mibig_hit_description, best_mibig_hit_acc,
            array_agg(t.term) AS terms, array_agg(t.description) AS descriptions, array_agg(t.category) AS categories
        FROM antismash.regions
        JOIN antismash.dna_sequences USING (accession)
        JOIN antismash.genomes USING (genome_id)
        JOIN antismash.taxa USING (tax_id)
        JOIN antismash.rel_regions_types USING (region_id)
        JOIN antismash.bgc_types AS t USING (bgc_type_id)
        WHERE region_id = ANY($1)
        GROUP BY region_id, region_number, record_number, start_pos, end_pos,
            accession, assembly_id, version, genus, species, strain,
            best_mibig_hit_similarity, best_mibig_hit_description, best_mibig_hit_acc
        ORDER BY region_id
        "#,
            ids,
        )
        .fetch_all(pool)
        .await?
        .into_iter()
        .map(|r| r.into())
        .collect();
    Ok(regions)
}

pub async fn ids_to_fasta(pool: &PgPool, ids: &[i32]) -> Result<Vec<String>> {
    let mut fastas = Vec::with_capacity(ids.len());
    let rows = sqlx::query!(
        r#"
    SELECT accession, version, start_pos, end_pos, genus, species, strain,
        SUBSTRING(dna FROM start_pos FOR end_pos - start_pos) AS sequence,
        array_agg(term) AS terms
    FROM antismash.regions
    JOIN antismash.dna_sequences USING (accession)
    JOIN antismash.genomes USING (genome_id)
    JOIN antismash.taxa USING (tax_id)
    JOIN antismash.rel_regions_types USING (region_id)
    JOIN antismash.bgc_types AS t USING (bgc_type_id)
    WHERE region_id = ANY($1)
    GROUP BY region_id, accession, version, start_pos, end_pos, genus, species, strain, sequence
    ORDER BY region_id
    "#,
        ids
    )
    .fetch_all(pool)
    .await?;

    for row in rows {
        fastas.push(format!(
            ">{}.{}|{}-{}|{} {} {}\n{}",
            row.accession,
            row.version.unwrap_or_default(),
            row.start_pos,
            row.end_pos,
            row.genus.unwrap_or_default(),
            row.species.unwrap_or_default(),
            row.strain.unwrap_or_default(),
            data::break_lines(&row.sequence.unwrap_or_default(), 80)
        ))
    }
    Ok(fastas)
}

async fn handle_term(pool: &PgPool, term: &Term) -> Result<Vec<i32>> {
    let ids = match term {
        Term::Expr(e) => handle_expression(pool, &e).await?,
        Term::Op(o) => handle_op(pool, &o).await?,
    };
    Ok(ids)
}

#[async_recursion]
async fn handle_op(pool: &PgPool, op: &Operation) -> Result<Vec<i32>> {
    let left_ids: HashSet<i32> = HashSet::from_iter(handle_term(pool, &op.left).await?.into_iter());
    let right_ids: HashSet<i32> =
        HashSet::from_iter(handle_term(pool, &op.right).await?.into_iter());

    let res = match op.operator {
        Operator::Except => left_ids
            .difference(&right_ids)
            .map(|i| *i)
            .collect::<Vec<i32>>(),
        Operator::Or => left_ids.union(&right_ids).map(|i| *i).collect::<Vec<i32>>(),
        Operator::And => left_ids
            .intersection(&right_ids)
            .map(|i| *i)
            .collect::<Vec<i32>>(),
    };
    Ok(res)
}

// License: GNU Affero General Public License v3 or later
// A copy of GNU AGPL v3 should have been included in this software package in LICENSE.txt.

use axum::{extract, Extension, Json};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sqlx::PgPool;

use super::{ids_to_regions, sanitise_id, Region, RegionId};
use crate::{Error, Result};

#[derive(Debug, Deserialize, Serialize)]
struct AreaResponse {
    pub regions: Vec<Region>,
}

pub async fn area(
    Extension(pool): Extension<PgPool>,
    extract::Path((accession, location)): extract::Path<(String, String)>,
) -> Result<Json<Value>> {
    let acc = sanitise_id(&accession);
    let (start, stop) = parse_location(&location)?;
    let ids: Vec<i32> = if let Some((a, v)) = acc.split_once(".") {
        let version: i32 = v.parse()?;
        sqlx::query_as!(
            RegionId,
            r#"
        SELECT region_id FROM antismash.regions
        JOIN antismash.dna_sequences USING (accession)
        WHERE accession = $1 AND version = $2 AND
        (
            start_pos BETWEEN $3 AND $4
            OR end_pos BETWEEN $3 AND $4
            OR $3 BETWEEN start_pos AND end_pos
            OR $4 BETWEEN start_pos AND end_pos
        )"#,
            a,
            version,
            start,
            stop,
        )
        .fetch_all(&pool)
        .await?
    } else {
        sqlx::query_as!(
            RegionId,
            r#"
        SELECT region_id FROM antismash.regions
        JOIN antismash.dna_sequences USING (accession)
        WHERE accession = $1 AND
        (
               start_pos BETWEEN $2 AND $3
            OR end_pos BETWEEN $2 AND $3
            OR $2 BETWEEN start_pos AND end_pos
            OR $3 BETWEEN start_pos AND end_pos
        )"#,
            acc,
            start,
            stop,
        )
        .fetch_all(&pool)
        .await?
    }
    .into_iter()
    .map(|r| r.region_id)
    .collect();

    let regions = ids_to_regions(&pool, &ids).await?;
    Ok(Json(json!(AreaResponse { regions })))
}

fn parse_location(location: &str) -> Result<(i32, i32)> {
    let Some((raw_start, raw_stop)) = location.split_once("-") else {
        return Err(Error::InvalidRequest(format!("Invalid location {location}")))
    };
    Ok((raw_start.parse()?, raw_stop.parse()?))
}

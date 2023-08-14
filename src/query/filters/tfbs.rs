// License: GNU Affero General Public License v3 or later
// A copy of GNU AGPL v3 should have been included in this software package in LICENSE.txt.

use sqlx::PgPool;

use crate::api::region::RegionId;
use crate::query::filters::Filter;

use crate::{Error, Result};

pub async fn tfbs_quality(
    pool: &PgPool,
    regions_to_filter: &[RegionId],
    filter: &Filter,
) -> Result<Vec<RegionId>> {
    let f = match filter {
        Filter::Qualitative(f) => f,
        invalid => {
            return Err(Error::InvalidRequest(format!(
                "tfbs query does not support {} filters",
                invalid.as_ref()
            )))
        }
    };

    let value = f.value.round() as i16;
    let r: Vec<i32> = regions_to_filter.iter().map(|r| r.region_id).collect();

    let regions: Vec<RegionId> = sqlx::query_as!(
        RegionId,
        r#"
    SELECT region_id FROM antismash.regions
    JOIN antismash.binding_sites USING (region_id)
    JOIN antismash.regulator_confidence USING (confidence_id)
    WHERE region_id = ANY($1) AND strength >= $2
        "#,
        &r,
        value,
    )
    .fetch_all(pool)
    .await?;

    Ok(regions)
}

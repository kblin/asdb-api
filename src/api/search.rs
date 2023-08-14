// License: GNU Affero General Public License v3 or later
// A copy of GNU AGPL v3 should have been included in this software package in LICENSE.txt.

use axum::{extract, routing::post, Extension, Json, Router};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sqlx::PgPool;

use super::region::search as region_search;
use crate::query::{Query, ReturnType, SearchType};
use crate::{Error, Result};

pub fn routes() -> Router {
    Router::new().route("/api/search", post(search))
}

#[derive(Debug, Deserialize, Serialize)]
struct SearchPayload {
    pub query: Query,
    pub offset: Option<usize>,
    pub paginate: Option<usize>,
}

async fn search(
    Extension(pool): Extension<PgPool>,
    extract::Json(req): extract::Json<SearchPayload>,
) -> Result<Json<Value>> {
    let offset = req.offset.unwrap_or(0);

    let paginate = req.paginate.unwrap_or(match &req.query.return_type {
        ReturnType::Json => 100,
        _ => 0,
    });

    let res = match req.query.search_type {
        SearchType::Region => region_search(&pool, &req.query, paginate, offset).await?,
        _ => {
            return Err(Error::NotImplementedError(format!(
                "{:?} searches",
                req.query.search_type
            )))
        }
    };
    Ok(res)
}

// License: GNU Affero General Public License v3 or later
// A copy of GNU AGPL v3 should have been included in this software package in LICENSE.txt.

use axum::{
    extract,
    routing::{get, post},
    Json, Router,
};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

use crate::query::{Query, ReturnType, SearchType, Term};
use crate::{Error, Result};

pub fn routes() -> Router {
    Router::new()
        .route("/api/convert", post(convert_post))
        .route("/api/convert", get(convert_get))
}

#[derive(Debug, Deserialize, Serialize)]
struct Payload {
    search_string: String,
    search_type: Option<SearchType>,
    return_type: Option<ReturnType>,
    verbose: Option<bool>,
}

async fn convert_post(extract::Json(payload): extract::Json<Payload>) -> Result<Json<Value>> {
    convert(payload)
}

async fn convert_get(extract::Query(payload): extract::Query<Payload>) -> Result<Json<Value>> {
    convert(payload)
}

fn convert(payload: Payload) -> Result<Json<Value>> {
    let search_type = payload.search_type.unwrap_or(SearchType::Region);
    let return_type = payload.return_type.unwrap_or(ReturnType::Json);
    let verbose = payload.verbose.unwrap_or(false);

    let query = match Term::parse(&payload.search_string) {
        Ok((_, term)) => Query {
            terms: term,
            search_type,
            return_type,
            verbose,
        },
        Err(_) => {
            return Err(Error::InvalidRequest(
                "failed to parse search string".to_string(),
            ))
        }
    };

    Ok(Json(json!(query)))
}

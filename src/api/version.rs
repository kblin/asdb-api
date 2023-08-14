// License: GNU Affero General Public License v3 or later
// A copy of GNU AGPL v3 should have been included in this software package in LICENSE.txt.

use axum::{routing::get, Json, Router};
use serde_json::{json, Value};

use crate::Result;

pub fn routes() -> Router {
    Router::new().route("/api/version", get(version))
}

const VERSION: &'static str = env!("CARGO_PKG_VERSION");

async fn version() -> Result<Json<Value>> {
    Ok(Json(json!({"api": VERSION})))
}

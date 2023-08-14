// License: GNU Affero General Public License v3 or later
// A copy of GNU AGPL v3 should have been included in this software package in LICENSE.txt.

pub mod available;
pub mod cds;
pub mod convert;
pub mod domains;
pub mod go;
pub mod job;
pub mod region;
pub mod search;
pub mod stats;
pub mod taxa;
pub mod version;

use axum::{Extension, Router};
use sqlx::PgPool;

pub fn init_routes(pool: PgPool) -> Router {
    Router::new()
        .merge(available::routes())
        .merge(convert::routes())
        .merge(go::routes())
        .merge(job::routes())
        .merge(region::routes())
        .merge(search::routes())
        .merge(stats::routes())
        .merge(taxa::routes())
        .merge(version::routes())
        .layer(Extension(pool))
}

// License: GNU Affero General Public License v3 or later
// A copy of GNU AGPL v3 should have been included in this software package in LICENSE.txt.

use std::collections::HashMap;
use std::str::FromStr;

use axum::{extract, routing::get, Extension, Json, Router};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sqlx::PgPool;
use strum::IntoEnumIterator;

use crate::search::category::{Category, CategoryGroup, CategoryType};
use crate::search::filters::{get_filters_by_category, AvailableFilter};
use crate::{Error, Result};

mod terms;

pub fn routes() -> Router {
    Router::new()
        .route(
            "/api/available/term/:category/:term",
            get(terms::available_terms_by_category),
        )
        .route("/api/available/categories", get(available_categories))
        .route(
            "/api/available/filters/:category",
            get(available_filters_by_category),
        )
        .route(
            "/api/available/filter_values/:category/:filter_name",
            get(available_filter_values_by_category),
        )
}

#[derive(Debug, Serialize)]
pub struct CategoryInfo {
    pub label: &'static str,
    pub value: &'static str,
    #[serde(rename = "type")]
    pub category_type: CategoryType,
    pub countable: bool,
    pub description: &'static str,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub filters: Vec<AvailableFilter>,
}

#[derive(Debug, Serialize)]
pub struct AvailableCategoryGroup {
    pub header: CategoryGroup,
    pub options: Vec<CategoryInfo>,
}

#[derive(Debug, Serialize)]
pub struct AvailableCategories {
    pub options: Vec<CategoryInfo>,
    pub groups: Vec<AvailableCategoryGroup>,
}

pub fn get_available_categories() -> AvailableCategories {
    let mut options: Vec<CategoryInfo> = Vec::new();
    let mut group_map: HashMap<CategoryGroup, Vec<CategoryInfo>> = HashMap::new();

    for cat in Category::iter() {
        let label = cat.get_label();
        let value: &'static str = cat.clone().into();
        let category_type = cat.get_type();
        let countable = cat.is_countable();
        let description = cat.get_description();
        let filters = cat.get_filters();

        let info = CategoryInfo {
            label,
            value,
            category_type,
            countable,
            description,
            filters,
        };

        if let Some(group) = cat.get_group() {
            match group_map.get_mut(&group) {
                Some(v) => v.push(info),
                None => {
                    let mut v: Vec<CategoryInfo> = Vec::new();
                    v.push(info);
                    group_map.insert(group, v);
                }
            }
        } else {
            options.push(info);
        }
    }

    let mut groups: Vec<AvailableCategoryGroup> = Vec::with_capacity(CategoryGroup::iter().count());
    for group in CategoryGroup::iter() {
        let options = match group_map.remove(&group) {
            Some(v) => v,
            None => Vec::new(),
        };
        groups.push(AvailableCategoryGroup {
            header: group,
            options,
        })
    }

    AvailableCategories { options, groups }
}

async fn available_categories(Extension(_pool): Extension<PgPool>) -> Result<Json<Value>> {
    Ok(Json(json!(get_available_categories())))
}

#[derive(Debug, Deserialize, Serialize)]
pub struct AvailableTerm {
    #[serde(rename = "val")]
    pub name: Option<String>,
    #[serde(rename = "desc")]
    pub description: Option<String>,
}

async fn available_filters_by_category(
    Extension(_pool): Extension<PgPool>,
    extract::Path(raw_category): extract::Path<String>,
) -> Result<Json<Value>> {
    let category = Category::from_str(&raw_category)?;
    Ok(Json(json!(get_filters_by_category(&category))))
}

async fn available_filter_values_by_category(
    Extension(_pool): Extension<PgPool>,
    extract::Path((_category, _filter_name)): extract::Path<(String, String)>,
) -> Result<Json<Value>> {
    Err(Error::NotImplementedError(
        "filters are not implemented yet".to_string(),
    ))
}

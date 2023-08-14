// License: GNU Affero General Public License v3 or later
// A copy of GNU AGPL v3 should have been included in this software package in LICENSE.txt.

use serde::Serialize;

use crate::search::Category;

#[derive(Debug, Serialize)]
pub struct AvailableFilter {
    pub value: String,
    pub label: String,
    #[serde(rename = "type")]
    pub data_type: String,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub choices: Vec<(String, u32)>,
}

impl AvailableFilter {
    pub fn new(value: &str, label: &str, data_type: &str) -> Self {
        Self {
            value: value.to_string(),
            label: label.to_string(),
            data_type: data_type.to_string(),
            choices: Vec::new(),
        }
    }

    pub fn add_choice(mut self, label: &str, value: u32) -> Self {
        self.choices.push((label.to_owned(), value));
        self
    }
}

pub fn get_filters_by_category(category: &Category) -> Vec<AvailableFilter> {
    match category {
        Category::CandidateKind => {
            let mut filters = Vec::new();
            filters.push(AvailableFilter::new("bgctype", "BGC Type", "text"));
            filters.push(AvailableFilter::new(
                "numprotoclusters",
                "Protocluster count",
                "numerical",
            ));
            filters
        }
        Category::Tfbs => {
            let mut filters = Vec::new();
            filters.push(AvailableFilter::new("score", "Score", "numeric"));
            filters.push(
                AvailableFilter::new("quality", "Quality", "qualitative")
                    .add_choice("strong", 30)
                    .add_choice("medium", 20)
                    .add_choice("weak", 10),
            );
            filters
        }
        _ => return Vec::new(),
    }
}

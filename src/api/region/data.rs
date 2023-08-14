// License: GNU Affero General Public License v3 or later
// A copy of GNU AGPL v3 should have been included in this software package in LICENSE.txt.
use std::convert::From;

use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct Region {
    #[serde(rename = "bgc_id")]
    pub region_id: i32,
    pub record_number: i32,
    pub region_number: i32,

    pub start_pos: i32,
    pub end_pos: i32,
    pub contig_edge: bool,

    #[serde(rename = "acc")]
    pub accession: Option<String>,
    pub assembly_id: Option<String>,
    pub version: Option<i32>,

    pub genus: Option<String>,
    pub species: Option<String>,
    pub strain: Option<String>,

    pub term: String,
    pub description: String,
    pub category: String,

    pub best_mibig_hit_similarity: Option<i32>,
    pub best_mibig_hit_description: Option<String>,
    pub best_mibig_hit_acc: Option<String>,
}

impl Region {
    pub fn csv_header() -> &'static str {
        "#Genus\tSpecies\tStrain\tNCBI accession\tFrom\tTo\tBGC type\tOn contig edge\tMost similar known cluster\tSimilarity in %\tMIBiG BGC-ID\tResults URL"
    }

    pub fn to_csv(self) -> String {
        let acc_with_version = format!(
            "{}.{}",
            self.accession.unwrap_or_default(),
            self.version.unwrap_or_default()
        );
        let parts = [
            self.genus.unwrap_or_default(),
            self.species.unwrap_or_default(),
            self.strain.unwrap_or_default(),
            acc_with_version.clone(),
            format!("{}", self.start_pos),
            format!("{}", self.end_pos),
            self.term,
            format!("{}", self.contig_edge),
            self.best_mibig_hit_description.unwrap_or_default(),
            format!("{}", self.best_mibig_hit_similarity.unwrap_or_default()),
            self.best_mibig_hit_acc.unwrap_or_default(),
            format!(
                "https://antismash-db.secondarymetabolites.org/area?record={}&start={}&end={}",
                acc_with_version, self.start_pos, self.end_pos
            ),
        ];

        parts.join("\t").to_string()
    }
}

#[derive(Debug, Deserialize, Serialize)]
pub struct DbRegion {
    pub region_id: i32,
    pub record_number: i32,
    pub region_number: i32,

    pub start_pos: i32,
    pub end_pos: i32,
    pub contig_edge: bool,

    pub accession: Option<String>,
    pub assembly_id: Option<String>,
    pub version: Option<i32>,

    pub genus: Option<String>,
    pub species: Option<String>,
    pub strain: Option<String>,

    pub terms: Option<Vec<String>>,
    pub descriptions: Option<Vec<String>>,
    pub categories: Option<Vec<String>>,

    pub best_mibig_hit_similarity: Option<i32>,
    pub best_mibig_hit_description: Option<String>,
    pub best_mibig_hit_acc: Option<String>,
}

impl From<DbRegion> for Region {
    fn from(value: DbRegion) -> Self {
        let term = if let Some(terms) = value.terms {
            if terms.len() == 1 {
                terms[0].to_owned()
            } else {
                format!("{} hybrid", terms.join(" "))
            }
        } else {
            "".to_string()
        };
        let description: String = if let Some(descs) = value.descriptions {
            if descs.len() == 1 {
                descs[0].to_string()
            } else {
                format!("Hybrid region: {}", descs.join(", "))
            }
        } else {
            "".to_string()
        };
        let categories = value.categories.unwrap_or(Vec::new());
        let category = if categories.len() == 1 {
            categories[0].to_owned()
        } else {
            "hybrid".to_string()
        };
        Self {
            region_id: value.region_id,
            record_number: value.record_number,
            region_number: value.region_number,
            start_pos: value.start_pos,
            end_pos: value.end_pos,
            contig_edge: value.contig_edge,
            accession: value.accession,
            assembly_id: value.assembly_id,
            version: value.version,
            genus: value.genus,
            species: value.species,
            strain: value.strain,
            term,
            description,
            category,
            best_mibig_hit_similarity: value.best_mibig_hit_similarity,
            best_mibig_hit_description: value.best_mibig_hit_description,
            best_mibig_hit_acc: value.best_mibig_hit_acc,
        }
    }
}

pub fn break_lines(input: &str, line_length: usize) -> String {
    let mut parts: Vec<&str> = Vec::with_capacity(input.len() / line_length);
    let mut iter = input.chars();
    let mut pos = 0;

    while pos < input.len() {
        let mut len = 0;
        for ch in iter.by_ref().take(line_length) {
            len += ch.len_utf8();
        }
        parts.push(&input[pos..pos + len]);
        pos += len;
    }

    parts.join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_break_lines() {
        let tests = [("ABCDE", 3, "ABC\nDE"), ("ABCDE", 10, "ABCDE")];

        for (input, line_length, expected) in tests {
            let result = break_lines(input, line_length);
            assert_eq!(result, expected);
        }
    }
}

// License: GNU Affero General Public License v3 or later
// A copy of GNU AGPL v3 should have been included in this software package in LICENSE.txt.

use std::convert::From;

use axum::{extract, Extension, Json};
use serde_json::{json, Value};
use sqlx::PgPool;

use crate::search::category::Category;
use crate::{Error, Result};

use super::AvailableTerm;

pub struct PossibleTermNoDesc {
    pub name: Option<String>,
}

impl From<&PossibleTermNoDesc> for AvailableTerm {
    fn from(value: &PossibleTermNoDesc) -> Self {
        let name = value.name.clone().unwrap_or("Unknown".to_string());
        Self {
            name: Some(name),
            description: None,
        }
    }
}

pub async fn available_terms_by_category(
    Extension(pool): Extension<PgPool>,
    extract::Path((cat, term)): extract::Path<(String, String)>,
) -> Result<Json<Value>> {
    let category = match Category::parse(&cat) {
        Ok((_, c)) => c,
        Err(e) => return Err(Error::InvalidRequest(format!("{e}"))),
    };

    let available = match category {
        Category::Acc => {
            sqlx::query_as!(
                AvailableTerm,
                r#"
        SELECT DISTINCT accession AS name, NULL AS description FROM antismash.dna_sequences
        WHERE accession ILIKE $1
        ORDER BY accession LIMIT 50"#,
                format!("{term}%"),
            )
            .fetch_all(&pool)
            .await?
        }
        Category::Assembly => {
            sqlx::query_as!(
                AvailableTerm,
                r#"
        SELECT DISTINCT assembly_id AS name, NULL AS description FROM antismash.genomes
        WHERE assembly_id ILIKE $1
        ORDER BY assembly_id LIMIT 50"#,
                format!("{term}%"),
            )
            .fetch_all(&pool)
            .await?
        }
        Category::Type => {
            sqlx::query_as!(
                AvailableTerm,
                r#"
        SELECT DISTINCT term AS name, description FROM antismash.bgc_types
        WHERE term ILIKE $1 OR description ILIKE $1
        ORDER BY term LIMIT 50"#,
                format!("{term}%"),
            )
            .fetch_all(&pool)
            .await?
        }
        Category::TypeCategory => {
            sqlx::query_as!(
                AvailableTerm,
                r#"
        SELECT DISTINCT category AS name, description FROM antismash.bgc_categories
        WHERE category ILIKE $1 OR description ILIKE $1
        ORDER BY category LIMIT 50"#,
                format!("{term}%"),
            )
            .fetch_all(&pool)
            .await?
        }
        Category::CandidateKind => {
            sqlx::query_as!(
                AvailableTerm,
                r#"
        SELECT DISTINCT description AS name, description FROM antismash.candidate_types
        WHERE description ILIKE $1
        ORDER BY description LIMIT 50"#,
                format!("{term}%"),
            )
            .fetch_all(&pool)
            .await?
        }
        Category::Substrate => sqlx::query_as!(
            AvailableTerm,
            r#"
        SELECT DISTINCT name, description FROM antismash.substrates
        WHERE name ILIKE $1 OR description ILIKE $1
        ORDER BY name LIMIT 50"#,
            format!("{term}%"),
        )
        .fetch_all(&pool)
        .await?,
        Category::Monomer => {
            sqlx::query_as!(
                AvailableTerm,
                r#"
        SELECT DISTINCT name, description FROM antismash.monomers
        WHERE name ILIKE $1 OR description ILIKE $1
        ORDER BY name LIMIT 50"#,
                format!("{term}%"),
            )
            .fetch_all(&pool)
            .await?
        }
        Category::Profile => {
            sqlx::query_as!(
                AvailableTerm,
                r#"
        SELECT DISTINCT name, description FROM antismash.profiles
        WHERE name ILIKE $1 OR description ILIKE $1
        ORDER BY name LIMIT 50"#,
                format!("{term}%"),
            )
            .fetch_all(&pool)
            .await?
        }
        Category::Resfam => {
            sqlx::query_as!(
                AvailableTerm,
                r#"
        SELECT DISTINCT name, description FROM antismash.resfams
        WHERE name ILIKE $1 OR accession ILIKE $1 OR description ILIKE $2
        ORDER BY name LIMIT 50"#,
                format!("{term}%"),
                format!("%{term}%"),
            )
            .fetch_all(&pool)
            .await?
        }
        Category::Pfam => {
            sqlx::query_as!(
                AvailableTerm,
                r#"
        SELECT DISTINCT name, description FROM antismash.pfams
        WHERE name ILIKE $1 OR pfam_id ILIKE $1 OR description ILIKE $2
        ORDER BY name LIMIT 50"#,
                format!("{term}%"),
                format!("%{term}%"),
            )
            .fetch_all(&pool)
            .await?
        }
        Category::Tigrfam => {
            sqlx::query_as!(
                AvailableTerm,
                r#"
        SELECT DISTINCT name, description FROM antismash.tigrfams
        WHERE name ILIKE $1 OR description ILIKE $2
        ORDER BY name LIMIT 50"#,
                format!("{term}%"),
                format!("%{term}%"),
            )
            .fetch_all(&pool)
            .await?
        }
        Category::GOTerm => {
            sqlx::query_as!(
                AvailableTerm,
                r#"
        SELECT DISTINCT identifier AS name, description FROM antismash.gene_ontologies
        WHERE identifier ILIKE $1 OR description ILIKE $2
        ORDER BY identifier LIMIT 50"#,
                format!("{term}%"),
                format!("%{term}%"),
            )
            .fetch_all(&pool)
            .await?
        }
        Category::AsDomain => {
            sqlx::query_as!(
                AvailableTerm,
                r#"
        SELECT DISTINCT name, description FROM antismash.as_domain_profiles
        WHERE name ILIKE $1 OR description ILIKE $2
        ORDER BY name LIMIT 50"#,
                format!("{term}%"),
                format!("%{term}%"),
            )
            .fetch_all(&pool)
            .await?
        }
        Category::AsDomainSubtype => {
            sqlx::query_as!(
                AvailableTerm,
                r#"
        SELECT DISTINCT subtype AS name, description FROM antismash.as_domain_subtypes
        WHERE subtype ILIKE $1 OR description ILIKE $2
        ORDER BY subtype LIMIT 50"#,
                format!("{term}%"),
                format!("%{term}%"),
            )
            .fetch_all(&pool)
            .await?
        }
        Category::ModuleQuery | Category::CrossCdsModule | Category::ContigEdge | Category::T2pksElongation => {
            return Err(Error::InvalidRequest(format!(
                "No terms available for {category}"
            )))
        }
        Category::T2pksProductClass => {
            sqlx::query_as!(
                AvailableTerm,
                r#"
        SELECT DISTINCT product_class AS name, NULL as description FROM antismash.t2pks_product_classes
        WHERE product_class ILIKE $1
        ORDER BY product_class LIMIT 50"#,
                format!("{term}%"),
            )
            .fetch_all(&pool)
            .await?
        }
        Category::T2pksStarter => {
            sqlx::query_as!(
                AvailableTerm,
                r#"
        SELECT DISTINCT name, NULL as description FROM antismash.t2pks_starters
        WHERE name ILIKE $1
        ORDER BY name LIMIT 50"#,
                format!("{term}%"),
            )
            .fetch_all(&pool)
            .await?
        }
        Category::T2pksProfile => {
            sqlx::query_as!(
                AvailableTerm,
                r#"
        SELECT DISTINCT name, description FROM antismash.t2pks_profiles
        WHERE name ILIKE $1 OR description ILIKE $2
        ORDER BY name LIMIT 50"#,
                format!("{term}%"),
                format!("%{term}%"),
            )
            .fetch_all(&pool)
            .await?
        }
        Category::SmCoG => {
            sqlx::query_as!(
                AvailableTerm,
                r#"
        SELECT DISTINCT name, description FROM antismash.smcogs
        WHERE name ILIKE $1 OR description ILIKE $2
        ORDER BY name LIMIT 50"#,
                format!("{term}%"),
                format!("%{term}%"),
            )
            .fetch_all(&pool)
            .await?
        }
        Category::Tfbs => {
            sqlx::query_as!(
                AvailableTerm,
                r#"
        SELECT DISTINCT name, description FROM antismash.regulators
        WHERE name ILIKE $1 OR description ILIKE $2
        ORDER BY name LIMIT 50"#,
                format!("{term}%"),
                format!("%{term}%"),
            )
            .fetch_all(&pool)
            .await?
        }
        | Category::CompoundSeq => {
            sqlx::query_as!(
                PossibleTermNoDesc,
                r#"
        SELECT DISTINCT peptide_sequence AS name FROM antismash.ripps
        WHERE peptide_sequence ILIKE $1
        ORDER BY peptide_sequence LIMIT 50"#,
                format!("{term}%"),
            )
            .fetch_all(&pool)
            .await?.iter()
            .map(|v| v.into())
            .collect()
        }
        Category::CompoundClass => {
            sqlx::query_as!(
                PossibleTermNoDesc,
                r#"
        SELECT DISTINCT subclass AS name FROM antismash.ripps
        WHERE subclass ILIKE $1
        ORDER BY subclass LIMIT 50"#,
                format!("{term}%"),
            )
            .fetch_all(&pool)
            .await?.iter()
            .map(|v| v.into())
            .collect()
        }
        Category::Strain => sqlx::query_as!(
            PossibleTermNoDesc,
            r#"
        SELECT DISTINCT strain AS name FROM antismash.taxa
        WHERE strain ILIKE $1
        ORDER BY strain LIMIT 50"#,
            format!("{term}%"),
        )
        .fetch_all(&pool)
        .await?
        .iter()
        .map(|v| v.into())
        .collect(),
        Category::Species => sqlx::query_as!(
            PossibleTermNoDesc,
            r#"
        SELECT DISTINCT species AS name FROM antismash.taxa
        WHERE species ILIKE $1
        ORDER BY species LIMIT 50"#,
            format!("{term}%"),
        )
        .fetch_all(&pool)
        .await?
        .iter()
        .map(|v| v.into())
        .collect(),
        Category::Genus => sqlx::query_as!(
            PossibleTermNoDesc,
            r#"
        SELECT DISTINCT genus AS name FROM antismash.taxa
        WHERE genus ILIKE $1
        ORDER BY genus LIMIT 50"#,
            format!("{term}%"),
        )
        .fetch_all(&pool)
        .await?
        .iter()
        .map(|v| v.into())
        .collect(),
        Category::Family => sqlx::query_as!(
            PossibleTermNoDesc,
            r#"
        SELECT DISTINCT family AS name FROM antismash.taxa
        WHERE family ILIKE $1
        ORDER BY family LIMIT 50"#,
            format!("{term}%"),
        )
        .fetch_all(&pool)
        .await?
        .iter()
        .map(|v| v.into())
        .collect(),
        Category::Order => sqlx::query_as!(
            PossibleTermNoDesc,
            r#"
        SELECT DISTINCT taxonomic_order AS name FROM antismash.taxa
        WHERE taxonomic_order ILIKE $1
        ORDER BY taxonomic_order LIMIT 50"#,
            format!("{term}%"),
        )
        .fetch_all(&pool)
        .await?
        .iter()
        .map(|v| v.into())
        .collect(),
        Category::Class => sqlx::query_as!(
            PossibleTermNoDesc,
            r#"
        SELECT DISTINCT class AS name FROM antismash.taxa
        WHERE class ILIKE $1
        ORDER BY class LIMIT 50"#,
            format!("{term}%"),
        )
        .fetch_all(&pool)
        .await?
        .iter()
        .map(|v| v.into())
        .collect(),
        Category::Phylum => sqlx::query_as!(
            PossibleTermNoDesc,
            r#"
        SELECT DISTINCT phylum AS name FROM antismash.taxa
        WHERE phylum ILIKE $1
        ORDER BY phylum LIMIT 50"#,
            format!("{term}%"),
        )
        .fetch_all(&pool)
        .await?
        .iter()
        .map(|v| v.into())
        .collect(),
        Category::Superkingdom => sqlx::query_as!(
            PossibleTermNoDesc,
            r#"
        SELECT DISTINCT superkingdom AS name FROM antismash.taxa
        WHERE superkingdom ILIKE $1
        ORDER BY superkingdom LIMIT 50"#,
            format!("{term}%"),
        )
        .fetch_all(&pool)
        .await?
        .iter()
        .map(|v| v.into())
        .collect(),
        Category::CompaRiPPsonMibig => {
            sqlx::query_as!(
                AvailableTerm,
                r#"
        SELECT DISTINCT accession AS name, product AS description FROM antismash.comparippson_mibig_references
        WHERE name ILIKE $1 OR accession ILIKE $2 OR product ILIKE $2
        ORDER BY name LIMIT 50"#,
                format!("{term}%"),
                format!("%{term}%"),
            )
            .fetch_all(&pool)
            .await?
        }
        Category::ClusterCompareRegion
        | Category::ClusterCompareProtocluster => {
            sqlx::query_as!(
                AvailableTerm,
                r#"
        SELECT DISTINCT reference_accession AS name, description FROM antismash.cluster_compare_hits
        WHERE reference_accession ILIKE $1 OR description ILIKE $2
        ORDER BY reference_accession LIMIT 50"#,
                format!("{term}%"),
                format!("%{term}%"),
            )
            .fetch_all(&pool)
            .await?
        }
        Category::ClusterBlast => {
            sqlx::query_as!(
                AvailableTerm,
                r#"
        SELECT DISTINCT acc AS name, description FROM antismash.clusterblast_hits
        JOIN antismash.clusterblast_algorithms USING (algorithm_id)
        WHERE name = 'clusterblast' AND (acc ILIKE $1 OR description ILIKE $2)
        ORDER BY acc LIMIT 50"#,
                format!("{term}%"),
                format!("%{term}%"),
            )
            .fetch_all(&pool)
            .await?
        }
        | Category::KnownCluster => {
            sqlx::query_as!(
                AvailableTerm,
                r#"
        SELECT DISTINCT acc AS name, description FROM antismash.clusterblast_hits
        JOIN antismash.clusterblast_algorithms USING (algorithm_id)
        WHERE name = 'knownclusterblast' AND (acc ILIKE $1 OR description ILIKE $2)
        ORDER BY acc LIMIT 50"#,
                format!("{term}%"),
                format!("%{term}%"),
            )
            .fetch_all(&pool)
            .await?
        }
        | Category::SubCluster => {
            sqlx::query_as!(
                AvailableTerm,
                r#"
        SELECT DISTINCT acc AS name, description FROM antismash.clusterblast_hits
        JOIN antismash.clusterblast_algorithms USING (algorithm_id)
        WHERE name = 'subclusterblast' AND (acc ILIKE $1 OR description ILIKE $2)
        ORDER BY acc LIMIT 50"#,
                format!("{term}%"),
                format!("%{term}%"),
            )
            .fetch_all(&pool)
            .await?
        }
    };

    Ok(Json(json!(available)))
}

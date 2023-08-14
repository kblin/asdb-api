// License: GNU Affero General Public License v3 or later
// A copy of GNU AGPL v3 should have been included in this software package in LICENSE.txt.

use sqlx::PgPool;
use strum;

use crate::query::Expression;
use crate::search::category::Category;
use crate::{Error, Result};

use crate::query::filters::tfbs;

use super::RegionId;

pub async fn handle_expression(pool: &PgPool, expr: &Expression) -> Result<Vec<i32>> {
    let region_ids = match expr.category {
        Category::Acc => {
            if let Some((acc, ver)) = expr.value.split_once(".") {
                let version: i32 = ver.parse()?;
                sqlx::query_as!(
                    RegionId,
                    r#"
            SELECT region_id FROM antismash.regions
            JOIN antismash.dna_sequences USING (accession)
            WHERE accession = $1 AND version = $2
                    "#,
                    acc,
                    version,
                )
                .fetch_all(pool)
                .await?
            } else {
                sqlx::query_as!(
                    RegionId,
                    r#"
            SELECT region_id FROM antismash.regions
            WHERE accession = $1
            "#,
                    expr.value,
                )
                .fetch_all(pool)
                .await?
            }
        }
        Category::Assembly => {
            sqlx::query_as!(
                RegionId,
                r#"
            SELECT region_id FROM antismash.regions
            JOIN antismash.dna_sequences USING (accession)
            JOIN antismash.genomes USING (genome_id)
            WHERE assembly_id = $1
                "#,
                expr.value
            )
            .fetch_all(pool)
            .await?
        }
        Category::Type => {
            sqlx::query_as!(
                RegionId,
                r#"
            SELECT region_id FROM antismash.regions
            JOIN antismash.rel_regions_types USING (region_id)
            JOIN antismash.bgc_types USING (bgc_type_id)
            WHERE term = $1
            GROUP BY region_id HAVING COUNT(*) >= $2
            "#,
                expr.value,
                expr.count,
            )
            .fetch_all(pool)
            .await?
        }
        Category::TypeCategory => {
            sqlx::query_as!(
                RegionId,
                r#"
            SELECT region_id FROM antismash.regions
            JOIN antismash.rel_regions_types USING (region_id)
            JOIN antismash.bgc_types USING (bgc_type_id)
            WHERE category = $1
            GROUP BY region_id HAVING COUNT(*) >= $2
            "#,
                expr.value,
                expr.count,
            )
            .fetch_all(pool)
            .await?
        }
        Category::CandidateKind => {
            sqlx::query_as!(
                RegionId,
                r#"
            SELECT region_id FROM antismash.regions
            JOIN antismash.candidates USING (region_id)
            JOIN antismash.candidate_types USING (candidate_type_id)
            WHERE description ILIKE $1
            GROUP BY region_id HAVING COUNT(*) >= $2
            "#,
                format!("%{}%", expr.value),
                expr.count,
            )
            .fetch_all(pool)
            .await?
        }
        Category::Substrate => {
            sqlx::query_as!(
                RegionId,
                r#"
            SELECT region_id FROM antismash.regions
            JOIN antismash.modules USING (region_id)
            JOIN antismash.rel_modules_monomers AS r_m_m USING (module_id)
            JOIN antismash.substrates AS substrates ON (r_m_m.substrate = substrates.substrate_id)
            WHERE substrates.name ILIKE $1
            GROUP BY region_id HAVING COUNT(*) >= $2
            "#,
                expr.value,
                expr.count,
            )
            .fetch_all(pool)
            .await?
        }
        Category::Monomer => {
            sqlx::query_as!(
                RegionId,
                r#"
            SELECT region_id FROM antismash.regions
            JOIN antismash.modules USING (region_id)
            JOIN antismash.rel_modules_monomers AS r_m_m USING (module_id)
            JOIN antismash.monomers AS monomers ON (r_m_m.monomer = monomers.monomer_id)
            WHERE monomers.name ILIKE $1
            GROUP BY region_id HAVING COUNT(*) >= $2
            "#,
                expr.value,
                expr.count,
            )
            .fetch_all(pool)
            .await?
        }
        Category::Profile => {
            sqlx::query_as!(
                RegionId,
                r#"
            SELECT region_id FROM antismash.regions
            JOIN antismash.cdss AS cds USING (region_id)
            JOIN antismash.profile_hits AS ph USING (cds_id)
            WHERE ph.name ILIKE $1
            GROUP BY region_id HAVING COUNT(*) >= $2
                "#,
                expr.value,
                expr.count,
            )
            .fetch_all(pool)
            .await?
        }
        Category::Resfam => {
            sqlx::query_as!(
                RegionId,
                r#"
            SELECT region_id FROM antismash.regions
            JOIN antismash.cdss USING (region_id)
            JOIN antismash.resfam_domains USING (cds_id)
            JOIN antismash.resfams AS resfam USING (resfam_id)
            WHERE (
                resfam.accession ILIKE $1
                OR resfam.name ILIKE $1
                OR resfam.description ILIKE $1
            )
            GROUP BY region_id HAVING COUNT(*) >= $2
                "#,
                expr.value,
                expr.count,
            )
            .fetch_all(pool)
            .await?
        }
        Category::Pfam => {
            if expr.value.to_lowercase().starts_with("pfam") {
                sqlx::query_as!(
                    RegionId,
                    r#"
            SELECT region_id FROM antismash.regions
            JOIN antismash.cdss USING (region_id)
            JOIN antismash.pfam_domains USING (cds_id)
            WHERE pfam_id ILIKE $1
            GROUP BY region_id HAVING COUNT(*) >= $2
                    "#,
                    expr.value,
                    expr.count,
                )
                .fetch_all(pool)
                .await?
            } else {
                sqlx::query_as!(
                    RegionId,
                    r#"
            SELECT region_id FROM antismash.regions
            JOIN antismash.cdss USING (region_id)
            JOIN antismash.pfam_domains USING (cds_id)
            JOIN antismash.pfams AS pfam USING (pfam_id)
            WHERE (
                pfam.pfam_id ILIKE $1
                OR pfam.name ILIKE $1
                OR pfam.description ILIKE $1
            )
            GROUP BY region_id HAVING COUNT(*) >= $2
                "#,
                    format!("%{}%", expr.value),
                    expr.count,
                )
                .fetch_all(pool)
                .await?
            }
        }
        Category::Tigrfam => {
            if expr.value.to_lowercase().starts_with("tigrfam") {
                sqlx::query_as!(
                    RegionId,
                    r#"
            SELECT region_id FROM antismash.regions
            JOIN antismash.cdss USING (region_id)
            JOIN antismash.tigrfam_domains USING (cds_id)
            WHERE tigrfam_id ILIKE $1
            GROUP BY region_id HAVING COUNT(*) >= $2
                    "#,
                    expr.value,
                    expr.count,
                )
                .fetch_all(pool)
                .await?
            } else {
                sqlx::query_as!(
                    RegionId,
                    r#"
            SELECT region_id FROM antismash.regions
            JOIN antismash.cdss USING (region_id)
            JOIN antismash.tigrfam_domains USING (cds_id)
            JOIN antismash.tigrfams AS tigrfam USING (tigrfam_id)
            WHERE (
                tigrfam.tigrfam_id ILIKE $1
                OR tigrfam.name ILIKE $1
                OR tigrfam.description ILIKE $1
            )
            GROUP BY region_id HAVING COUNT(*) >= $2
                "#,
                    format!("%{}%", expr.value),
                    expr.count,
                )
                .fetch_all(pool)
                .await?
            }
        }
        Category::GOTerm => {
            sqlx::query_as!(
                RegionId,
                r#"
            SELECT region_id FROM antismash.regions
            JOIN antismash.cdss USING (region_id)
            JOIN antismash.pfam_domains USING (cds_id)
            JOIN antismash.pfam_go_entries USING (pfam_domain_id)
            JOIN antismash.gene_ontologies USING (go_id)
            WHERE identifier ILIKE $1 OR description ILIKE $1
            GROUP BY region_id HAVING COUNT(*) >= $2
                "#,
                format!("%{}%", expr.value),
                expr.count,
            )
            .fetch_all(pool)
            .await?
        }
        Category::AsDomain => {
            sqlx::query_as!(
                RegionId,
                r#"
            SELECT region_id FROM antismash.regions
            JOIN antismash.cdss USING (region_id)
            JOIN antismash.as_domains USING (cds_id)
            JOIN antismash.as_domain_profiles AS profiles USING (as_domain_profile_id)
            WHERE profiles.name ILIKE $1 OR description ILIKE $1
            GROUP BY region_id HAVING COUNT(*) >= $2
                "#,
                format!("%{}%", expr.value),
                expr.count,
            )
            .fetch_all(pool)
            .await?
        }
        Category::AsDomainSubtype => {
            sqlx::query_as!(
                RegionId,
                r#"
            WITH subtype_cte AS
            (
                SELECT subtype
                FROM antismash.as_domain_subtypes
                WHERE antismash.as_domain_subtypes.subtype ILIKE $1
            )
            SELECT region_id
            FROM antismash.regions JOIN antismash.cdss USING (region_id)
            JOIN antismash.as_domains USING (cds_id)
            JOIN antismash.rel_as_domain_to_subtype USING (as_domain_id)
            JOIN subtype_cte USING (subtype)
            GROUP BY region_id HAVING COUNT(*) >= $2
                "#,
                expr.value,
                expr.count,
            )
            .fetch_all(pool)
            .await?
        }
        Category::ModuleQuery => handle_modulequery(pool, &expr.value).await?,
        Category::CrossCdsModule => {
            sqlx::query_as!(
                RegionId,
                r#"
            SELECT region_id FROM antismash.regions
            JOIN antismash.modules USING (region_id)
            WHERE multi_gene IS TRUE
                "#
            )
            .fetch_all(pool)
            .await?
        }
        Category::ContigEdge => {
            sqlx::query_as!(
                RegionId,
                r#"
            SELECT region_id FROM antismash.regions
            WHERE contig_edge IS TRUE
                "#
            )
            .fetch_all(pool)
            .await?
        }
        Category::T2pksElongation => {
            // This is a numeric search type
            let elongations: i32 = expr.value.parse()?;
            sqlx::query_as!(
                RegionId,
                r#"
            SELECT region_id FROM antismash.regions
            JOIN antismash.protoclusters USING (region_id)
            JOIN antismash.t2pks USING (protocluster_id)
            JOIN antismash.t2pks_starters USING (t2pks_id)
            JOIN antismash.t2pks_starter_elongation USING (domain_id)
            WHERE elongation = $1
            GROUP BY region_id HAVING COUNT(*) >= $2
                "#,
                elongations,
                expr.count,
            )
            .fetch_all(pool)
            .await?
        }
        Category::T2pksProductClass => {
            sqlx::query_as!(
                RegionId,
                r#"
            SELECT region_id FROM antismash.regions
            JOIN antismash.protoclusters USING (region_id)
            JOIN antismash.t2pks USING (protocluster_id)
            JOIN antismash.t2pks_product_classes USING (t2pks_id)
            WHERE product_class ILIKE $1
            GROUP BY region_id HAVING COUNT(*) >= $2
                "#,
                expr.value,
                expr.count,
            )
            .fetch_all(pool)
            .await?
        }
        Category::T2pksStarter => {
            sqlx::query_as!(
                RegionId,
                r#"
            SELECT region_id FROM antismash.regions
            JOIN antismash.protoclusters USING (region_id)
            JOIN antismash.t2pks USING (protocluster_id)
            JOIN antismash.t2pks_starters USING (t2pks_id)
            WHERE name ILIKE $1
            GROUP BY region_id HAVING COUNT(*) >= $2
                "#,
                expr.value,
                expr.count,
            )
            .fetch_all(pool)
            .await?
        }
        Category::T2pksProfile => {
            sqlx::query_as!(
                RegionId,
                r#"
            SELECT region_id FROM antismash.regions
            JOIN antismash.protoclusters USING (region_id)
            JOIN antismash.t2pks USING (protocluster_id)
            JOIN antismash.t2pks_cds_domain USING (t2pks_id)
            JOIN antismash.t2pks_profiles USING (profile_id)
            WHERE name ILIKE $1
            GROUP BY region_id HAVING COUNT(*) >= $2
                "#,
                expr.value,
                expr.count,
            )
            .fetch_all(pool)
            .await?
        }
        Category::SmCoG => {
            sqlx::query_as!(
                RegionId,
                r#"
            SELECT region_id FROM antismash.regions
            JOIN antismash.cdss USING (region_id)
            JOIN antismash.smcog_hits USING (cds_id)
            JOIN antismash.smcogs AS smcog USING (smcog_id)
            WHERE smcog.name ILIKE $1
            GROUP BY region_id HAVING COUNT(*) >= $2
                "#,
                expr.value,
                expr.count,
            )
            .fetch_all(pool)
            .await?
        }
        Category::Tfbs => {
            let mut ids: Vec<RegionId> = sqlx::query_as!(
                RegionId,
                r#"
            SELECT region_id FROM antismash.regions
            JOIN antismash.binding_sites USING (region_id)
            JOIN antismash.regulators USING (regulator_id)
            WHERE name ILIKE $1
            GROUP BY region_id HAVING COUNT(*) >= $2
                "#,
                expr.value,
                expr.count,
            )
            .fetch_all(pool)
            .await?;

            for filter in &expr.filters {
                ids = tfbs::tfbs_quality(pool, &ids, filter).await?;
            }

            ids
        }
        Category::CompoundSeq => {
            sqlx::query_as!(
                RegionId,
                r#"
            SELECT region_id FROM antismash.regions
            JOIN antismash.protoclusters USING (region_id)
            JOIN antismash.ripps USING (protocluster_id)
            WHERE peptide_sequence ILIKE $1
            GROUP BY region_id HAVING COUNT(*) >= $2
                "#,
                format!("%{}%", expr.value),
                expr.count,
            )
            .fetch_all(pool)
            .await?
        }
        Category::CompoundClass => {
            sqlx::query_as!(
                RegionId,
                r#"
            SELECT region_id FROM antismash.regions
            JOIN antismash.protoclusters USING (region_id)
            JOIN antismash.ripps USING (protocluster_id)
            WHERE subclass ILIKE $1
                "#,
                expr.value,
            )
            .fetch_all(pool)
            .await?
        }
        Category::Strain => {
            sqlx::query_as!(
                RegionId,
                r#"
            SELECT region_id FROM antismash.regions
            JOIN antismash.dna_sequences USING (accession)
            JOIN antismash.genomes USING (genome_id)
            JOIN antismash.taxa USING (tax_id)
            WHERE strain ILIKE $1
                "#,
                expr.value,
            )
            .fetch_all(pool)
            .await?
        }
        Category::Species => {
            sqlx::query_as!(
                RegionId,
                r#"
            SELECT region_id FROM antismash.regions
            JOIN antismash.dna_sequences USING (accession)
            JOIN antismash.genomes USING (genome_id)
            JOIN antismash.taxa USING (tax_id)
            WHERE species ILIKE $1
                "#,
                expr.value,
            )
            .fetch_all(pool)
            .await?
        }
        Category::Genus => {
            sqlx::query_as!(
                RegionId,
                r#"
            SELECT region_id FROM antismash.regions
            JOIN antismash.dna_sequences USING (accession)
            JOIN antismash.genomes USING (genome_id)
            JOIN antismash.taxa USING (tax_id)
            WHERE genus ILIKE $1
                "#,
                expr.value,
            )
            .fetch_all(pool)
            .await?
        }
        Category::Family => {
            sqlx::query_as!(
                RegionId,
                r#"
            SELECT region_id FROM antismash.regions
            JOIN antismash.dna_sequences USING (accession)
            JOIN antismash.genomes USING (genome_id)
            JOIN antismash.taxa USING (tax_id)
            WHERE family ILIKE $1
                "#,
                expr.value,
            )
            .fetch_all(pool)
            .await?
        }
        Category::Order => {
            sqlx::query_as!(
                RegionId,
                r#"
            SELECT region_id FROM antismash.regions
            JOIN antismash.dna_sequences USING (accession)
            JOIN antismash.genomes USING (genome_id)
            JOIN antismash.taxa USING (tax_id)
            WHERE taxonomic_order ILIKE $1
                "#,
                expr.value,
            )
            .fetch_all(pool)
            .await?
        }
        Category::Class => {
            sqlx::query_as!(
                RegionId,
                r#"
            SELECT region_id FROM antismash.regions
            JOIN antismash.dna_sequences USING (accession)
            JOIN antismash.genomes USING (genome_id)
            JOIN antismash.taxa USING (tax_id)
            WHERE class ILIKE $1
                "#,
                expr.value,
            )
            .fetch_all(pool)
            .await?
        }
        Category::Phylum => {
            sqlx::query_as!(
                RegionId,
                r#"
            SELECT region_id FROM antismash.regions
            JOIN antismash.dna_sequences USING (accession)
            JOIN antismash.genomes USING (genome_id)
            JOIN antismash.taxa USING (tax_id)
            WHERE phylum ILIKE $1
                "#,
                expr.value,
            )
            .fetch_all(pool)
            .await?
        }
        Category::Superkingdom => {
            sqlx::query_as!(
                RegionId,
                r#"
            SELECT region_id FROM antismash.regions
            JOIN antismash.dna_sequences USING (accession)
            JOIN antismash.genomes USING (genome_id)
            JOIN antismash.taxa USING (tax_id)
            WHERE superkingdom ILIKE $1
                "#,
                expr.value,
            )
            .fetch_all(pool)
            .await?
        }
        Category::CompaRiPPsonMibig => {
            sqlx::query_as!(
                RegionId,
                r#"
            SELECT region_id FROM antismash.regions
            JOIN antismash.comparippson_hits USING (region_id)
            JOIN antismash.comparippson_mibig_references AS mibig USING (comparippson_mibig_id)
            WHERE (
                   mibig.accession ILIKE $1
                OR compound ILIKE $1
                OR product ILIKE $1
            )
            GROUP BY region_id HAVING COUNT(*) >= $2
                "#,
                format!("%{}%", expr.value),
                expr.count
            )
            .fetch_all(pool)
            .await?
        }
        Category::ClusterCompareRegion => {
            sqlx::query_as!(
                RegionId,
                r#"
            SELECT r.region_id FROM antismash.regions AS r
            JOIN antismash.protoclusters USING (region_id)
            JOIN antismash.cluster_compare_hits USING (protocluster_id)
            WHERE reference_accession ILIKE $1 AND protocluster_id != NULL
                "#,
                expr.value,
            )
            .fetch_all(pool)
            .await?
        }
        Category::ClusterCompareProtocluster => {
            sqlx::query_as!(
                RegionId,
                r#"
            SELECT r.region_id FROM antismash.regions AS r
            JOIN antismash.cluster_compare_hits USING (region_id)
            WHERE reference_accession ILIKE $1 AND region_id != NULL
            GROUP BY region_id HAVING COUNT(*) >= $2
                "#,
                expr.value,
                expr.count,
            )
            .fetch_all(pool)
            .await?
        }
        Category::ClusterBlast => {
            handle_clusterblast(pool, &expr.value, ClusterBlastAlgorithm::ClusterBlast).await?
        }
        Category::KnownCluster => {
            handle_clusterblast(pool, &expr.value, ClusterBlastAlgorithm::KnownClusterBlast).await?
        }
        Category::SubCluster => {
            handle_clusterblast(pool, &expr.value, ClusterBlastAlgorithm::SubClusterBlast).await?
        }
    };
    let results: Vec<i32> = region_ids.into_iter().map(|r| r.region_id).collect();
    Ok(results)
}

#[derive(Debug, PartialEq, Eq, strum::AsRefStr)]
#[strum(serialize_all = "lowercase")]
enum ClusterBlastAlgorithm {
    ClusterBlast,
    KnownClusterBlast,
    SubClusterBlast,
}

async fn handle_clusterblast(
    pool: &PgPool,
    term: &str,
    algorithm: ClusterBlastAlgorithm,
) -> Result<Vec<RegionId>> {
    Ok(sqlx::query_as!(
        RegionId,
        r#"
    SELECT r.region_id FROM antismash.regions AS r
    JOIN antismash.clusterblast_hits USING (region_id)
    JOIN antismash.clusterblast_algorithms USING (algorithm_id)
    WHERE acc ILIKE $1 AND name = $2
        "#,
        term,
        algorithm.as_ref(),
    )
    .fetch_all(pool)
    .await?)
}

async fn handle_modulequery(_pool: &PgPool, _term: &str) -> Result<Vec<RegionId>> {
    Err(Error::NotImplementedError(
        "module query not implemented yet".to_string(),
    ))
}

// License: GNU Affero General Public License v3 or later
// A copy of GNU AGPL v3 should have been included in this software package in LICENSE.txt.

use std::str::FromStr;

use nom::IResult;
use serde::{Deserialize, Serialize};
use serde_json;
use strum::EnumMessage;

use super::filters::{get_filters_by_category, AvailableFilter};
use crate::Error;

pub trait CategoryMetadata {
    fn get_group(&self) -> Option<CategoryGroup>;
    fn get_description(&self) -> Option<&'static str>;
    fn get_type(&self) -> Option<CategoryType>;
}

#[derive(Debug, Deserialize, Serialize, PartialEq, strum::EnumString)]
#[serde(rename_all = "lowercase")]
pub enum CategoryType {
    Text,
    Bool,
    Numeric,
    ModuleQuery,
}

#[derive(
    Debug, Deserialize, Serialize, PartialEq, Eq, Hash, Clone, strum::EnumString, strum::EnumIter,
)]
pub enum CategoryGroup {
    #[serde(rename = "antiSMASH prediction")]
    AntismashPrediction,
    #[serde(rename = "Compound properties")]
    CompoundProperty,
    #[serde(rename = "Quality filters")]
    QualityFilter,
    // No need to rename this one
    Taxonomy,
    #[serde(rename = "Similar Clusters")]
    SimilarClusters,
}

// We're abusing the strum::EnumMessage message to set the CategoryGroup,
// the doc string comment to set the label,
// and the detailed message for the description
#[derive(
    Debug,
    Deserialize,
    Serialize,
    PartialEq,
    Clone,
    strum::EnumIter,
    strum::EnumMessage,
    strum::Display,
    strum::IntoStaticStr,
)]
#[serde(rename_all = "lowercase")]
#[strum(serialize_all = "lowercase")]
pub enum Category {
    // uncategorised
    /// NCBI RefSeq Accession
    #[strum(detailed_message = "DNA record accession from RefSeq")]
    Acc,

    /// NCBI Assembly ID
    #[strum(detailed_message = "NCBI assembly ID")]
    Assembly,

    /// BGC type
    #[strum(
        message = "AntismashPrediction",
        detailed_message = "BGC type as predicted by antiSMASH"
    )]
    Type,

    /// BGC category
    #[strum(
        message = "AntismashPrediction",
        detailed_message = "BGC type category (e.g. PKS, Terpene)"
    )]
    TypeCategory,

    /// Candidate cluster type
    #[strum(
        message = "AntismashPrediction",
        detailed_message = "A specific kind of CandidateCluster"
    )]
    CandidateKind,

    /// Substrate
    #[strum(
        message = "AntismashPrediction",
        detailed_message = "Substrate integrated into the cluster product"
    )]
    Substrate,

    /// Monomer
    #[strum(
        message = "AntismashPrediction",
        detailed_message = "Monomer contained in the cluster product"
    )]
    Monomer,

    /// Biosynthetic profile
    #[strum(
        message = "AntismashPrediction",
        detailed_message = "Regions containing a specific antiSMASH BGC detection profile hit"
    )]
    Profile,

    /// ResFam profile
    #[strum(
        message = "AntismashPrediction",
        detailed_message = "Regions containing a hit to the given ResFams ID"
    )]
    Resfam,

    /// Pfam profile
    #[strum(
        message = "AntismashPrediction",
        detailed_message = "Regions containing a hit to the given PFAM ID"
    )]
    Pfam,

    /// TIGRFAM profile
    #[strum(
        message = "AntismashPrediction",
        detailed_message = "Regions containing a hit to the given TIGRFam ID"
    )]
    Tigrfam,

    /// GO term
    #[strum(
        message = "AntismashPrediction",
        detailed_message = "Regions containing a hit to the given GO term (based on PFAM hits)"
    )]
    GOTerm,

    /// NRPS/PKS domain
    #[strum(
        message = "AntismashPrediction",
        detailed_message = "Regions containing a specific aSDomain by name"
    )]
    AsDomain,

    /// NRPS/PKS domain subtype
    #[strum(
        message = "AntismashPrediction",
        detailed_message = "Regions containig a specific aSDomain subtype"
    )]
    AsDomainSubtype,

    /// NRPS/PKS module query
    #[strum(
        message = "AntismashPrediction",
        detailed_message = "Regions containing a module with the requested component domains"
    )]
    ModuleQuery,

    /// NRPS/PKS cross-CDS module
    #[strum(
        message = "AntismashPrediction",
        detailed_message = "Regions containing a cross-CDS module"
    )]
    CrossCdsModule,

    /// PKS type II profile
    #[strum(
        message = "AntismashPrediction",
        detailed_message = "Regions with a specific PKS type II detection profile"
    )]
    T2pksProfile,

    /// PKS type II product class
    #[strum(
        message = "AntismashPrediction",
        detailed_message = "Regions with a specific PKS type II product class"
    )]
    T2pksProductClass,

    /// PKS type II starter moiety
    #[strum(
        message = "AntismashPrediction",
        detailed_message = "Regions with a specific PKS type II starter"
    )]
    T2pksStarter,

    /// PKS type II elongation
    #[strum(
        message = "AntismashPrediction",
        detailed_message = "Regions with PKS type II elongations of a specific size"
    )]
    T2pksElongation,

    /// smCoG hit
    #[strum(
        message = "AntismashPrediction",
        detailed_message = "Regions containing a specific smCoG hit"
    )]
    SmCoG,

    /// Binding site regulator
    #[strum(
        message = "AntismashPrediction",
        detailed_message = "Regions containing a TFBS regulator of the given name"
    )]
    Tfbs,

    /// Compound sequence
    #[strum(
        message = "CompoundProperty",
        detailed_message = "RiPP BGC containing a compound with a sequence containing this string"
    )]
    CompoundSeq,

    /// RiPP compound class
    #[strum(
        message = "CompoundProperty",
        detailed_message = "RiPP BGC containing a given compound class"
    )]
    CompoundClass,

    /// Region on contig edge
    #[strum(
        message = "QualityFilter",
        detailed_message = "Regions on a contig edge"
    )]
    ContigEdge,

    #[strum(
        message = "Taxonomy",
        detailed_message = "By strain according to NCBI taxonomy"
    )]
    Strain,

    #[strum(
        message = "Taxonomy",
        detailed_message = "By species according to NCBI taxonomy"
    )]
    Species,

    #[strum(
        message = "Taxonomy",
        detailed_message = "By genus according to NCBI taxonomy"
    )]
    Genus,

    #[strum(
        message = "Taxonomy",
        detailed_message = "By family according to NCBI taxonomy"
    )]
    Family,

    #[strum(
        message = "Taxonomy",
        detailed_message = "By order according to NCBI taxonomy"
    )]
    Order,

    #[strum(
        message = "Taxonomy",
        detailed_message = "By class according to NCBI taxonomy"
    )]
    Class,

    #[strum(
        message = "Taxonomy",
        detailed_message = "By phylum according to NCBI taxonomy"
    )]
    Phylum,

    #[strum(
        message = "Taxonomy",
        detailed_message = "By superkingdom according to NCBI taxonomy"
    )]
    Superkingdom,

    /// CompaRiPPson MIBiG hit
    #[strum(
        message = "SimilarClusters",
        detailed_message = "Regions containing a CompaRiPPson hit against the given MIBiG ID"
    )]
    CompaRiPPsonMibig,

    /// ClusterCompare by region
    #[strum(
        message = "SimilarClusters",
        detailed_message = "Regions with ClusterCompare hits matching the given MIBiG ID"
    )]
    ClusterCompareRegion,

    /// ClusterCompare by protocluster
    #[strum(
        message = "SimilarClusters",
        detailed_message = "Regions with protoclusters with ClusterCompare hits matching the given MIBiG ID"
    )]
    ClusterCompareProtocluster,

    /// ClusterBlast hit
    #[strum(
        message = "SimilarClusters",
        detailed_message = "Regions containing a hit to the given ClusterBlast entry"
    )]
    ClusterBlast,

    /// KnownClusterBlast hit
    #[strum(
        message = "SimilarClusters",
        detailed_message = "Regions containing a hit to the given KnownClusterBlast entry"
    )]
    KnownCluster,

    /// SubClusterBlast hit
    #[strum(
        message = "SimilarClusters",
        detailed_message = "Regions containing a hit to the given SubClusterBlast entry"
    )]
    SubCluster,
}

impl Category {
    pub fn parse(input: &str) -> IResult<&str, Self, Error> {
        let ret = match input.parse::<Self>() {
            Ok(cat) => cat,
            Err(e) => return Err(nom::Err::Failure(e)),
        };
        Ok(("", ret))
    }

    pub fn get_group(&self) -> Option<CategoryGroup> {
        if let Some(msg) = self.get_message() {
            CategoryGroup::from_str(msg).ok()
        } else {
            None
        }
    }

    pub fn get_label(&self) -> &'static str {
        if let Some(label) = self.get_documentation() {
            label
        } else {
            self.into()
        }
    }

    pub fn get_type(&self) -> CategoryType {
        match self {
            Category::ModuleQuery => CategoryType::ModuleQuery,
            Category::ContigEdge | Category::CrossCdsModule => CategoryType::Bool,
            Category::T2pksElongation => CategoryType::Numeric,
            _ => CategoryType::Text,
        }
    }

    pub fn get_description(&self) -> &'static str {
        if let Some(desc) = self.get_detailed_message() {
            desc
        } else {
            "No description"
        }
    }

    pub fn is_countable(&self) -> bool {
        match self {
            Category::Strain
            | Category::Species
            | Category::Genus
            | Category::Family
            | Category::Order
            | Category::Class
            | Category::Phylum
            | Category::Superkingdom
            | Category::Acc
            | Category::Assembly
            | Category::CompoundClass
            | Category::ClusterCompareRegion
            | Category::ContigEdge
            | Category::ClusterBlast
            | Category::KnownCluster
            | Category::SubCluster
            | Category::CompaRiPPsonMibig => false,
            _ => true,
        }
    }

    pub fn get_filters(&self) -> Vec<AvailableFilter> {
        get_filters_by_category(self)
    }
}

impl FromStr for Category {
    type Err = Error;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        // serde expects the value to be quoted
        let quoted = format!("\"{s}\"");
        Ok(serde_json::from_str::<Self>(&quoted)?)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse() {
        let (_, cat) = Category::parse("acc").unwrap();
        assert_eq!(cat, Category::Acc);
    }

    #[test]
    fn test_from_str() {
        let c: Category = "acc".parse().unwrap();
        assert_eq!(c, Category::Acc);
    }

    #[test]
    fn test_group() {
        let tests = [
            (Category::Acc, None),
            (Category::Type, Some(CategoryGroup::AntismashPrediction)),
            (
                Category::CompoundClass,
                Some(CategoryGroup::CompoundProperty),
            ),
            (Category::Species, Some(CategoryGroup::Taxonomy)),
        ];
        for (cat, expected) in tests {
            assert_eq!(cat.get_group(), expected);
        }
    }

    #[test]
    fn test_label() {
        let tests = [
            (Category::Acc, "NCBI RefSeq Accession"),
            (Category::Genus, "genus"),
        ];
        for (cat, expected) in tests {
            assert_eq!(cat.get_label(), expected);
        }
    }

    #[test]
    fn test_type() {
        let tests = [
            (Category::Acc, CategoryType::Text),
            (Category::ModuleQuery, CategoryType::ModuleQuery),
            (Category::CrossCdsModule, CategoryType::Bool),
        ];
        for (cat, expected) in tests {
            assert_eq!(cat.get_type(), expected);
        }
    }

    #[test]
    fn test_description() {
        let tests = [
            (Category::Acc, "DNA record accession from RefSeq"),
            (Category::Genus, "By genus according to NCBI taxonomy"),
        ];
        for (cat, expected) in tests {
            assert_eq!(cat.get_description(), expected);
        }
    }

    #[test]
    fn test_countable() {
        let tests = [(Category::Acc, false), (Category::ModuleQuery, true)];
        for (cat, expected) in tests {
            assert_eq!(cat.is_countable(), expected);
        }
    }
}

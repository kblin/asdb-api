// License: GNU Affero General Public License v3 or later
// A copy of GNU AGPL v3 should have been included in this software package in LICENSE.txt.

use axum::{extract::Query, routing::get, Extension, Json, Router};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sqlx::PgPool;

use crate::{Error, Result};

pub fn routes() -> Router {
    Router::new()
        .route("/api/tree/taxa", get(tax_tree))
        .route("/api/v1.0/tree/taxa", get(tax_tree))
}

#[derive(Debug, Deserialize)]
struct TaxTreeQuery {
    id: String,
}

#[derive(Debug, Serialize)]
struct TreeNode {
    id: String,
    parent: String,
    text: String,
    state: Option<NodeState>,
    #[serde(rename = "type")]
    node_type: Option<String>,
    assembly_id: Option<String>,
    li_attr: Option<LiAttr>,
    children: bool,
}

impl TreeNode {
    fn new(id: String, parent: String, text: String) -> Self {
        TreeNode {
            id: id,
            parent: parent,
            text: text,
            state: Some(NodeState { disabled: true }),
            node_type: None,
            assembly_id: None,
            li_attr: None,
            children: true,
        }
    }

    fn set_leaf(&mut self, assembly_id: String) -> Self {
        TreeNode {
            id: self.id.clone(),
            parent: self.parent.clone(),
            text: self.text.clone(),
            state: None,
            node_type: Some("strain".to_string()),
            assembly_id: Some(assembly_id.clone()),
            li_attr: Some(LiAttr {
                data_assembly: assembly_id,
            }),
            children: false,
        }
    }
}

#[derive(Debug, Serialize)]
struct NodeState {
    disabled: bool,
}

#[derive(Debug, Serialize)]
struct LiAttr {
    #[serde(rename = "data-assembly")]
    data_assembly: String,
}

async fn tax_tree(
    Extension(pool): Extension<PgPool>,
    Query(params): Query<TaxTreeQuery>,
) -> Result<Json<Value>> {
    let id = params.id;
    let body = Json(json!(get_taxon_tree_nodes(pool, id).await?));
    Ok(body)
}

async fn get_taxon_tree_nodes(pool: PgPool, tree_id: String) -> Result<Vec<TreeNode>> {
    let mut nodes: Vec<TreeNode> = Vec::new();
    if tree_id == "1" {
        nodes.extend(get_superkingdom(pool).await?)
    } else {
        let params: Vec<&str> = tree_id.split("_").collect();
        if params.len() < 1 {
            return Err(Error::InvalidRequest("Invalid tree id".to_string()));
        }
        let tax_level = params[0];
        match tax_level {
            "superkingdom" => nodes.extend(get_phylum(pool, &params[1..]).await?),
            "phylum" => nodes.extend(get_class(pool, &params[1..]).await?),
            "class" => nodes.extend(get_order(pool, &params[1..]).await?),
            "order" => nodes.extend(get_family(pool, &params[1..]).await?),
            "family" => nodes.extend(get_genus(pool, &params[1..]).await?),
            "genus" => nodes.extend(get_species(pool, &params[1..]).await?),
            "species" => nodes.extend(get_strain(pool, &params[1..]).await?),
            _ => {
                return Err(Error::InvalidRequest(format!(
                    "Invalid tax_level {tax_level}"
                )))
            }
        }
    }

    Ok(nodes)
}

async fn get_superkingdom(pool: PgPool) -> Result<Vec<TreeNode>> {
    let nodes = sqlx::query!(
        r#"
        SELECT superkingdom, COUNT(assembly_id)
        FROM antismash.taxa
        JOIN antismash.genomes USING (tax_id)
        GROUP BY superkingdom
        ORDER BY superkingdom;"#
    )
    .fetch_all(&pool)
    .await?
    .iter()
    .map(|node| {
        TreeNode::new(
            format!(
                "superkingdom_{}",
                node.superkingdom.clone().unwrap_or_default().to_lowercase()
            ),
            "#".to_string(),
            format!(
                "{} ({})",
                node.superkingdom.clone().unwrap_or_default(),
                node.count.unwrap_or_default()
            ),
        )
    })
    .collect();

    Ok(nodes)
}

async fn get_phylum(pool: PgPool, params: &[&str]) -> Result<Vec<TreeNode>> {
    if params.len() < 1 {
        return Err(Error::InvalidRequest(
            "Not enough taxon parameters".to_string(),
        ));
    }
    let nodes = sqlx::query!(
        r#"
        SELECT phylum, COUNT(assembly_id)
        FROM antismash.taxa
        JOIN antismash.genomes USING (tax_id)
        WHERE superkingdom ILIKE $1
        GROUP BY phylum
        ORDER BY phylum;"#,
        params[0]
    )
    .fetch_all(&pool)
    .await?
    .iter()
    .map(|node| {
        TreeNode::new(
            format!(
                "phylum_{}_{}",
                params.join("_"),
                node.phylum.clone().unwrap_or_default().to_lowercase()
            ),
            format!("superkingdom_{}", params.join("_")),
            format!(
                "{} ({})",
                node.phylum.clone().unwrap_or_default(),
                node.count.unwrap_or_default()
            ),
        )
    })
    .collect();

    Ok(nodes)
}

async fn get_class(pool: PgPool, params: &[&str]) -> Result<Vec<TreeNode>> {
    eprintln!("{params:?}");
    if params.len() < 2 {
        return Err(Error::InvalidRequest(
            "Not enough taxon parameters".to_string(),
        ));
    }
    let nodes = sqlx::query!(
        r#"
        SELECT class, COUNT(assembly_id)
        FROM antismash.taxa
        JOIN antismash.genomes USING (tax_id)
        WHERE superkingdom ILIKE $1
        AND phylum ILIKE $2
        GROUP BY class
        ORDER BY class;"#,
        params[0],
        params[1],
    )
    .fetch_all(&pool)
    .await?
    .iter()
    .map(|node| {
        TreeNode::new(
            format!(
                "class_{}_{}",
                params.join("_"),
                node.class.clone().unwrap_or_default().to_lowercase()
            ),
            format!("phylum_{}", params.join("_")),
            format!(
                "{} ({})",
                node.class.clone().unwrap_or_default(),
                node.count.unwrap_or_default()
            ),
        )
    })
    .collect();

    Ok(nodes)
}

async fn get_order(pool: PgPool, params: &[&str]) -> Result<Vec<TreeNode>> {
    eprintln!("{params:?}");
    if params.len() < 3 {
        return Err(Error::InvalidRequest(
            "Not enough taxon parameters".to_string(),
        ));
    }
    let nodes = sqlx::query!(
        r#"
        SELECT taxonomic_order, COUNT(assembly_id)
        FROM antismash.taxa
        JOIN antismash.genomes USING (tax_id)
        WHERE superkingdom ILIKE $1
        AND phylum ILIKE $2
        AND class ILIKE $3
        GROUP BY taxonomic_order
        ORDER BY taxonomic_order;"#,
        params[0],
        params[1],
        params[2],
    )
    .fetch_all(&pool)
    .await?
    .iter()
    .map(|node| {
        TreeNode::new(
            format!(
                "order_{}_{}",
                params.join("_"),
                node.taxonomic_order
                    .clone()
                    .unwrap_or_default()
                    .to_lowercase()
            ),
            format!("class_{}", params.join("_")),
            format!(
                "{} ({})",
                node.taxonomic_order.clone().unwrap_or_default(),
                node.count.unwrap_or_default()
            ),
        )
    })
    .collect();

    Ok(nodes)
}

async fn get_family(pool: PgPool, params: &[&str]) -> Result<Vec<TreeNode>> {
    eprintln!("{params:?}");
    if params.len() < 4 {
        return Err(Error::InvalidRequest(
            "Not enough taxon parameters".to_string(),
        ));
    }
    let nodes = sqlx::query!(
        r#"
        SELECT family, COUNT(assembly_id)
        FROM antismash.taxa
        JOIN antismash.genomes USING (tax_id)
        WHERE superkingdom ILIKE $1
        AND phylum ILIKE $2
        AND class ILIKE $3
        AND taxonomic_order ILIKE $4
        GROUP BY family
        ORDER BY family;"#,
        params[0],
        params[1],
        params[2],
        params[3],
    )
    .fetch_all(&pool)
    .await?
    .iter()
    .map(|node| {
        TreeNode::new(
            format!(
                "family_{}_{}",
                params.join("_"),
                node.family.clone().unwrap_or_default().to_lowercase()
            ),
            format!("order_{}", params.join("_")),
            format!(
                "{} ({})",
                node.family.clone().unwrap_or_default(),
                node.count.unwrap_or_default()
            ),
        )
    })
    .collect();

    Ok(nodes)
}

async fn get_genus(pool: PgPool, params: &[&str]) -> Result<Vec<TreeNode>> {
    eprintln!("{params:?}");
    if params.len() < 5 {
        return Err(Error::InvalidRequest(
            "Not enough taxon parameters".to_string(),
        ));
    }
    let nodes = sqlx::query!(
        r#"
        SELECT genus, COUNT(assembly_id)
        FROM antismash.taxa
        JOIN antismash.genomes USING (tax_id)
        WHERE superkingdom ILIKE $1
        AND phylum ILIKE $2
        AND class ILIKE $3
        AND taxonomic_order ILIKE $4
        AND family ILIKE $5
        GROUP BY genus
        ORDER BY genus;"#,
        params[0],
        params[1],
        params[2],
        params[3],
        params[4],
    )
    .fetch_all(&pool)
    .await?
    .iter()
    .map(|node| {
        TreeNode::new(
            format!(
                "genus_{}_{}",
                params.join("_"),
                node.genus.clone().unwrap_or_default().to_lowercase()
            ),
            format!("family_{}", params.join("_")),
            format!(
                "{} ({})",
                node.genus.clone().unwrap_or_default(),
                node.count.unwrap_or_default()
            ),
        )
    })
    .collect();

    Ok(nodes)
}

async fn get_species(pool: PgPool, params: &[&str]) -> Result<Vec<TreeNode>> {
    eprintln!("{params:?}");
    if params.len() < 6 {
        return Err(Error::InvalidRequest(
            "Not enough taxon parameters".to_string(),
        ));
    }
    let nodes = sqlx::query!(
        r#"
        SELECT species, COUNT(assembly_id)
        FROM antismash.taxa
        JOIN antismash.genomes USING (tax_id)
        WHERE superkingdom ILIKE $1
        AND phylum ILIKE $2
        AND class ILIKE $3
        AND taxonomic_order ILIKE $4
        AND family ILIKE $5
        AND genus ILIKE $6
        GROUP BY species
        ORDER BY species;"#,
        params[0],
        params[1],
        params[2],
        params[3],
        params[4],
        params[5],
    )
    .fetch_all(&pool)
    .await?
    .iter()
    .map(|node| {
        TreeNode::new(
            format!(
                "species_{}_{}",
                params.join("_"),
                node.species.clone().unwrap_or_default().to_lowercase()
            ),
            format!("genus_{}", params.join("_")),
            format!(
                "{} ({})",
                node.species.clone().unwrap_or_default(),
                node.count.unwrap_or_default()
            ),
        )
    })
    .collect();

    Ok(nodes)
}

async fn get_strain(pool: PgPool, params: &[&str]) -> Result<Vec<TreeNode>> {
    if params.len() < 7 {
        return Err(Error::InvalidRequest(
            "Not enough taxon parameters".to_string(),
        ));
    }
    let nodes = sqlx::query!(
        r#"
        SELECT genus, species, strain, assembly_id
        FROM antismash.taxa
        JOIN antismash.genomes USING (tax_id)
        WHERE superkingdom ILIKE $1
        AND phylum ILIKE $2
        AND class ILIKE $3
        AND taxonomic_order ILIKE $4
        AND family ILIKE $5
        AND genus ILIKE $6
        AND species ILIKE $7
        ORDER BY strain;"#,
        params[0],
        params[1],
        params[2],
        params[3],
        params[4],
        params[5],
        params[6],
    )
    .fetch_all(&pool)
    .await?
    .iter()
    .map(|node| {
        let parent = format!("species_{}", params.join("_"));
        let text = format!(
            "{} {} {} {}",
            node.genus.clone().unwrap_or_default(),
            node.species.clone().unwrap_or_default(),
            node.strain.clone().unwrap_or_default(),
            node.assembly_id.clone()
        );
        TreeNode::new(node.assembly_id.to_owned(), parent, text)
            .set_leaf(node.assembly_id.to_owned())
    })
    .collect();

    Ok(nodes)
}

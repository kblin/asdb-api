// License: GNU Affero General Public License v3 or later
// A copy of GNU AGPL v3 should have been included in this software package in LICENSE.txt.

use std::io::{Cursor, Write};
use std::path::PathBuf;

use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use tokio::fs;
use tokio::io::{self, AsyncReadExt};
use zip::{write::FileOptions, CompressionMethod, ZipWriter};

use crate::api::cds;
use crate::api::domains;
use crate::api::region;
use crate::query::{ReturnType, SearchType};
use crate::{Error, Result};

use super::RunConfig;

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct StoredQueryInput {
    pub job_id: String,
    pub ids: Vec<i32>,
    pub search_type: SearchType,
    pub return_type: ReturnType,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct StoredQuery {
    pub input: StoredQueryInput,
    pub filename: Option<String>,
}

impl StoredQuery {
    pub fn new(
        job_id: String,
        ids: &[i32],
        search_type: SearchType,
        return_type: ReturnType,
    ) -> Self {
        Self {
            input: StoredQueryInput {
                job_id,
                ids: Vec::from(ids),
                search_type,
                return_type,
            },
            filename: None,
        }
    }
}

pub async fn run(mut query: StoredQuery, pool: &PgPool, config: &RunConfig) -> Result<StoredQuery> {
    let job_id = query.input.job_id.as_str();
    let jobdir = config.jobdir.join(job_id);
    let urlroot = &config.urlroot;
    fs::create_dir_all(&jobdir).await?;

    let (filename, data) = match query.input.search_type {
        SearchType::Region => run_region(&query, pool, config).await?,
        SearchType::Gene => run_cds(&query, pool).await?,
        SearchType::Domain => run_domain(&query, pool).await?,
    };

    fs::write(jobdir.join(&filename), &data).await?;

    query.filename = Some(format!("/{urlroot}/{job_id}/{filename}"));
    Ok(query)
}

async fn run_region(
    query: &StoredQuery,
    pool: &PgPool,
    config: &RunConfig,
) -> Result<(String, Vec<u8>)> {
    let filename: String;
    let data = match query.input.return_type {
        ReturnType::Json => {
            filename = format!("{}.json", &query.input.job_id);
            let regions = region::ids_to_regions(pool, &query.input.ids).await?;
            serde_json::to_vec(&regions)?
        }
        ReturnType::Csv => {
            filename = format!("{}.csv", &query.input.job_id);
            let regions = region::ids_to_regions(pool, &query.input.ids)
                .await?
                .into_iter()
                .map(|r| r.to_csv())
                .collect::<Vec<String>>()
                .join("\n");
            Vec::from(format!("{}\n{regions}", region::Region::csv_header()))
        }
        ReturnType::Fasta => {
            filename = format!("{}.fa", &query.input.job_id);
            let sequences = region::ids_to_fasta(pool, &query.input.ids)
                .await?
                .join("\n");
            Vec::from(sequences)
        }
        ReturnType::Fastaa => {
            return Err(Error::InvalidRequest(
                "Cannot request region in protein fasta format".to_string(),
            ))
        }
        ReturnType::Genbank => {
            let Some(outdir) = &config.outdir else {
                return Err(Error::InvalidRequest(
                    "Genbank format requested, but no output directory specified".to_string(),
                ));
            };

            filename = format!("{}.zip", &query.input.job_id);
            let regions = region::ids_to_regions(pool, &query.input.ids).await?;
            let mut gbk_files: Vec<PathBuf> = Vec::with_capacity(regions.len());
            for region in &regions {
                let Some(assembly_id) = &region.assembly_id else {
                    continue;
                };
                let Some(accession) = &region.accession else {
                    continue;
                };
                let Some(version) = &region.version else {
                    continue;
                };
                let number = region.region_number;
                let mut file_path = outdir.to_owned();
                file_path.push(assembly_id);
                file_path.push(format!("{accession}.{version}.region{number:03}.gbk",));
                gbk_files.push(file_path);
            }

            zip_files(&gbk_files).await?
        }
    };
    Ok((filename, data))
}

async fn zip_files(gbk_files: &[PathBuf]) -> Result<Vec<u8>> {
    let mut buffer = Cursor::new(Vec::new());
    {
        let mut zip = ZipWriter::new(&mut buffer);
        let options = FileOptions::default().compression_method(CompressionMethod::Deflated);

        for file_path in gbk_files {
            let name = get_filename(file_path)?;
            let Ok(file) = fs::File::open(file_path).await else {
                eprintln!("->> Failed to find file {name}");
                continue;
            };
            zip.start_file(name, options)?;

            let mut buf = Vec::new();
            io::copy(&mut file.take(u64::MAX), &mut buf).await?;
            zip.write_all(&buf)?;
        }

        zip.finish()?;
    }
    Ok(buffer.into_inner())
}

fn get_filename(path: &PathBuf) -> Result<&str> {
    let Some(os_name) = path.file_name() else {
        return Err(Error::NotFound);
    };

    os_name
        .to_str()
        .ok_or(Error::OsStringError(os_name.to_owned()))
}

async fn run_cds(query: &StoredQuery, pool: &PgPool) -> Result<(String, Vec<u8>)> {
    let filename: String;
    let data = match query.input.return_type {
        ReturnType::Json => {
            filename = format!("{}.json", &query.input.job_id);
            let cdses = cds::ids_to_genes(pool, &query.input.ids).await?;
            serde_json::to_vec(&cdses)?
        }
        ReturnType::Csv => {
            filename = format!("{}.csv", &query.input.job_id);
            let cdses = cds::ids_to_genes(pool, &query.input.ids)
                .await?
                .into_iter()
                .map(|c| c.to_csv())
                .collect::<Vec<String>>()
                .join("\n");
            Vec::from(format!("{}\n{cdses}", cds::Cds::csv_header()))
        }
        ReturnType::Fasta => {
            filename = format!("{}.fa", &query.input.job_id);
            let sequences = cds::ids_to_fna(pool, &query.input.ids).await?.join("\n");
            Vec::from(sequences)
        }
        ReturnType::Fastaa => {
            filename = format!("{}.fa", &query.input.job_id);
            let sequences = cds::ids_to_faa(pool, &query.input.ids).await?.join("\n");
            Vec::from(sequences)
        }
        ReturnType::Genbank => {
            return Err(Error::InvalidRequest(
                "Cannot request CDSes in Genbank format".to_string(),
            ))
        }
    };
    Ok((filename, data))
}

async fn run_domain(query: &StoredQuery, pool: &PgPool) -> Result<(String, Vec<u8>)> {
    let filename: String;
    let data = match query.input.return_type {
        ReturnType::Json => {
            filename = format!("{}.json", &query.input.job_id);
            let domains = domains::ids_to_domains(pool, &query.input.ids).await?;
            serde_json::to_vec(&domains)?
        }
        ReturnType::Csv => {
            filename = format!("{}.csv", &query.input.job_id);
            let domains = domains::ids_to_domains(pool, &query.input.ids)
                .await?
                .into_iter()
                .map(|c| c.to_csv())
                .collect::<Vec<String>>()
                .join("\n");
            Vec::from(domains)
        }
        ReturnType::Fasta => {
            filename = format!("{}.fa", &query.input.job_id);
            let sequences = domains::ids_to_fna(pool, &query.input.ids)
                .await?
                .join("\n");
            Vec::from(sequences)
        }
        ReturnType::Fastaa => {
            filename = format!("{}.fa", &query.input.job_id);
            let sequences = domains::ids_to_faa(pool, &query.input.ids)
                .await?
                .join("\n");
            Vec::from(sequences)
        }
        ReturnType::Genbank => {
            return Err(Error::InvalidRequest(
                "Cannot request domains in Genbank format".to_string(),
            ))
        }
    };
    Ok((filename, data))
}

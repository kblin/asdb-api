// License: GNU Affero General Public License v3 or later
// A copy of GNU AGPL v3 should have been included in this software package in LICENSE.txt.

use std::collections::HashMap;
use std::convert::TryFrom;
use std::path::PathBuf;
use std::process::Stdio;
use std::sync::Arc;

use serde::{Deserialize, Serialize};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};

use super::blast::{BlastInput, BlastResult};
use crate::{Error, Result};

pub const COMPARIPPSON_DB_BASE: &'static str = "/databases/comparippson/asdb/3.9/cores.fa";
pub const COMPARIPPSON_METADATA: &'static str = "comparippson/asdb/3.9/metadata.json";

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct CompaRiPPsonResults {
    pub hits: Vec<CompaRiPPsonResult>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct CompaRiPPson {
    pub input: BlastInput,
    pub results: CompaRiPPsonResults,
}

impl CompaRiPPson {
    pub fn new(name: String, sequence: String) -> Self {
        Self {
            input: BlastInput { name, sequence },
            results: CompaRiPPsonResults { hits: Vec::new() },
        }
    }

    pub fn from_blast(input: BlastInput) -> Self {
        Self {
            input,
            results: CompaRiPPsonResults { hits: Vec::new() },
        }
    }
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct CompaRiPPsonResult {
    pub q_acc: String,
    pub s_locus: String,
    pub s_type: String,
    pub s_acc: String,
    pub s_rec_start: u64,
    pub s_rec_end: u64,
    pub identity: f64,
    pub q_seq: String,
    pub q_start: u64,
    pub q_end: u64,
    pub q_len: u64,
    pub s_seq: String,
    pub s_start: u64,
    pub s_end: u64,
    pub s_len: u64,
}

impl CompaRiPPsonResult {
    pub fn from_blast(value: BlastResult, metadata: &Metadata) -> Result<Self> {
        let entry_id = if let Some(eid) = value.s_acc.split("|").next() {
            eid.to_string()
        } else {
            return Err(Error::ParserError);
        };
        let entry = if let Some(e) = metadata.entries.get(&entry_id) {
            e
        } else {
            return Err(Error::CompaRiPPsonError(format!(
                "failed to find entry {entry_id}"
            )));
        };
        let s_locus = entry.locus.to_owned();
        let s_type = entry.entry_type.to_owned();
        let s_acc = entry.accession.to_owned();
        let s_rec_start = (&entry.start).try_into()?;
        let s_rec_end = (&entry.end).try_into()?;

        Ok(Self {
            q_acc: value.q_acc,
            s_locus,
            s_type,
            s_acc,
            s_rec_start,
            s_rec_end,
            identity: value.identity,
            q_seq: value.q_seq,
            q_start: value.q_start,
            q_end: value.q_end,
            q_len: value.q_len,
            s_seq: value.s_seq,
            s_start: value.s_start,
            s_end: value.s_end,
            s_len: value.s_len,
        })
    }
}

pub async fn run(mut data: CompaRiPPson, config: &super::RunConfig) -> Result<CompaRiPPson> {
    // The dbdir should always convert to a str
    let dbdir = config.dbdir.to_str().unwrap();
    let dbdir_mapping = format!("{}:/databases:ro", dbdir);

    #[rustfmt::skip]
    let args = &[
        "run", "--detach=false", "--rm", "--interactive",
        "--volume", dbdir_mapping.as_str(),
        "--name", config.name.as_str(),
        "docker.io/antismash/asdb-jobs:latest",
        "blastp",
        "-num_threads", "4",
        "-db", COMPARIPPSON_DB_BASE,
        "-outfmt", "6 qacc sacc nident qseq qstart qend qlen sseq sstart send slen",
    ];

    let mut command = tokio::process::Command::new("podman");
    command.args(args);
    command.stdin(Stdio::piped());
    command.stdout(Stdio::piped());

    let mut child = command.spawn()?;
    let mut stdin = child.stdin.take().unwrap();
    stdin.write(data.input.to_fasta().as_bytes()).await?;
    drop(stdin);

    let res = child.wait_with_output().await?;

    let mut reader = BufReader::new(res.stdout.as_ref()).lines();

    while let Some(line) = reader.next_line().await? {
        let blast = BlastResult::from_str(&line)?;
        data.results.hits.push(CompaRiPPsonResult::from_blast(
            blast,
            &config.comparippson_config.metadata,
        )?);
    }

    Ok(data)
}

#[derive(Debug, Deserialize, Serialize, PartialEq, Clone)]
pub struct Metadata {
    pub description_format: String,
    pub fields: Arc<[String]>,
    pub id_format: String,
    pub name: String,
    pub url: String,
    pub version: String,
    pub entries: HashMap<String, Entry>,
}

impl Metadata {
    pub fn from_json(data: &str) -> Result<Self> {
        let metadata = serde_json::from_str(data)?;
        Ok(metadata)
    }
}

#[derive(Debug, Deserialize, Serialize, PartialEq, Clone)]
pub struct Entry {
    pub accession: String,
    pub locus: String,
    #[serde(rename = "type")]
    pub entry_type: String,
    pub start: Coordinate,
    pub end: Coordinate,
}

#[derive(Debug, Deserialize, Serialize, PartialEq, Clone)]
pub struct CompaRiPPsonConfig {
    pub metadata: Metadata,
    pub dbdir: PathBuf,
}

/// Biopython coordinates can be fuzzy locations that start with < or >
#[derive(Debug, Deserialize, Serialize, PartialEq, Clone)]
#[serde(transparent)]
pub struct Coordinate {
    value: String,
}

impl TryFrom<&Coordinate> for u64 {
    type Error = Error;

    fn try_from(value: &Coordinate) -> std::result::Result<Self, Self::Error> {
        let mut val = value.value.as_str();

        if val.starts_with("<") || val.starts_with(">") {
            val = &val[1..];
        }

        Ok(val.parse()?)
    }
}
impl TryFrom<Coordinate> for u64 {
    type Error = Error;

    fn try_from(value: Coordinate) -> std::result::Result<Self, Self::Error> {
        u64::try_from(&value)
    }
}

#[cfg(test)]
mod tests {
    use std::ops::Deref;

    use super::*;

    #[test]
    fn test_metadata_from_json() {
        let data = r#"{
            "description_format": "@type@: @locus@",
            "fields": ["accession", "type", "locus", "start", "end"],
            "id_format": "@accession@",
            "name": "antiSMASH-DB",
            "url": "https://antismash-db.secondarymetabolites.org/area.html?record=@accession@&start=@start@&end=@end@",
            "version": "3.0",
            "entries": {
             "1": {"accession": "NZ_SODQ01000009", "locus": "EDF57_RS22025", "type": "Lassopeptides", "start": "144181", "end": "144301"},
             "2": {"accession": "NZ_SODQ01000013", "locus": "EDF57_RS23870", "type": "Lassopeptides", "start": "<9544", "end": ">9667"},
             "3": {"accession": "NZ_SODQ01000013", "locus": "EDF57_RS23885", "type": "Lassopeptides", "start": "12420", "end": "12552"}
            }}"#;
        let meta = Metadata::from_json(data).unwrap();

        assert_eq!(meta.description_format, "@type@: @locus@");
        assert_eq!(meta.fields.len(), 5);
        assert_eq!(
            meta.fields.deref(),
            [
                "accession".to_string(),
                "type".to_string(),
                "locus".to_string(),
                "start".to_string(),
                "end".to_string()
            ]
        );
        assert_eq!(meta.entries.len(), 3);
        assert_eq!(
            meta.entries.get("1").unwrap().accession,
            "NZ_SODQ01000009".to_string()
        );
        assert_eq!(
            u64::try_from(&meta.entries.get("2").unwrap().start).unwrap(),
            9544
        );
        assert_eq!(
            u64::try_from(&meta.entries.get("2").unwrap().end).unwrap(),
            9667
        );
        assert_eq!(
            meta.entries.get("3").unwrap().locus,
            "EDF57_RS23885".to_string()
        );
    }
}

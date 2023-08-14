// License: GNU Affero General Public License v3 or later
// A copy of GNU AGPL v3 should have been included in this software package in LICENSE.txt.

use std::convert::TryFrom;
use std::process::Stdio;

use serde::{Deserialize, Serialize};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};

use super::blast::{BlastInput, BlastResult};
use crate::{Error, Result};

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct ClusterBlastResults {
    pub hits: Vec<ClusterBlastResult>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct ClusterBlast {
    pub input: BlastInput,
    pub results: ClusterBlastResults,
}

impl ClusterBlast {
    pub fn new(name: String, sequence: String) -> Self {
        Self {
            input: BlastInput { name, sequence },
            results: ClusterBlastResults { hits: Vec::new() },
        }
    }

    pub fn from_blast(input: BlastInput) -> Self {
        Self {
            input,
            results: ClusterBlastResults { hits: Vec::new() },
        }
    }
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct ClusterBlastResult {
    pub q_acc: String,
    pub s_locus: String,
    pub s_description: String,
    pub s_acc: String,
    pub s_rec_start: String,
    pub s_rec_end: String,
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

impl TryFrom<BlastResult> for ClusterBlastResult {
    type Error = Error;

    fn try_from(value: BlastResult) -> std::result::Result<Self, Self::Error> {
        let data: Vec<&str> = value.s_acc.split("|").collect();
        if data.len() != 7 {
            return Err(Error::ParserError);
        };

        let s_locus = data[4].to_owned();
        let s_description = data[5].replace("_", " ").to_owned();
        let s_acc = data[0].to_owned();
        let coords: Vec<&str> = data[2].splitn(2, "-").collect();
        if coords.len() != 2 {
            return Err(Error::ParserError);
        }
        let s_rec_start = coords[0].to_owned();
        let s_rec_end = coords[1].to_owned();

        Ok(Self {
            q_acc: value.q_acc,
            s_locus,
            s_description,
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

pub async fn run(mut data: ClusterBlast, config: &super::RunConfig) -> Result<ClusterBlast> {
    // The dbdir should always convert to a str
    let dbdir = config.dbdir.to_str().unwrap();
    let dbdir_mapping = format!("{}:/databases:ro", dbdir);

    #[rustfmt::skip]
    let args = &[
        "run", "--detach=false", "--rm", "--interactive",
        "--volume", dbdir_mapping.as_str(), 
        "--name", config.name.as_str(),
        "docker.io/antismash/asdb-jobs:latest",
        "diamond", "blastp",
        "--threads", "4",
        "--db", "/databases/clusterblast/proteins",
        "--compress", "0",
        "--max-target-seqs", "50",
        "--evalue", "1e-05",
        "--outfmt", "6", "qseqid", "sseqid", "nident", "qseq", "qstart", "qend", "qlen", "sseq", "sstart", "send", "slen",
        ];

    let mut command = tokio::process::Command::new("podman");
    command.args(args);
    command.stdin(Stdio::piped());
    command.stdout(Stdio::piped());
    command.stderr(Stdio::null());

    let mut child = command.spawn()?;
    let mut stdin = child.stdin.take().unwrap();
    stdin.write(data.input.to_fasta().as_bytes()).await?;
    drop(stdin);

    let res = child.wait_with_output().await?;

    let mut reader = BufReader::new(res.stdout.as_ref()).lines();

    while let Some(line) = reader.next_line().await? {
        let hit: ClusterBlastResult = BlastResult::from_str(&line)?.try_into()?;
        data.results.hits.push(hit);
    }

    Ok(data)
}

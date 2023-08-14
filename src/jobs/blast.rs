// License: GNU Affero General Public License v3 or later
// A copy of GNU AGPL v3 should have been included in this software package in LICENSE.txt.

use serde::{Deserialize, Serialize};

use crate::{Error, Result};

#[derive(Debug, Deserialize, Serialize, Clone, PartialEq, PartialOrd)]
pub struct BlastInput {
    pub name: String,
    pub sequence: String,
}

impl BlastInput {
    pub fn to_fasta(&self) -> String {
        format!(">{}\n{}", self.name, self.sequence)
    }
}

#[derive(Debug, PartialEq)]
pub struct BlastResult {
    pub q_acc: String,
    pub s_acc: String,
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

impl BlastResult {
    pub fn from_str(line: &str) -> Result<Self> {
        let parts: Vec<&str> = line.trim().split('\t').collect();
        if parts.len() != 11 {
            return Err(Error::ParserError);
        }

        let q_acc = parts[0].to_owned();
        let s_acc = parts[1].to_owned();
        let nident: u64 = parts[2].parse()?;
        let q_seq = parts[3].to_owned();
        let q_start = parts[4].parse()?;
        let q_end = parts[5].parse()?;
        let q_len = parts[6].parse()?;
        let s_seq = parts[7].to_owned();
        let s_start = parts[8].parse()?;
        let s_end = parts[9].parse()?;
        let s_len = parts[10].parse()?;

        let identity = (nident as f64 / f64::max(q_len as f64, s_len as f64)) * 100.0;

        Ok(Self {
            q_acc,
            s_acc,
            identity,
            q_seq,
            q_start,
            q_end,
            q_len,
            s_seq,
            s_start,
            s_end,
            s_len,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_from_str() {
        let line = "ABCD\tDEFG\t7\tMAGICHAT\t1\t8\t8\tMAGICCAT\t1\t8\t8";
        let expected = BlastResult {
            q_acc: "ABCD".to_owned(),
            s_acc: "DEFG".to_owned(),
            identity: 87.5,
            q_seq: "MAGICHAT".to_owned(),
            q_start: 1,
            q_end: 8,
            q_len: 8,
            s_seq: "MAGICCAT".to_owned(),
            s_start: 1,
            s_end: 8,
            s_len: 8,
        };

        let res = BlastResult::from_str(line).unwrap();
        assert_eq!(res, expected);
    }
}

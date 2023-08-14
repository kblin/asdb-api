// License: GNU Affero General Public License v3 or later
// A copy of GNU AGPL v3 should have been included in this software package in LICENSE.txt.

use serde::{Deserialize, Serialize};

use crate::{Error, Result};

#[derive(Debug, Deserialize, Serialize)]
pub struct ModuleQuery {
    pub starter: Vec<Vec<String>>,
    pub loader: Vec<Vec<String>>,
    pub modifications: Vec<Vec<String>>,
    pub transport: Vec<Vec<String>>,
    pub finalisation: Vec<Vec<String>>,
    pub other: Vec<Vec<String>>,
}

impl ModuleQuery {
    pub fn new() -> Self {
        Self {
            starter: Vec::new(),
            loader: Vec::new(),
            modifications: Vec::new(),
            transport: Vec::new(),
            finalisation: Vec::new(),
            other: Vec::new(),
        }
    }

    pub fn parse(input: &str) -> Result<ModuleQuery> {
        if input.len() == 0 {
            return Err(Error::InvalidRequest(
                "module query cannot be empty".to_string(),
            ));
        }

        if input.contains("+0") || input.contains("0+") {
            return Err(Error::InvalidRequest(
                "incompatible combination: + and 0, will always be false".to_string(),
            ));
        }

        if input.contains("+?") || input.contains("?+") {
            return Err(Error::InvalidRequest(
                "incompatible combination: + and ?, ? would be ignored".to_string(),
            ));
        }
        if input.contains("+*") || input.contains("*+") {
            return Err(Error::InvalidRequest(
                "incompatible combination: + and *, * would be ignored".to_string(),
            ));
        }
        if input.contains("+,") || input.contains(",+") {
            return Err(Error::InvalidRequest(
                "incompatible combination: + and ',', * would be ignored".to_string(),
            ));
        }
        if input.contains(">*") || input.contains("*>") {
            return Err(Error::InvalidRequest(
                "incompatible combination: * and >, * would be ignored".to_string(),
            ));
        }
        if input.contains("?,") || input.contains(",?") {
            return Err(Error::InvalidRequest(
                "incompatible combination: ? and ',', ? would be ignored".to_string(),
            ));
        }

        let mut module_query = ModuleQuery::new();

        for section in input.split("|") {
            let (label, alternatives) = parse_section(section)?;
            match label {
                "S" => module_query.starter = alternatives,
                "L" => module_query.loader = alternatives,
                "M" => module_query.modifications = alternatives,
                "T" => module_query.transport = alternatives,
                "F" => module_query.finalisation = alternatives,
                "O" => module_query.other = alternatives,
                invalid => {
                    return Err(Error::InvalidRequest(format!(
                        "Invalid section label {invalid}"
                    )))
                }
            }
        }

        Ok(module_query)
    }
}

fn parse_section(input: &str) -> Result<(&str, Vec<Vec<String>>)> {
    let Some((label, raw_term)) = input.split_once("=") else {
        return Err(Error::ParserError)
    };

    let tokens = split_tokens(raw_term)?;
    let alternatives = group_alternatives(tokens)?;

    Ok((label, alternatives))
}

fn split_tokens(input: &str) -> Result<Vec<String>> {
    let mut result = Vec::new();
    let mut domain: Vec<char> = Vec::new();
    let operators = [',', '+', '>'];
    for c in input.chars() {
        if operators.contains(&c) {
            if domain.is_empty() {
                return Err(Error::InvalidRequest(format!(
                    "bad syntax in domain chunk {input}"
                )));
            }
            let domain_string: String = domain.into_iter().collect();
            result.push(domain_string);
            result.push(c.to_string());
            domain = Vec::new();
        } else {
            domain.push(c);
        }
    }
    if domain.is_empty() {
        return Err(Error::InvalidRequest(
            "domain chunk must end with a domain, not an operator".to_owned(),
        ));
    }
    let domain_string: String = domain.into_iter().collect();
    result.push(domain_string);

    Ok(result)
}

fn group_alternatives(content: Vec<String>) -> Result<Vec<Vec<String>>> {
    let mut alternatives = Vec::new();
    if content.is_empty() {
        return Ok(alternatives);
    }
    if content.len() == 1 {
        alternatives.push(content);
        return Ok(alternatives);
    }
    if content.len() % 2 != 1 {
        return Err(Error::InvalidRequest(format!(
            "Invalid query {}",
            content.join("")
        )));
    }

    let mut chunk: Vec<String> = Vec::new();

    let mut i = 1;
    while i < content.len() {
        let operator = content[i].clone();
        if chunk.is_empty() {
            chunk.push(content[i - 1].clone());
        }
        eprintln!("{chunk:?}, {i}, {content:?}");
        match operator.as_str() {
            "+" | ">" => chunk.push(operator),
            "," => {
                alternatives.push(chunk);
                chunk = Vec::new();
            }
            _ => {
                return Err(Error::InvalidRequest(format!(
                    "Unknown operator {operator}"
                )))
            }
        };
        chunk.push(content[i + 1].clone());
        i += 2;
    }

    if !chunk.is_empty() {
        alternatives.push(chunk);
    }

    Ok(alternatives)
}

#[cfg(test)]
mod tests {
    use std::vec;

    use super::*;

    #[test]
    fn test_split_tokens() {
        let tests = [
            ("bob", vec!["bob"]),
            ("alice,bob", vec!["alice", ",", "bob"]),
        ];

        for (input, expected) in tests {
            let result = split_tokens(input).unwrap();
            assert_eq!(result, expected)
        }
    }

    #[test]
    fn test_group_alternatives() {
        let tests = [
            (vec![], vec![]),
            (vec!["bob"], vec![vec!["bob"]]),
            (vec!["alice", ",", "bob"], vec![vec!["alice"], vec!["bob"]]),
            (vec!["alice", "+", "bob"], vec![vec!["alice", "+", "bob"]]),
        ];

        for (input, expected) in tests {
            let result =
                group_alternatives(input.into_iter().map(|v| v.to_string()).collect()).unwrap();
            assert_eq!(result, expected);
        }
    }

    #[test]
    fn test_parse_section() {
        let tests = [
            ("S=bob", ("S", vec![vec!["bob"]])),
            ("L=alice+bob", ("L", vec![vec!["alice", "+", "bob"]])),
        ];

        for (input, (expected_label, excpected_alternatives)) in tests {
            let (label, alternatives) = parse_section(input).unwrap();
            assert_eq!(label, expected_label);
            assert_eq!(alternatives, excpected_alternatives);
        }
    }
}

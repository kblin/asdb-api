// License: GNU Affero General Public License v3 or later
// A copy of GNU AGPL v3 should have been included in this software package in LICENSE.txt.

use nom::{
    bytes::complete::tag,
    sequence::{delimited, tuple},
    IResult,
};
use serde::{Deserialize, Serialize};

use super::parser::contrib::take_until_unbalanced;
use crate::Error;

pub mod tfbs;

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, strum::AsRefStr)]
pub enum Operator {
    #[serde(rename = ">")]
    Greater,
    #[serde(rename = ">=")]
    GreaterOrEqual,
    #[serde(rename = "==")]
    Equal,
    #[serde(rename = "<=")]
    LessOrEqual,
    #[serde(rename = "<")]
    Less,
}

impl Operator {
    pub fn parse(input: &str) -> IResult<&str, Self, Error> {
        let op: Self;
        let remaining: &str;

        if input.len() < 1 {
            return Err(nom::Err::Failure(Error::ParserError));
        }
        if input.len() == 1 {
            remaining = "";
            op = match input {
                ">" => Operator::Greater,
                "<" => Operator::Less,
                _ => return Err(nom::Err::Failure(Error::ParserError)),
            };
        } else {
            let op_raw = &input[..2];
            remaining = &input[2..];
            op = match op_raw {
                ">=" => Operator::GreaterOrEqual,
                "==" => Operator::Equal,
                "<=" => Operator::LessOrEqual,
                _ => return Err(nom::Err::Failure(Error::ParserError)),
            }
        }
        Ok((remaining, op))
    }
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
pub struct BooleanFilter {
    pub name: String,
}

impl BooleanFilter {
    pub fn new(name: &str) -> Self {
        Self {
            name: name.to_owned(),
        }
    }
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
pub struct NumericalFilter {
    pub name: String,
    pub value: f32,
}

impl NumericalFilter {
    pub fn new(name: &str, value: f32) -> Self {
        Self {
            name: name.to_owned(),
            value,
        }
    }
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
pub struct QualitativeFilter {
    pub name: String,
    pub value: f32,
    pub operator: Operator,
}

impl QualitativeFilter {
    pub fn new(name: &str, value: f32, operator: Operator) -> Self {
        Self {
            name: name.to_owned(),
            value,
            operator,
        }
    }
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
pub struct TextFilter {
    pub name: String,
    pub value: String,
}

impl TextFilter {
    pub fn new(name: &str, value: &str) -> Self {
        Self {
            name: name.to_owned(),
            value: value.to_owned(),
        }
    }
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, strum::AsRefStr)]
#[serde(untagged)]
#[strum(serialize_all = "lowercase")]
pub enum Filter {
    Qualitative(QualitativeFilter),
    Numerical(NumericalFilter),
    Text(TextFilter),
    Boolean(BooleanFilter),
}

impl Filter {
    pub fn parse(input: &str) -> IResult<&str, Self, Error> {
        let filter: Filter;

        let (remaining, (_, inner)) = tuple((
            tag(" WITH "),
            delimited(tag("["), take_until_unbalanced('[', ']'), tag("]")),
        ))(input)?;

        if let Some((name, value_raw)) = inner.split_once("|") {
            if let Some((operator_raw, value)) = value_raw.split_once(":") {
                let (_, op) = Operator::parse(operator_raw)?;
                let Ok(val) = value.parse::<f32>() else {
                    return Err(nom::Err::Failure(Error::InvalidRequest(format!("failed to parse filter value {value_raw}"))))
                };
                filter = Filter::Qualitative(QualitativeFilter::new(name, val, op));
            } else {
                let Ok(value) = value_raw.parse::<f32>() else {
                    return Ok((remaining, Filter::Text(TextFilter::new(name, value_raw))))
                };
                filter = Filter::Numerical(NumericalFilter::new(name, value));
            }
        } else {
            filter = Filter::Boolean(BooleanFilter::new(inner));
        }

        Ok((remaining, filter))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_operator() {
        let tests = [
            (">", Operator::Greater),
            (">=", Operator::GreaterOrEqual),
            ("==", Operator::Equal),
            ("<=", Operator::LessOrEqual),
            ("<", Operator::Less),
        ];
        for (input, expected_output) in tests {
            let (_, op) = Operator::parse(input).unwrap();
            assert_eq!(op, expected_output);
        }
    }

    #[test]
    fn test_parse_filter() {
        let tests = [
            (" WITH [bob]", Filter::Boolean(BooleanFilter::new("bob"))),
            (
                " WITH [alice|bob]",
                Filter::Text(TextFilter::new("alice", "bob")),
            ),
            (
                " WITH [alice|==:30]",
                Filter::Qualitative(QualitativeFilter::new("alice", 30.0, Operator::Equal)),
            ),
        ];
        for (input, expected_output) in tests {
            let (_, filter) = Filter::parse(input).unwrap();
            assert_eq!(filter, expected_output);
        }
    }

    #[test]
    fn test_filter_from_json() {
        let tests = [(
            r#"{"name": "quality", "operator": ">=", "value": 30}"#,
            Filter::Qualitative(QualitativeFilter::new(
                "quality",
                30.0,
                Operator::GreaterOrEqual,
            )),
        )];

        for (input, expected) in tests {
            let result: Filter = serde_json::from_str(input).unwrap();
            assert_eq!(result, expected);
        }
    }
}

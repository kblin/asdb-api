// License: GNU Affero General Public License v3 or later
// A copy of GNU AGPL v3 should have been included in this software package in LICENSE.txt.

use nom::IResult;
use serde::{Deserialize, Serialize};

pub mod expression;
pub mod filters;
pub mod module;
pub mod operation;
pub mod parser;

pub use crate::search::Category;
use crate::{Error, Result};
pub use expression::Expression;
pub use filters::Filter;
pub use operation::{Operation, Operator};

#[derive(Debug, Deserialize, Serialize, PartialEq, Eq, Clone)]
#[serde(rename_all = "lowercase")]
pub enum SearchType {
    Region,
    Gene,
    Domain,
}

#[derive(Debug, Deserialize, Serialize, PartialEq, Eq, Clone)]
#[serde(rename_all = "lowercase")]
pub enum ReturnType {
    Json,
    Csv,
    Fasta,
    Fastaa,
    Genbank,
}

#[derive(Debug, Deserialize, Serialize, PartialEq)]
#[serde(tag = "termType", rename_all = "lowercase")]
pub enum Term {
    Expr(Expression),
    Op(Operation),
}

impl Term {
    pub fn parse(input: &str) -> IResult<&str, Self, Error> {
        if input.starts_with('(') {
            let (remaining, op) = Operation::parse(input)?;
            return Ok((remaining, Term::Op(op)));
        }
        let (remaining, expr) = Expression::parse(input)?;
        Ok((remaining, Term::Expr(expr)))
    }
}

#[derive(Debug, Deserialize, Serialize)]
pub struct Query {
    pub terms: Term,
    #[serde(rename = "search")]
    pub search_type: SearchType,
    pub return_type: ReturnType,
    #[serde(default)]
    pub verbose: bool,
}

impl Query {
    pub fn from_str(input: &str) -> Result<Self> {
        let (_, term) = Term::parse(input).or_else(|_| return Err(Error::ParserError))?;
        Ok(Self {
            terms: term,
            search_type: SearchType::Region,
            return_type: ReturnType::Json,
            verbose: false,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_operation() {
        let tests = [
            (
                "{[acc]}",
                Term::Expr(Expression::new(Category::Acc, None, &[], 1)),
            ),
            (
                "({[acc]} AND {[type]})",
                Term::Op(Operation::new(
                    Operator::And,
                    Term::Expr(Expression::new(Category::Acc, None, &[], 1)),
                    Term::Expr(Expression::new(Category::Type, None, &[], 1)),
                )),
            ),
        ];
        for (input, expected_output) in tests {
            let (_, output) = Term::parse(input).unwrap();
            assert_eq!(output, expected_output);
        }
    }
}

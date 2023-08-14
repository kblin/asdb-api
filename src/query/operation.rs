// License: GNU Affero General Public License v3 or later
// A copy of GNU AGPL v3 should have been included in this software package in LICENSE.txt.

use nom::{
    branch::alt,
    bytes::complete::{tag, tag_no_case},
    character::complete::{multispace0, multispace1},
    sequence::delimited,
    IResult,
};
use serde::{Deserialize, Serialize};

use super::parser::contrib::take_until_unbalanced;
use super::Term;
use crate::Error;

#[derive(Debug, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "UPPERCASE")]
pub enum Operator {
    And,
    Or,
    Except,
}

impl Operator {
    pub fn parse(input: &str) -> IResult<&str, Self, Error> {
        let (remaining, raw_op) =
            alt((tag_no_case("and"), tag_no_case("or"), tag_no_case("except")))(input)?;
        let op = match raw_op.to_lowercase().as_str() {
            "and" => Operator::And,
            "or" => Operator::Or,
            "except" => Operator::Except,
            _ => {
                eprintln!("{}", raw_op);
                return Err(nom::Err::Failure(Error::ParserError));
            }
        };
        Ok((remaining, op))
    }
}

#[derive(Debug, Deserialize, Serialize)]
pub struct Operation {
    #[serde(rename = "operation")]
    pub operator: Operator,
    pub left: Box<Term>,
    pub right: Box<Term>,
}

impl Operation {
    pub fn new(operator: Operator, left: Term, right: Term) -> Self {
        Operation {
            operator,
            left: left.into(),
            right: right.into(),
        }
    }

    pub fn parse(input: &str) -> IResult<&str, Self, Error> {
        let (remaining, partial) =
            delimited(tag("("), take_until_unbalanced('(', ')'), tag(")"))(input)?;

        let (partial, _) = multispace0(partial)?;
        let (partial, left) = Term::parse(partial)?;
        let (partial, _) = multispace1(partial)?;
        let (partial, op) = Operator::parse(partial)?;
        let (partial, _) = multispace1(partial)?;
        let (partial, right) = Term::parse(partial)?;
        let (partial, _) = multispace0(partial)?;
        if partial.len() > 0 {
            return Err(nom::Err::Failure(Error::ParserError));
        }

        return Ok((remaining, Operation::new(op, left, right)));
    }
}

impl PartialEq for Operation {
    fn eq(&self, other: &Self) -> bool {
        if self.operator != other.operator {
            return false;
        }
        if *self.left != *other.left {
            return false;
        }
        if *self.right != *other.right {
            return false;
        }
        true
    }
}

#[cfg(test)]
mod tests {
    use super::super::Expression;
    use super::*;
    use crate::search::Category;

    #[test]
    fn test_parse_operator() {
        let tests = [
            ("AND", Operator::And),
            ("OR", Operator::Or),
            ("EXCEPT", Operator::Except),
        ];
        for (input, expected_output) in tests {
            let (_, output) = Operator::parse(input).unwrap();
            assert_eq!(output, expected_output);
        }
    }

    #[test]
    fn test_parse_operation() {
        let tests = [
            (
                "({[acc]} AND {[type]})",
                Operation::new(
                    Operator::And,
                    Term::Expr(Expression::new(Category::Acc, None, &[], 1)),
                    Term::Expr(Expression::new(Category::Type, None, &[], 1)),
                ),
            ),
            (
                "({[acc]} OR {[type]})",
                Operation::new(
                    Operator::Or,
                    Term::Expr(Expression::new(Category::Acc, None, &[], 1)),
                    Term::Expr(Expression::new(Category::Type, None, &[], 1)),
                ),
            ),
            (
                "({[acc]} EXCEPT {[type]})",
                Operation::new(
                    Operator::Except,
                    Term::Expr(Expression::new(Category::Acc, None, &[], 1)),
                    Term::Expr(Expression::new(Category::Type, None, &[], 1)),
                ),
            ),
            (
                "({[acc]} AND ({[type]} OR {[tfbs]}))",
                Operation::new(
                    Operator::And,
                    Term::Expr(Expression::new(Category::Acc, None, &[], 1)),
                    Term::Op(Operation::new(
                        Operator::Or,
                        Term::Expr(Expression::new(Category::Type, None, &[], 1)),
                        Term::Expr(Expression::new(Category::Tfbs, None, &[], 1)),
                    )),
                ),
            ),
        ];
        for (input, expected_output) in tests {
            let (_, output) = Operation::parse(input).unwrap();
            assert_eq!(output, expected_output);
        }
    }
}

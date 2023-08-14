// License: GNU Affero General Public License v3 or later
// A copy of GNU AGPL v3 should have been included in this software package in LICENSE.txt.

use nom::{
    bytes::complete::tag,
    sequence::{delimited, terminated},
    IResult,
};
use serde::{Deserialize, Serialize};

use super::{
    filters::Filter,
    parser::{contrib::take_until_unbalanced, parse_number, with_mustache},
};
use crate::search::Category;
use crate::Error;

#[derive(Debug, Deserialize, Serialize, PartialEq)]
pub struct Expression {
    pub category: Category,
    pub value: String,
    pub filters: Vec<Filter>,
    pub count: i64,
}

impl Expression {
    pub fn new(category: Category, value: Option<&str>, filters: &[Filter], count: i64) -> Self {
        Expression {
            category,
            value: {
                match value {
                    Some(val) => val.to_owned(),
                    None => "".to_owned(),
                }
            },
            filters: filters.to_vec(),
            count,
        }
    }
    pub fn parse(input: &str) -> IResult<&str, Self, Error> {
        let count: i64;
        let remaining: &str;
        let mut filters: Vec<Filter> = Vec::new();

        if input.len() < 5 {
            return Err(nom::Err::Failure(Error::ParserError));
        }

        if input.chars().next().unwrap().is_numeric() {
            (remaining, count) = terminated(parse_number::<i64>, tag("*"))(input)?;
        } else {
            remaining = input;
            count = 1;
        }

        let (remaining, inner) = with_mustache(remaining)?;
        let (mut filters_raw, term) =
            delimited(tag("["), take_until_unbalanced('[', ']'), tag("]"))(inner)?;

        while filters_raw.len() > 0 {
            let (rest, filter) = Filter::parse(filters_raw)?;
            filters_raw = rest;
            filters.push(filter);
        }

        let parts: Vec<&str> = term.split("|").collect();
        let (_, category) = Category::parse(parts[0])?;
        let value = match parts.len() {
            1 => None,
            2 => Some(parts[1]),
            _ => return Err(nom::Err::Failure(Error::ParserError)),
        };

        Ok((remaining, Expression::new(category, value, &filters, count)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::query::filters::{Filter, Operator as FilterOperator, QualitativeFilter};

    #[test]
    fn test_parse_expression() {
        let tests = [
            ("{[acc]}", Expression::new(Category::Acc, None, &[], 1)),
            (
                "{[acc|bob]}",
                Expression::new(Category::Acc, Some("bob"), &[], 1),
            ),
            ("3*{[acc]}", Expression::new(Category::Acc, None, &[], 3)),
            (
                "{[acc] WITH [charlie|==:30]}",
                Expression::new(
                    Category::Acc,
                    None,
                    &[Filter::Qualitative(QualitativeFilter::new(
                        "charlie",
                        30.0,
                        FilterOperator::Equal,
                    ))],
                    1,
                ),
            ),
        ];
        for (input, expected_output) in tests {
            let (_, output) = Expression::parse(input).unwrap();
            assert_eq!(output, expected_output);
        }
    }
}

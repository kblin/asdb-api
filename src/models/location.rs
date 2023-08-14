// License: GNU Affero General Public License v3 or later
// A copy of GNU AGPL v3 should have been included in this software package in LICENSE.txt.

use nom::{
    bytes::complete::{take_till, take_until1},
    character::complete::{char, digit1, one_of},
    combinator::map_res,
    error::{Error as NomError, ErrorKind},
    sequence::{delimited, pair, separated_pair},
    IResult,
};

use serde::{Deserialize, Serialize};

use crate::{Error, Result};

#[derive(Debug, Deserialize, Serialize, Clone, Copy, PartialEq, Eq)]
pub enum Strand {
    Forward,
    Reverse,
    Unstranded,
}

impl Strand {
    pub fn parse(input: &str) -> IResult<&str, Self> {
        if input.is_empty() {
            return Ok(("", Self::Unstranded));
        }
        let (remaining, dir) = delimited(char('('), one_of("+-"), char(')'))(input)?;

        let strand = match dir {
            '+' => Self::Forward,
            '-' => Self::Reverse,
            _ => {
                // This should be unreachable, but if we ever get here, one_of() failed.
                return Err(nom::Err::Failure(NomError {
                    input,
                    code: ErrorKind::OneOf,
                }));
            }
        };
        Ok((remaining, strand))
    }
    pub fn to_str(&self) -> &'static str {
        match self {
            Self::Forward => "(+)",
            Self::Reverse => "(-)",
            Self::Unstranded => "",
        }
    }
}

#[derive(Debug, Deserialize, Serialize, Clone, Copy, PartialEq, Eq)]
pub struct SimpleLocation {
    pub start: u32,
    pub end: u32,
    pub strand: Strand,
}

impl SimpleLocation {
    pub fn parse(input: &str) -> IResult<&str, Self> {
        let (remaining, coordinates) = delimited(char('['), take_until1("]"), char(']'))(input)?;
        let (remaining, strand) = Strand::parse(remaining)?;
        let (_, (start, end)) = separated_pair(parse_coord, char(':'), parse_coord)(coordinates)?;
        Ok((remaining, Self { start, end, strand }))
    }
}

#[derive(Debug, Deserialize, Serialize, Clone, PartialEq, Eq)]
pub struct CompoundLocation {
    pub start: u32,
    pub end: u32,
    pub strand: Strand,
    pub parts: Vec<SimpleLocation>,
}

impl CompoundLocation {
    pub fn parse(_input: &str) -> IResult<&str, Self, Error> {
        todo!()
    }
}

#[derive(Debug, Deserialize, Serialize, Clone, PartialEq, Eq)]
pub enum Location {
    Simple(SimpleLocation),
    Compound(CompoundLocation),
}

impl Location {
    pub fn parse(input: &str) -> Result<Location> {
        if !input.contains('{') {
            let Ok((_, loc)) = SimpleLocation::parse(input) else {
                return Err(Error::ParserError);
            };
            return Ok(Location::Simple(loc));
        }
        let Ok((_, loc)) = CompoundLocation::parse(input) else {
            return Err(Error::ParserError);
        };
        Ok(Location::Compound(loc))
    }
}

fn parse_coord(input: &str) -> IResult<&str, u32> {
    let not_digit = take_till(|c: char| c.is_digit(10));
    map_res(pair(not_digit, digit1), |(_, number)| {
        u32::from_str_radix(number, 10)
    })(input)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_coord() {
        let tests = [("1", 1_u32), ("<1", 1), (">1", 1)];
        for (input, expected) in tests {
            let (_, result) = parse_coord(input).unwrap();
            assert_eq!(result, expected);
        }
    }

    #[test]
    fn test_strand() {
        let tests = [
            ("(+)", Strand::Forward),
            ("(-)", Strand::Reverse),
            ("", Strand::Unstranded),
        ];

        for (input, expected) in tests {
            let (_, result) = Strand::parse(input).unwrap();
            assert_eq!(result, expected);
            assert_eq!(result.to_str(), input);
        }
    }

    #[test]
    fn test_simple_location() {
        let tests = [
            (
                "[1:6](+)",
                SimpleLocation {
                    start: 1,
                    end: 6,
                    strand: Strand::Forward,
                },
            ),
            (
                "[1:6](-)",
                SimpleLocation {
                    start: 1,
                    end: 6,
                    strand: Strand::Reverse,
                },
            ),
            (
                "[1:6]",
                SimpleLocation {
                    start: 1,
                    end: 6,
                    strand: Strand::Unstranded,
                },
            ),
            (
                "[<1:>6](+)",
                SimpleLocation {
                    start: 1,
                    end: 6,
                    strand: Strand::Forward,
                },
            ),
        ];
        for (input, expected) in tests {
            let (_, result) = SimpleLocation::parse(input).unwrap();
            assert_eq!(result, expected);
        }
    }
}

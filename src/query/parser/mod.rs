// License: GNU Affero General Public License v3 or later
// A copy of GNU AGPL v3 should have been included in this software package in LICENSE.txt.

use std::str::FromStr;

use nom::{bytes::complete::tag, character::complete::digit1, sequence::delimited, Err, IResult};

pub mod contrib;

use crate::Error;
use contrib::take_until_unbalanced;

pub fn parse_number<T: FromStr>(input: &str) -> IResult<&str, T, Error> {
    let (remain, raw_int) = digit1(input)?;
    match raw_int.parse::<T>() {
        Ok(i) => Ok((remain, i)),
        Err(_) => Err(Err::Failure(Error::ParserError)),
    }
}

pub fn with_mustache(input: &str) -> IResult<&str, &str, Error> {
    delimited(tag("{"), take_until_unbalanced('{', '}'), tag("}"))(input)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_number() {
        let (remaining, output) = parse_number::<i32>("123").unwrap();
        assert_eq!(output, 123_i32);
        assert_eq!(remaining, "");
    }

    #[test]
    fn test_with_mustache() {
        let (_, output) = with_mustache("{bob}").unwrap();
        assert_eq!(output, "bob");
    }
}

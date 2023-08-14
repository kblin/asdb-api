use nom::Err;
use nom::IResult;

use crate::Error;

// This function is borrowed from the parse-hyperlinks crate at https://crates.io/crates/parse-hyperlinks
// under an MIT/Apache-2 dual licence
pub fn take_until_unbalanced(
    opening_bracket: char,
    closing_bracket: char,
) -> impl Fn(&str) -> IResult<&str, &str, Error> {
    move |i: &str| {
        let mut index = 0;
        let mut bracket_counter = 0;
        while let Some(n) = &i[index..].find(&[opening_bracket, closing_bracket, '\\'][..]) {
            index += n;
            let mut it = i[index..].chars();
            match it.next().unwrap_or_default() {
                c if c == '\\' => {
                    // Skip the escape char `\`.
                    index += '\\'.len_utf8();
                    // Skip also the following char.
                    let c = it.next().unwrap_or_default();
                    index += c.len_utf8();
                }
                c if c == opening_bracket => {
                    bracket_counter += 1;
                    index += opening_bracket.len_utf8();
                }
                c if c == closing_bracket => {
                    // Closing bracket.
                    bracket_counter -= 1;
                    index += closing_bracket.len_utf8();
                }
                // Can not happen.
                _ => unreachable!(),
            };
            // We found the unmatched closing bracket.
            if bracket_counter == -1 {
                // We do not consume it.
                index -= closing_bracket.len_utf8();
                return Ok((&i[index..], &i[0..index]));
            };
        }

        if bracket_counter == 0 {
            Ok(("", i))
        } else {
            Err(Err::Failure(Error::ParserError))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_take_until_unmatched() {
        assert_eq!(
            take_until_unbalanced('(', ')')("url)abc").unwrap(),
            (")abc", "url")
        );
        assert_eq!(
            take_until_unbalanced('(', ')')("u()rl)abc").unwrap(),
            (")abc", "u()rl")
        );
        assert_eq!(
            take_until_unbalanced('(', ')')("u(())rl)abc").unwrap(),
            (")abc", "u(())rl")
        );
        assert_eq!(
            take_until_unbalanced('(', ')')("u(())r()l)abc").unwrap(),
            (")abc", "u(())r()l")
        );
        assert_eq!(
            take_until_unbalanced('(', ')')("u(())r()labc").unwrap(),
            ("", "u(())r()labc")
        );
        assert_eq!(
            take_until_unbalanced('(', ')')(r#"u\((\))r()labc"#).unwrap(),
            ("", r#"u\((\))r()labc"#)
        );
        assert_eq!(
            take_until_unbalanced('€', 'ü')("€uü€€üürlüabc").unwrap(),
            ("üabc", "€uü€€üürl")
        );
    }
}

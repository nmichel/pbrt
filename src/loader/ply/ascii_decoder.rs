use std::fmt::Display;
use std::str::FromStr;

use super::ValueDecoder;

/// Reads the body of an `ascii` PLY file.
///
/// Reference: Greg Turk, *The PLY Polygon File Format* — in the ASCII variant every value is
/// written in decimal and separated from the next by whitespace, one element per line.
///
/// A block of data is therefore a **token**, and decoding it is a decimal parse. The decoder
/// refills its token buffer lazily, one line at a time, which means the trait needs no notion of
/// "end of row": a property that spans the rest of a line and a property that starts the next one
/// are read the same way. Empty lines and `comment` lines are skipped wherever they appear, as the
/// header reader does.
pub struct AsciiDecoder<'a> {
    lines: std::str::Lines<'a>,
    tokens: std::str::SplitWhitespace<'a>,
}

impl<'a> AsciiDecoder<'a> {
    pub fn new(body: &'a str) -> AsciiDecoder<'a> {
        AsciiDecoder {
            lines: body.lines(),
            tokens: "".split_whitespace(),
        }
    }

    fn next_token(&mut self) -> Result<&'a str, String> {
        loop {
            if let Some(token) = self.tokens.next() {
                return Ok(token);
            }

            let line = self.lines.next().ok_or_else(|| "Unexpected end of data".to_string())?;
            let line = line.trim();
            if line.is_empty() || line.starts_with("comment") {
                continue;
            }

            self.tokens = line.split_whitespace();
        }
    }

    fn parse_next<T>(&mut self) -> Result<T, String>
    where
        T: FromStr,
        T::Err: Display,
    {
        let token = self.next_token()?;
        token
            .parse::<T>()
            .map_err(|err| format!("Cannot read '{}' as {}: {}", token, std::any::type_name::<T>(), err))
    }
}

impl<'a> ValueDecoder for AsciiDecoder<'a> {
    fn read_i8(&mut self) -> Result<i8, String> {
        self.parse_next()
    }

    fn read_u8(&mut self) -> Result<u8, String> {
        self.parse_next()
    }

    fn read_i16(&mut self) -> Result<i16, String> {
        self.parse_next()
    }

    fn read_u16(&mut self) -> Result<u16, String> {
        self.parse_next()
    }

    fn read_i32(&mut self) -> Result<i32, String> {
        self.parse_next()
    }

    fn read_u32(&mut self) -> Result<u32, String> {
        self.parse_next()
    }

    fn read_f32(&mut self) -> Result<f32, String> {
        self.parse_next()
    }

    fn read_f64(&mut self) -> Result<f64, String> {
        self.parse_next()
    }
}

#[cfg(test)]
mod test {
    use super::{AsciiDecoder, ValueDecoder};

    #[test]
    fn test_reads_tokens_across_lines() {
        let mut decoder = AsciiDecoder::new("0.5 -1.5\n3\n");

        assert_eq!(decoder.read_f32().unwrap(), 0.5);
        assert_eq!(decoder.read_f32().unwrap(), -1.5);
        assert_eq!(decoder.read_u8().unwrap(), 3);
    }

    #[test]
    fn test_skips_blank_and_comment_lines() {
        let mut decoder = AsciiDecoder::new("\ncomment ignore me\n  7  \n");

        assert_eq!(decoder.read_i32().unwrap(), 7);
    }

    #[test]
    fn test_tolerates_repeated_and_tab_separators() {
        let mut decoder = AsciiDecoder::new("1\t\t2   3");

        assert_eq!(decoder.read_i32().unwrap(), 1);
        assert_eq!(decoder.read_i32().unwrap(), 2);
        assert_eq!(decoder.read_i32().unwrap(), 3);
    }

    #[test]
    fn test_end_of_data_is_an_error() {
        let mut decoder = AsciiDecoder::new("1");

        assert!(decoder.read_i32().is_ok());
        assert!(decoder.read_i32().is_err());
    }

    #[test]
    fn test_malformed_token_is_an_error() {
        let mut decoder = AsciiDecoder::new("not_a_number");

        assert!(decoder.read_f32().is_err());
    }
}

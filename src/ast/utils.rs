use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ParseError<'a> {
    InvalidNumber(&'a str),
    NumberOverflow(&'a str),
}

impl<'a> fmt::Display for ParseError<'a> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidNumber(s) => write!(f, "invalid number '{s}'"),
            Self::NumberOverflow(s) => write!(f, "number overflow '{s}'"),
        }
    }
}

pub fn parse_number<'a>(s: &'a str) -> Result<f64, ParseError<'a>> {
    let bytes = s.as_bytes();
    if bytes.is_empty() || !bytes[0].is_ascii_digit() {
        return Err(ParseError::InvalidNumber(s));
    }

    // Leading '0' can only be followed by '.', 'e', or 'E'
    if bytes[0] == b'0' && bytes.len() > 1 && !matches!(bytes[1], b'.' | b'e' | b'E') {
        return Err(ParseError::InvalidNumber(s));
    }

    // '.' must be immediately followed by an ASCII digit
    if let Some(dot_idx) = memchr::memchr(b'.', bytes) {
        if dot_idx + 1 >= bytes.len() || !bytes[dot_idx + 1].is_ascii_digit() {
            return Err(ParseError::InvalidNumber(s));
        }
    }

    // If no underscores, just parse
    if memchr::memchr(b'_', bytes).is_none() {
        let v = s.parse::<f64>().map_err(|_| ParseError::InvalidNumber(s))?;
        return if v.is_infinite() {
            Err(ParseError::NumberOverflow(s))
        } else {
            Ok(v)
        };
    }

    let mut clean = String::with_capacity(bytes.len());
    for i in 0..bytes.len() {
        let c = bytes[i];
        if c == b'_' {
            if i == 0
                || i + 1 >= bytes.len()
                || !bytes[i - 1].is_ascii_digit()
                || !bytes[i + 1].is_ascii_digit()
            {
                return Err(ParseError::InvalidNumber(s));
            }
        } else {
            clean.push(c as char);
        }
    }

    let v = clean
        .parse::<f64>()
        .map_err(|_| ParseError::InvalidNumber(s))?;
    if v.is_infinite() {
        return Err(ParseError::NumberOverflow(s));
    }
    Ok(v)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_standard_numbers() {
        let cases = [
            // From arith_float.jsonnet
            ("0", 0.0),
            ("0.0", 0.0),
            ("0e0", 0.0),
            ("0e+0", 0.0),
            ("0e-0", 0.0),
            ("0e00", 0.0),
            ("0E0", 0.0),
            ("0E+0", 0.0),
            ("0E-0", 0.0),
            ("0.0e0", 0.0),
            ("0.00E-0", 0.0),
            ("1", 1.0),
            ("1.0", 1.0),
            ("1e0", 1.0),
            ("1e+0", 1.0),
            ("1e-0", 1.0),
            ("1.00E+0", 1.0),
            ("11", 11.0),
            ("11.0", 11.0),
            ("11e0", 11.0),
            ("11.00e-0", 11.0),
            ("4.5", 4.5),
            ("1e20", 1e20),
            ("1.1e100", 1.1e100),
            ("1.1e-100", 1.1e-100),
            ("1.1e+100", 1.1e+100),
            ("9007199254740991", 9007199254740991.0),
            ("1.7976931348623157e308", f64::MAX),
        ];

        for (input, expected) in cases {
            assert_eq!(parse_number(input), Ok(expected), "failed on {input}");
        }
    }

    #[test]
    fn test_valid_digit_separators() {
        // From digitsep.jsonnet & go-jsonnet lexer_test.go
        let cases = [
            ("123_456", 123_456.0),
            ("1_750_000", 1_750_000.0),
            ("1_2_3", 123.0),
            ("3.141_592", 3.141_592),
            ("1_200.0", 1_200.0),
            ("0.0_1", 0.01),
            ("0e1_01", 0e101),
            ("10_10e3", 1010e3),
            ("2_3e1_2", 23e12),
            ("1.1_2e100", 1.12e100),
            ("1.1e-10_1", 1.1e-101),
            ("1.1e+10_1", 1.1e+101),
            ("9.109_383_56e-31", 9.109_383_56e-31),
        ];

        for (input, expected) in cases {
            assert_eq!(parse_number(input), Ok(expected), "failed on {input}");
        }
    }

    #[test]
    fn test_invalid_numbers_and_separators() {
        // From go-jsonnet lexer_test.go & error.parse.static_error_bad_number.jsonnet
        let invalid = [
            "", ".5", "1.", "1.e5", "1.+3", "00", "05", "042", "0_5", "0_0", "_123", "123_",
            "123456_!", "123__456", "1_200_.0", "1_200._0", "1_200_e2", "1_200e_2", "200e-_2",
            "200e+_2", "1e", "1e+", "1e-", "1e!", "1e+!", "0x42", "NaN", "inf", "infinity",
        ];

        for input in invalid {
            assert_eq!(
                parse_number(input),
                Err(ParseError::InvalidNumber(input)),
                "expected InvalidNumber for {input:?}"
            );
        }
    }

    #[test]
    fn test_number_overflow() {
        // From error.overflow.jsonnet & error.overflow3.jsonnet
        let overflows = ["1e309", "1e500", "1_000e308", "1.7976931348623159e308"];

        for input in overflows {
            assert_eq!(
                parse_number(input),
                Err(ParseError::NumberOverflow(input)),
                "expected NumberOverflow for {input:?}"
            );
        }
    }
}

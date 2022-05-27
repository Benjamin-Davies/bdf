use crate::chars::{
  is_alphabetic_char, is_name_char, is_newline_char, is_numeric_char, is_whitespace_char, peek_char,
};
use crate::error::{Error, Result};
use std::borrow::Cow;
use std::cmp::min;
use std::num::ParseIntError;
use std::str::FromStr;

/// Every parser returns a result containing a tuple. The first element is the
/// object that was parsed, and the second is the remaining bytes to be parsed.
pub type ParseResult<'a, T> = Result<(T, &'a [u8])>;

/// A token is an object, somewhere between a character and an object in
/// complexity. Some tokens constitute the entire object (eg. Name, Int, Float),
/// while others are markers for the ends of objects.
#[derive(Debug, PartialEq)]
pub enum Token<'a> {
  Keyword(Cow<'a, [u8]>),
  Integer(usize),
  Real(f32),
  LiteralString(Cow<'a, [u8]>),
  Name(Cow<'a, [u8]>),
}

/// Parses a block of whitespace, including comments (Adobe, 2008, p. 13).
pub fn parse_whitespace(mut raw: &[u8]) -> ParseResult<()> {
  loop {
    let next = peek_char(raw)?;
    if is_whitespace_char(next) {
      raw = &raw[1..];
    } else if next == b'%' {
      while !is_newline_char(peek_char(raw)?) {
        raw = &raw[1..];
      }
    } else {
      break;
    }
  }

  Ok(((), raw))
}

/// Parses an integer.
///
/// This is not used for parsing tokens, but is instead used to parse (some of)
/// the numbers used in the trailer and xref table.
pub fn parse_number<I: FromStr<Err = ParseIntError>>(raw: &[u8]) -> ParseResult<I> {
  let ((), raw) = parse_whitespace(raw)?;

  let mut length = 0;
  while is_numeric_char(peek_char(&raw[length..])?) {
    length += 1;
  }

  let number = String::from_utf8_lossy(&raw[..length]).parse()?;

  Ok((number, &raw[length..]))
}

/// Parses a keyword, which must consist exclusively of alphabetic characters.
pub fn parse_keyword(raw: &[u8]) -> ParseResult<Cow<[u8]>> {
  let mut length = 0;
  while is_alphabetic_char(peek_char(&raw[length..])?) {
    length += 1;
  }

  let keyword = raw[..length].into();

  Ok((keyword, &raw[length..]))
}

/// Parses a numeric object, either as an int or as a float
/// (Adobe, 2008, p. 14).
pub fn parse_numeric(raw: &[u8]) -> ParseResult<Token> {
  let mut contains_decimal = false;
  let mut length = 0;
  while is_numeric_char(peek_char(&raw[length..])?) {
    if raw[length] == b'.' {
      contains_decimal = true;
    }

    length += 1;
  }

  let token = if contains_decimal {
    let number = String::from_utf8_lossy(&raw[..length]).parse()?;
    Token::Real(number)
  } else {
    let number = String::from_utf8_lossy(&raw[..length]).parse()?;
    Token::Integer(number)
  };

  Ok((token, &raw[length..]))
}

/// Parses an escape sequence, such as those that may occur in a literal string
/// (Adobe, 2008, p. 15).
pub fn parse_escape_sequence(raw: &[u8]) -> ParseResult<Option<u8>> {
  if peek_char(raw)? != b'\\' {
    return Err(Error::Syntax("Escape Sequence must start with a '\\'"));
  }

  // First try parsing an octal escape sequence
  let first_non_octal_position = raw
    .iter()
    .skip(1)
    .take(3)
    .position(|&c| c < b'0' || c >= b'8');
  if first_non_octal_position != Some(0) {
    let digit_count = match first_non_octal_position {
      Some(n) => n,
      None => min(3, raw.len() - 1),
    };
    let octal = String::from_utf8_lossy(&raw[1..1 + digit_count]);
    let byte = u8::from_str_radix(&octal, 8)?;
    return Ok((Some(byte), &raw[1 + digit_count..]));
  }

  let c = peek_char(&raw[1..])?;
  let (result, length) = match c {
    b'n' => (Some(b'\n'), 2),
    b'r' => (Some(b'\n'), 2),
    b't' => (Some(b'\t'), 2),
    // BACKSPACE (BS)
    b'b' => (Some(0x08), 2),
    // FORM FEED (FF)
    b'f' => (Some(0x0C), 2),
    b'(' | b')' | b'\\' => (Some(c), 2),
    b'\n' => (None, 2),
    b'\r' => (
      Some(b'\n'),
      if peek_char(&raw[2..]) == Ok(b'\n') {
        3
      } else {
        2
      },
    ),
    _ => {
      return Err(Error::Syntax("Invalid escape sequence"));
    }
  };

  Ok((result, &raw[length..]))
}

/// Parses a literal string (Adobe, 2008, p. 15-16).
pub fn parse_literal_string(raw: &[u8]) -> ParseResult<Cow<[u8]>> {
  if raw[0] != b'(' {
    return Err(Error::Syntax("Literal String must start with '('"));
  }

  let mut length = 1;
  let mut depth = 1;
  let mut requires_extra_processing = false;

  while depth > 0 {
    match peek_char(&raw[length..])? {
      b'(' => depth += 1,
      b')' => depth -= 1,
      b'\\' => {
        requires_extra_processing = true;
        length += 1;
      }
      b'\r' => {
        requires_extra_processing = true;
      }
      _ => {}
    }
    length += 1;
  }

  let string = if requires_extra_processing {
    let mut raw = &raw[1..length - 1];
    let mut bytes = Vec::with_capacity(length);

    while raw.len() > 0 {
      match raw[0] {
        b'\\' => {
          let (result, next) = parse_escape_sequence(raw)?;
          if let Some(c) = result {
            bytes.push(c);
          }
          raw = next;
        }
        b'\r' => {
          bytes.push(b'\n');
          raw = &raw[1..];
          if peek_char(raw) == Ok(b'\n') {
            raw = &raw[1..];
          }
        }
        _ => {
          bytes.push(raw[0]);
          raw = &raw[1..];
        }
      }
    }

    bytes.into()
  } else {
    raw[1..length - 1].into()
  };

  Ok((string, &raw[length..]))
}

/// Parses a name object (Adobe, 2008, p. 16).
pub fn parse_name(raw: &[u8]) -> ParseResult<Cow<[u8]>> {
  if peek_char(raw)? != b'/' {
    return Err(Error::Syntax("Name must start with a '/'"));
  }
  let raw = &raw[1..];

  let mut contains_escapes = false;
  let mut length = 0;
  while is_name_char(peek_char(&raw[length..])?) {
    if raw[length] == b'#' {
      contains_escapes = true;
    }

    length += 1;
  }

  let name = if contains_escapes {
    let mut bytes = Vec::with_capacity(length);
    let mut i = 0;
    while i < length {
      match raw[i] {
        b'#' => {
          let hex = String::from_utf8_lossy(&raw[i + 1..i + 3]);
          bytes.push(u8::from_str_radix(&hex, 16)?);
          i += 3;
        }
        _ => {
          bytes.push(raw[i]);
          i += 1;
        }
      }
    }
    bytes.into()
  } else {
    raw[..length].into()
  };

  Ok((name, &raw[length..]))
}

/// Parses a token, automatically detecting its type.
pub fn parse_token(raw: &[u8]) -> ParseResult<Token> {
  let ((), raw) = parse_whitespace(raw)?;

  let first_char = peek_char(raw)?;
  if is_numeric_char(first_char) {
    parse_numeric(raw)
  } else if is_alphabetic_char(first_char) {
    let (keyword, raw) = parse_keyword(raw)?;
    Ok((Token::Keyword(keyword), raw))
  } else if first_char == b'/' {
    let (name, raw) = parse_name(raw)?;
    Ok((Token::Name(name), raw))
  } else if first_char == b'(' {
    let (string, raw) = parse_literal_string(raw)?;
    Ok((Token::LiteralString(string), raw))
  } else {
    Err(Error::Syntax("Unrecognised token"))
  }
}

#[cfg(test)]
mod test {
  use super::*;

  macro_rules! assert_eq_cow {
    ($left:expr, $right:expr $(,)?) => {
      assert_eq!($left, Cow::Borrowed($right));
    };
  }

  #[test]
  fn should_peek_char() {
    assert_eq!(peek_char(b"Hello, world!").unwrap(), b'H');
  }

  #[test]
  fn should_parse_whitespace() {
    let ((), rest) = parse_whitespace(b" \t \r\nHello, world!").unwrap();
    assert_eq!(rest, b"Hello, world!");
  }

  #[test]
  fn should_parse_comments_as_whitespace() {
    let ((), rest) = parse_whitespace(b"\r\n% A Simple Comment\nHello, world!").unwrap();
    assert_eq!(rest, b"Hello, world!");
  }

  #[test]
  fn should_parse_keyword() {
    let (keyword, rest) = parse_keyword(b"keyword  ").unwrap();
    assert_eq_cow!(keyword, b"keyword");
    assert_eq!(rest, b"  ");
  }

  #[test]
  fn should_parse_number() {
    let (number, rest) = parse_number::<usize>(b"  42  ").unwrap();
    assert_eq!(number, 42);
    assert_eq!(rest, b"  ");
  }

  #[test]
  fn should_parse_literal_string() {
    const TEST_CASES: &[(&[u8], &str)] = &[
      (b"(This is a string)", "This is a string"),
      (
        b"(Strings may contain newlines\nas such.)",
        "Strings may contain newlines\nas such.",
      ),
      (
        b"(Strings may contain balanced parentheses () and\r\nspecial characters (*!&}^% and so on).)",
        "Strings may contain balanced parentheses () and\nspecial characters (*!&}^% and so on).",
      ),
      (
        b"(The following is an empty string.)",
        "The following is an empty string.",
      ),
      (
        b"()",
        "",
      ),
      (
        b"(It has zero (0) length.)",
        "It has zero (0) length.",
      ),
      (
        b"(These \\\ntwo strings \\\nare the same.)",
        "These two strings are the same.",
      ),
      (
        b"(This string has and end-of-line at the end of it.\n)",
        "This string has and end-of-line at the end of it.\n",
      ),
      (
        b"(\\0533)",
        "+3",
      ),
      (
        b"(\\53)",
        "+",
      ),
    ];

    for (raw, expected) in TEST_CASES {
      let (string, _raw) = parse_literal_string(raw).unwrap();
      assert_eq!(String::from_utf8_lossy(&string), Cow::Borrowed(*expected));
    }
  }

  #[test]
  fn should_parse_name() {
    let raw = b"/Name1/ASomewhatLongerName/A;Name_With-Various***Characters?/1.2 ";
    let (name, raw) = parse_name(raw).unwrap();
    assert_eq_cow!(name, b"Name1");
    let (name, raw) = parse_name(raw).unwrap();
    assert_eq_cow!(name, b"ASomewhatLongerName");
    let (name, raw) = parse_name(raw).unwrap();
    assert_eq_cow!(name, b"A;Name_With-Various***Characters?");
    let (name, _raw) = parse_name(raw).unwrap();
    assert_eq_cow!(name, b"1.2");

    let raw = b"/$$@pattern/.notdef/Lime#20Green/paired#28#29parentheses ";
    let (name, raw) = parse_name(raw).unwrap();
    assert_eq_cow!(name, b"$$@pattern");
    let (name, raw) = parse_name(raw).unwrap();
    assert_eq_cow!(name, b".notdef");
    let (name, raw) = parse_name(raw).unwrap();
    assert_eq_cow!(name, b"Lime Green");
    let (name, _raw) = parse_name(raw).unwrap();
    assert_eq_cow!(name, b"paired()parentheses");

    let raw = b"/The_Key_of_F#23_Minor/A#42 ";
    let (name, raw) = parse_name(raw).unwrap();
    assert_eq_cow!(name, b"The_Key_of_F#_Minor");
    let (name, _raw) = parse_name(raw).unwrap();
    assert_eq_cow!(name, b"AB");
  }

  #[test]
  fn should_parse_token() {
    let raw = b"/one two +3 +4.0 5 -.6 (seven (7)) ";
    let (token, raw) = parse_token(raw).unwrap();
    assert_eq!(token, Token::Name(Cow::Borrowed(b"one")));
    let (token, raw) = parse_token(raw).unwrap();
    assert_eq!(token, Token::Keyword(Cow::Borrowed(b"two")));
    let (token, raw) = parse_token(raw).unwrap();
    assert_eq!(token, Token::Integer(3));
    let (token, raw) = parse_token(raw).unwrap();
    assert_eq!(token, Token::Real(4.0));
    let (token, raw) = parse_token(raw).unwrap();
    assert_eq!(token, Token::Integer(5));
    let (token, raw) = parse_token(raw).unwrap();
    assert_eq!(token, Token::Real(-0.6));
    let (token, _raw) = parse_token(raw).unwrap();
    assert_eq!(token, Token::LiteralString(Cow::Borrowed(b"seven (7)")));
  }
}

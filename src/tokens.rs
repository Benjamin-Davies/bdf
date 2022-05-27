use crate::chars::{
  is_alphabetic_char, is_name_char, is_newline_char, is_numeric_char, is_whitespace_char, peek_char,
};
use crate::error::{Error, Result};
use std::borrow::Cow;
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
  Name(Cow<'a, [u8]>),
  Integer(usize),
  Real(f32),
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
  } else {
    Err(Error::Syntax("Unrecognised token"))
  }
}

#[cfg(test)]
mod test {
  use super::*;

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
    assert_eq!(keyword, Cow::Borrowed(b"keyword"));
    assert_eq!(rest, b"  ");
  }

  #[test]
  fn should_parse_number() {
    let (number, rest) = parse_number::<usize>(b"  42  ").unwrap();
    assert_eq!(number, 42);
    assert_eq!(rest, b"  ");
  }

  #[test]
  fn should_parse_name() {
    let raw = b"/Name1/ASomewhatLongerName/A;Name_With-Various***Characters?/1.2 ";
    let (name, raw) = parse_name(raw).unwrap();
    assert_eq!(name, Cow::Borrowed(b"Name1"));
    let (name, raw) = parse_name(raw).unwrap();
    assert_eq!(name, Cow::Borrowed(b"ASomewhatLongerName"));
    let (name, raw) = parse_name(raw).unwrap();
    assert_eq!(name, Cow::Borrowed(b"A;Name_With-Various***Characters?"));
    let (name, _raw) = parse_name(raw).unwrap();
    assert_eq!(name, Cow::Borrowed(b"1.2"));

    let raw = b"/$$@pattern/.notdef/Lime#20Green/paired#28#29parentheses ";
    let (name, raw) = parse_name(raw).unwrap();
    assert_eq!(name, Cow::Borrowed(b"$$@pattern"));
    let (name, raw) = parse_name(raw).unwrap();
    assert_eq!(name, Cow::Borrowed(b".notdef"));
    let (name, raw) = parse_name(raw).unwrap();
    assert_eq!(name, Cow::Borrowed(b"Lime Green"));
    let (name, _raw) = parse_name(raw).unwrap();
    assert_eq!(name, Cow::Borrowed(b"paired()parentheses"));

    let raw = b"/The_Key_of_F#23_Minor/A#42 ";
    let (name, raw) = parse_name(raw).unwrap();
    assert_eq!(name, Cow::Borrowed(b"The_Key_of_F#_Minor"));
    let (name, _raw) = parse_name(raw).unwrap();
    assert_eq!(name, Cow::Borrowed(b"AB"));
  }

  #[test]
  fn should_parse_token() {
    let raw = b"/one two +3 +4.0 5 -.6 ";
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
    let (token, _raw) = parse_token(raw).unwrap();
    assert_eq!(token, Token::Real(-0.6));
  }
}

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
  Keyword(Cow<'a, str>),
  Name(Cow<'a, str>),
  Int(usize),
  Float(f32),
}

/// Characters which "delimit syntactic entities such as arrays, names, and
/// comments. Any of these characters terminates the entity preceding it and is
/// not included in the entity." (Adobe, 2008, p. 13)
const DELIMETER_CHARACTERS: &str = "()<>[]{}/%";

/// Characters which may be part of a numeric object token.
const NUMERIC_CHARACTERS: &str = "+-.";

/// Returns the corresponding char for the next byte in the buffer, interpreted
/// as utf8.
///
/// If the buffer is empty, then returns `Err(Error::EOF)`.
///
/// If the next byte is not a valid utf8 character, or it is part of a character
/// that spans multiple bytes, then the function will return
/// [`U+FFFD REPLACEMENT CHARACTER`][U+FFFD], which looks like this: �
#[inline]
fn peek_char(raw: &[u8]) -> Result<char> {
  if raw.len() < 1 {
    return Err(Error::EOF);
  }
  let c = String::from_utf8_lossy(&raw[..1]).chars().next().unwrap();
  Ok(c)
}

/// Returns true if the character constututes whitespace. This includes both
/// what unicode considers whitespace, as well as comments (Adobe, 2008, p. 13).
#[inline]
fn is_whitespace_char(c: char) -> bool {
  c == '%' || c.is_whitespace()
}

/// Returns true if the character is may be part of a name object token
/// (excluding the initial `/`)
#[inline]
fn is_name_char(c: char) -> bool {
  !DELIMETER_CHARACTERS.contains(c) && !is_whitespace_char(c)
}

/// Returns true if the character is may be part of a numeric object token
/// (0-9, +, -, .)
#[inline]
fn is_numeric_char(c: char) -> bool {
  NUMERIC_CHARACTERS.contains(c) || c.is_numeric()
}

/// Parses a block of whitespace, including comments (Adobe, 2008, p. 13).
pub fn parse_whitespace(mut raw: &[u8]) -> ParseResult<()> {
  loop {
    let next = peek_char(raw)?;
    if next.is_whitespace() {
      raw = &raw[1..];
    } else if next == '%' {
      while !"\r\n".contains(peek_char(raw)?) {
        raw = &raw[1..];
      }
    } else {
      break;
    }
  }

  Ok(((), raw))
}

/// Parses a keyword, which must consist exclusively of alphabetic characters.
pub fn parse_keyword(mut raw: &[u8]) -> ParseResult<Cow<str>> {
  ((), raw) = parse_whitespace(raw)?;

  let mut length = 0;
  while peek_char(&raw[length..])?.is_alphabetic() {
    length += 1;
  }

  let keyword = String::from_utf8_lossy(&raw[..length]);

  Ok((keyword, &raw[length..]))
}

/// Parses a simple integer, consisting exclusively of digits.
///
/// This is not used for parsing tokens, but is instead used to parse (some of)
/// the numbers used in the trailer and xref table.
pub fn parse_number<I: FromStr<Err = ParseIntError>>(mut raw: &[u8]) -> ParseResult<I> {
  ((), raw) = parse_whitespace(raw)?;

  let mut length = 0;
  while peek_char(&raw[length..])?.is_numeric() {
    length += 1;
  }

  let number = String::from_utf8_lossy(&raw[..length]).parse()?;

  Ok((number, &raw[length..]))
}

/// Parses a name object (Adobe, 2008, p. 16).
pub fn parse_name(mut raw: &[u8]) -> ParseResult<Cow<str>> {
  ((), raw) = parse_whitespace(raw)?;

  if peek_char(raw)? != '/' {
    return Err(Error::Syntax("Name must start with a '/'"));
  }
  raw = &raw[1..];

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

    // I think that this does exactly one alloc
    // If name is valid utf8: ref, copy, move
    // If name is not valid utf8: copy, move, move
    String::from_utf8_lossy(&bytes).into_owned().into()
  } else {
    String::from_utf8_lossy(&raw[..length])
  };

  Ok((name, &raw[length..]))
}

/// Parses a numeric object, either as an int or as a float
/// (Adobe, 2008, p. 14).
pub fn parse_numeric(mut raw: &[u8]) -> ParseResult<Token> {
  ((), raw) = parse_whitespace(raw)?;

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
    Token::Float(number)
  } else {
    let number = String::from_utf8_lossy(&raw[..length]).parse()?;
    Token::Int(number)
  };

  Ok((token, &raw[length..]))
}

/// Parses a token, automatically detecting its type.
pub fn parse_token(mut raw: &[u8]) -> ParseResult<Token> {
  ((), raw) = parse_whitespace(raw)?;

  let first_char = peek_char(raw)?;
  if is_numeric_char(first_char) {
    parse_numeric(raw)
  } else if first_char.is_alphabetic() {
    let (keyword, raw) = parse_keyword(raw)?;
    Ok((Token::Keyword(keyword), raw))
  } else if first_char == '/' {
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
    assert_eq!(peek_char(b"Hello, world!").unwrap(), 'H');
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
    let (keyword, rest) = parse_keyword(b"  keyword  ").unwrap();
    assert_eq!(keyword, "keyword");
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
    let raw = b"/Name1/ASomewhatLongerName/A;Name_With-Various***Characters?/1.2
      /$$@pattern/.notdef/Lime#20Green/paired#28#29parentheses
      /The_Key_of_F#23_Minor/A#42 ";
    let (name, raw) = parse_name(raw).unwrap();
    assert_eq!(name, "Name1");
    let (name, raw) = parse_name(raw).unwrap();
    assert_eq!(name, "ASomewhatLongerName");
    let (name, raw) = parse_name(raw).unwrap();
    assert_eq!(name, "A;Name_With-Various***Characters?");
    let (name, raw) = parse_name(raw).unwrap();
    assert_eq!(name, "1.2");
    let (name, raw) = parse_name(raw).unwrap();
    assert_eq!(name, "$$@pattern");
    let (name, raw) = parse_name(raw).unwrap();
    assert_eq!(name, ".notdef");
    let (name, raw) = parse_name(raw).unwrap();
    assert_eq!(name, "Lime Green");
    let (name, raw) = parse_name(raw).unwrap();
    assert_eq!(name, "paired()parentheses");
    let (name, raw) = parse_name(raw).unwrap();
    assert_eq!(name, "The_Key_of_F#_Minor");
    let (name, _raw) = parse_name(raw).unwrap();
    assert_eq!(name, "AB");
  }

  #[test]
  fn should_parse_token() {
    let raw = b"/one two +3 +4.0 5 -.6 ";
    let (token, raw) = parse_token(raw).unwrap();
    assert_eq!(token, Token::Name("one".into()));
    let (token, raw) = parse_token(raw).unwrap();
    assert_eq!(token, Token::Keyword("two".into()));
    let (token, raw) = parse_token(raw).unwrap();
    assert_eq!(token, Token::Int(3));
    let (token, raw) = parse_token(raw).unwrap();
    assert_eq!(token, Token::Float(4.0));
    let (token, raw) = parse_token(raw).unwrap();
    assert_eq!(token, Token::Int(5));
    let (token, _raw) = parse_token(raw).unwrap();
    assert_eq!(token, Token::Float(-0.6));
  }
}

use crate::error::{Error, Result};
use std::borrow::Cow;
use std::num::ParseIntError;
use std::str::FromStr;

pub type ParseResult<'a, T> = Result<(T, &'a [u8])>;

pub enum Token<'a> {
  Keyword(Cow<'a, str>),
  Name(Cow<'a, str>),
  Int(usize),
  Float(f32),
}

const DELIMETER_CHARACTERS: &str = "()<>[]{}/%";

#[inline]
fn peek_char(raw: &[u8]) -> Result<char> {
  String::from_utf8_lossy(&raw[..1])
    .chars()
    .next()
    .ok_or(Error::Syntax("Reached EOF"))
}

pub fn parse_whitespace(mut raw: &[u8]) -> ParseResult<()> {
  while peek_char(raw)?.is_whitespace() {
    raw = &raw[1..];
  }
  Ok(((), raw))
}

pub fn parse_keyword(mut raw: &[u8]) -> ParseResult<Cow<str>> {
  ((), raw) = parse_whitespace(raw)?;

  let mut length = 0;
  while peek_char(&raw[length..])?.is_alphabetic() {
    length += 1;
  }

  let keyword = String::from_utf8_lossy(&raw[..length]);

  Ok((keyword, &raw[length..]))
}

pub fn parse_number<I: FromStr<Err = ParseIntError>>(mut raw: &[u8]) -> ParseResult<I> {
  ((), raw) = parse_whitespace(raw)?;

  let mut length = 0;
  while peek_char(&raw[length..])?.is_numeric() {
    length += 1;
  }

  let number = String::from_utf8_lossy(&raw[..length]).parse()?;

  Ok((number, &raw[length..]))
}

pub fn parse_name(mut raw: &[u8]) -> ParseResult<Cow<str>> {
  let is_name_char = |c: char| !DELIMETER_CHARACTERS.contains(c) && !c.is_whitespace();

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

  if contains_escapes {
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
    let name = String::from_utf8_lossy(&bytes).into_owned().into();
    Ok((name, &raw[length..]))
  } else {
    let name = String::from_utf8_lossy(&raw[..length]);
    Ok((name, &raw[length..]))
  }
}

pub fn parse_token(mut raw: &[u8]) -> ParseResult<Token> {
  ((), raw) = parse_whitespace(raw)?;

  let first_char = peek_char(raw)?;
  if first_char.is_alphanumeric() || "+-.".contains(first_char) {
    todo!("parse numeric")
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
      /The_Key_of_F#23_Minor/A#42";
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
  }
}

use crate::error::{Error, Result};
use std::borrow::Cow;
use std::num::ParseIntError;
use std::str::FromStr;

pub type ParseResult<'a, T> = Result<(T, &'a [u8])>;

#[inline]
fn peek_char(raw: &[u8]) -> Result<char> {
  String::from_utf8_lossy(&raw[..1])
    .chars()
    .next()
    .ok_or(Error::Syntax("Could not find next char"))
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

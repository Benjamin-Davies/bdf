use std::{io, num::ParseIntError, result, string::FromUtf8Error};
#[derive(Debug)]
pub enum Error {
  IO(io::Error),
  Syntax(&'static str),
  Encoding(FromUtf8Error),
  ParseInt(ParseIntError),
}

impl From<io::Error> for Error {
  fn from(err: io::Error) -> Self {
    Self::IO(err)
  }
}

impl From<FromUtf8Error> for Error {
  fn from(err: FromUtf8Error) -> Self {
    Self::Encoding(err)
  }
}

impl From<ParseIntError> for Error {
  fn from(err: ParseIntError) -> Self {
    Self::ParseInt(err)
  }
}

pub type Result<T> = result::Result<T, Error>;

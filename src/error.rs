use std::{io, num::ParseIntError, result};

#[derive(Debug)]
pub enum Error {
  IO(io::Error),
  Syntax(&'static str),
  ParseInt(ParseIntError),
}

impl From<io::Error> for Error {
  fn from(err: io::Error) -> Self {
    Self::IO(err)
  }
}

impl From<ParseIntError> for Error {
  fn from(err: ParseIntError) -> Self {
    Self::ParseInt(err)
  }
}

pub type Result<T> = result::Result<T, Error>;

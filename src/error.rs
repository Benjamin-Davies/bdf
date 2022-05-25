use crate::pdf_file::IndirectRef;
use std::{io, num::ParseIntError, result};

#[derive(Debug, PartialEq)]
pub enum Error {
  IO(String),
  NotLoaded(&'static str),
  ObjectNotFound(IndirectRef),
  ParseInt(ParseIntError),
  Syntax(&'static str),
}

impl From<io::Error> for Error {
  fn from(err: io::Error) -> Self {
    Self::IO(format!("{:?}", err))
  }
}

impl From<ParseIntError> for Error {
  fn from(err: ParseIntError) -> Self {
    Self::ParseInt(err)
  }
}

pub type Result<T> = result::Result<T, Error>;

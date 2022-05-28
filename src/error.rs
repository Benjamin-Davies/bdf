use crate::objects::IndirectRef;
use std::io;
use std::num::{ParseFloatError, ParseIntError};
use std::result;

#[derive(Debug, PartialEq)]
pub enum Error {
    EOF,
    IO(String),
    NotLoaded(&'static str),
    ObjectNotFound(IndirectRef),
    ParseFloat(ParseFloatError),
    ParseInt(ParseIntError),
    Syntax(&'static str),
}

impl From<io::Error> for Error {
    fn from(err: io::Error) -> Self {
        Self::IO(format!("{:?}", err))
    }
}

impl From<ParseFloatError> for Error {
    fn from(err: ParseFloatError) -> Self {
        Self::ParseFloat(err)
    }
}

impl From<ParseIntError> for Error {
    fn from(err: ParseIntError) -> Self {
        Self::ParseInt(err)
    }
}

pub type Result<T> = result::Result<T, Error>;

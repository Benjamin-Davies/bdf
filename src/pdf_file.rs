use std::{
  fs::File,
  io::{self, Read},
  path::Path,
  result,
  string::FromUtf8Error,
};

#[derive(Debug)]
pub enum Error {
  IO(io::Error),
  Syntax(&'static str),
  Encoding(FromUtf8Error),
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

pub type Result<T> = result::Result<T, Error>;

pub struct PdfFile {
  raw: Vec<u8>,
}

impl PdfFile {
  pub fn from_raw(raw: Vec<u8>) -> Self {
    Self { raw }
  }

  pub fn read_file<P: AsRef<Path>>(path: P) -> Result<Self> {
    let mut file = File::open(path)?;
    let mut buf = Vec::new();
    file.read_to_end(&mut buf)?;
    Ok(Self::from_raw(buf))
  }

  pub fn version(&self) -> Result<String> {
    if &self.raw[..5] != b"%PDF-" {
      return Err(Error::Syntax("Could not find pdf header"));
    }

    let end_index = match self.raw.iter().position(|&c| c == b'\n') {
      Some(i) => i,
      None => return Err(Error::Syntax("Could not find end of first line")),
    };

    let ver = String::from_utf8(self.raw[5..end_index].to_owned())?;

    Ok(ver)
  }
}

#[cfg(test)]
mod tests {
  use super::PdfFile;

  #[test]
  fn should_read_raw() {
    let file = PdfFile::read_file("./examples/hello-world.pdf").unwrap();
    assert_eq!(file.raw.len(), 13_200);
    assert_eq!(&file.raw[..9], b"%PDF-1.6\n");
  }

  #[test]
  fn should_detect_version() {
    let file = PdfFile::read_file("./examples/hello-world.pdf").unwrap();
    assert_eq!(&file.version().unwrap(), "1.6");
  }
}

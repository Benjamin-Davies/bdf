use crate::error::{Error, Result};
use crate::keywords::*;
use crate::slice_utils::last_position_of_sequence;
use std::{borrow::Cow, fs::File, io::Read, path::Path};

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

  pub fn version(&self) -> Result<Cow<str>> {
    if !self.raw.starts_with(PDF_HEADER) {
      return Err(Error::Syntax("Could not find pdf header"));
    }

    let end_index = self
      .raw
      .iter()
      .position(|&c| c == b'\n')
      .ok_or(Error::Syntax("Could not find end of first line"))?;

    let ver = String::from_utf8_lossy(&self.raw[PDF_HEADER.len()..end_index]);

    Ok(ver)
  }

  pub fn last_xref_offset(&self) -> Result<usize> {
    if !self.raw.ends_with(EOF_MARKER) {
      return Err(Error::Syntax("Could not find eof marker"));
    }

    let startxref_index = last_position_of_sequence(&self.raw, STARTXREF_KEYWORD)
      .ok_or(Error::Syntax("Could not find startxref keyword"))?;
    let value_index = startxref_index + STARTXREF_KEYWORD.len();

    let value = String::from_utf8_lossy(&self.raw[value_index..self.raw.len() - EOF_MARKER.len()]);
    Ok(value.trim().parse()?)
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

  #[test]
  fn should_find_last_xref_offset() {
    let file = PdfFile::read_file("./examples/hello-world.pdf").unwrap();
    assert_eq!(file.last_xref_offset().unwrap(), 12596);
  }
}

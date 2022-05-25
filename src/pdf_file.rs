use crate::error::{Error, Result};
use crate::keywords::*;
use crate::slice_utils::last_position_of_sequence;
use std::{borrow::Cow, collections::HashMap, fs::File, io::Read, path::Path};

#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq)]
pub struct IndirectRef {
  pub number: u32,
  pub generation: u16,
}

pub struct PdfFile {
  raw: Vec<u8>,
  xref_table: Option<HashMap<IndirectRef, Option<usize>>>,
}

impl PdfFile {
  pub fn from_raw(raw: Vec<u8>) -> Self {
    Self {
      raw,
      xref_table: None,
    }
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

  pub fn load_xref_table(&mut self) -> Result<()> {
    if self.xref_table.is_some() {
      return Ok(());
    }

    let xref_offset = self.last_xref_offset()?;
    if !self.raw[xref_offset..].starts_with(XREF_KEYWORD) {
      return Err(Error::Syntax("Could not find xref keyword"));
    }

    let intro_offset = xref_offset + XREF_KEYWORD.len() + 1;
    let length_offset = intro_offset
      + self.raw[intro_offset..]
        .iter()
        .position(|&c| c == b' ')
        .ok_or(Error::Syntax("Could not find space preceeding xref length"))?
      + 1;
    let content_offset = intro_offset
      + self.raw[intro_offset..]
        .iter()
        .position(|&c| c == b'\n')
        .ok_or(Error::Syntax(
          "Could not find newline preceeding xref content",
        ))?
      + 1;

    let first_object_number: u32 = String::from_utf8_lossy(&self.raw[intro_offset..length_offset])
      .trim()
      .parse()?;
    let length: u32 = String::from_utf8_lossy(&self.raw[length_offset..content_offset])
      .trim()
      .parse()?;

    let mut xref_table = HashMap::new();
    for i in 0..length {
      const LINE_LENGTH: usize = 20;
      let number = first_object_number + i;

      let line_offset = content_offset + LINE_LENGTH * i as usize;
      let line = &self.raw[line_offset..line_offset + LINE_LENGTH];

      let object_offset = String::from_utf8_lossy(&line[0..10]).trim().parse()?;
      let generation = String::from_utf8_lossy(&line[11..16]).trim().parse()?;
      let in_use = line[17] == b'n';
      xref_table.insert(
        IndirectRef { number, generation },
        if in_use { Some(object_offset) } else { None },
      );
    }

    self.xref_table = Some(xref_table);
    Ok(())
  }

  pub fn indirect_object_offset(&self, reference: IndirectRef) -> Result<usize> {
    let xref_table = self
      .xref_table
      .as_ref()
      .ok_or(Error::NotLoaded("xref_table"))?;

    xref_table
      .get(&reference)
      .ok_or(Error::ObjectNotFound(reference))?
      .ok_or(Error::ObjectNotFound(reference))
  }
}

#[cfg(test)]
mod tests {
  use super::*;

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

  #[test]
  fn should_locate_objects() {
    let mut file = PdfFile::read_file("./examples/hello-world.pdf").unwrap();
    file.load_xref_table().unwrap();
    // Redeclare file as immutable
    let file = file;

    let reference = IndirectRef {
      number: 0,
      generation: 0,
    };
    assert_eq!(
      file.indirect_object_offset(reference),
      Err(Error::ObjectNotFound(reference))
    );

    let reference = IndirectRef {
      number: 1,
      generation: 0,
    };
    assert_eq!(file.indirect_object_offset(reference), Ok(6608));

    let reference = IndirectRef {
      number: 19,
      generation: 0,
    };
    assert_eq!(file.indirect_object_offset(reference), Ok(12421));
  }
}

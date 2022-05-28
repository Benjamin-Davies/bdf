use crate::error::{Error, Result};
use crate::keywords::*;
use crate::objects::parse_object_until_keyword;
use crate::objects::IndirectRef;
use crate::objects::Object;
use crate::slice_utils::last_position_of_sequence;
use crate::tokens;
use std::{borrow::Cow, collections::HashMap, fs::File, io::Read, path::Path};

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
            return Err(Error::Syntax("Could not find pdf header", "".into()));
        }

        let end_index = self
            .raw
            .iter()
            .position(|&c| c == b'\n')
            .ok_or(Error::Syntax("Could not find end of first line", "".into()))?;

        let ver = String::from_utf8_lossy(&self.raw[PDF_HEADER.len()..end_index]);

        Ok(ver)
    }

    pub fn last_xref_offset(&self) -> Result<usize> {
        if !self.raw.ends_with(EOF_MARKER) {
            return Err(Error::Syntax("Could not find eof marker", "".into()));
        }

        let startxref_index = last_position_of_sequence(&self.raw, STARTXREF_KEYWORD)
            .ok_or(Error::Syntax("Could not find startxref keyword", "".into()))?;
        let raw = &self.raw[startxref_index..];

        let (startxref_keyword, raw) = tokens::parse_keyword(raw)?;
        if startxref_keyword != STARTXREF_KEYWORD {
            return Err(Error::Syntax("Could not read startxref keyword", "".into()));
        }

        let (last_xref_offset, _raw) = tokens::parse_number(raw)?;
        Ok(last_xref_offset)
    }

    pub fn load_xref_table(&mut self) -> Result<()> {
        if self.xref_table.is_some() {
            return Ok(());
        }

        let xref_offset = self.last_xref_offset()?;
        let raw = &self.raw[xref_offset..];

        let (xref_keyword, raw) = tokens::parse_keyword(raw)?;
        if xref_keyword != XREF_KEYWORD {
            return Err(Error::Syntax("Could not find xref keyword", "".into()));
        }

        let (first_object_number, raw) = tokens::parse_number::<u32>(raw)?;
        let (length, raw) = tokens::parse_number::<u32>(raw)?;
        let ((), raw) = tokens::parse_whitespace(raw)?;

        let mut xref_table = HashMap::new();
        for i in 0..length {
            const LINE_LENGTH: usize = 20;
            let number = first_object_number + i;

            let line_offset = LINE_LENGTH * i as usize;
            let line = &raw[line_offset..line_offset + LINE_LENGTH];

            let object_offset = String::from_utf8_lossy(&line[0..10]).parse()?;
            let generation = String::from_utf8_lossy(&line[11..16]).parse()?;
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

    pub fn trailer(&self) -> Result<Object> {
        let trailer_index = last_position_of_sequence(&self.raw, TRAILER_KEYWORD)
            .ok_or(Error::Syntax("Could not find trailer keyword", "".into()))?;
        let raw = &self.raw[trailer_index + TRAILER_KEYWORD.len()..];

        let (obj, _raw) = parse_object_until_keyword(raw, STARTXREF_KEYWORD)?;

        Ok(obj)
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

    #[test]
    fn should_parse_trailer() {
        let file = PdfFile::read_file("./examples/hello-world.pdf").unwrap();
        let trailer = file.trailer().unwrap();

        let mut expected = HashMap::<Cow<[u8]>, Object>::new();
        expected.insert(Cow::Borrowed(b"Size"), Object::Integer(20));
        expected.insert(
            Cow::Borrowed(b"Root"),
            Object::Indirect(IndirectRef {
                number: 18,
                generation: 0,
            }),
        );
        expected.insert(
            Cow::Borrowed(b"Info"),
            Object::Indirect(IndirectRef {
                number: 19,
                generation: 0,
            }),
        );
        expected.insert(
            Cow::Borrowed(b"ID"),
            Object::Array(vec![
                Object::String(Cow::Borrowed(&[
                    0x67, 0x2D, 0xFA, 0x4F, 0x7E, 0xF7, 0x2C, 0x74, 0x08, 0xCF, 0x44, 0xB9, 0x85,
                    0x04, 0x9C, 0x31,
                ])),
                Object::String(Cow::Borrowed(&[
                    0x67, 0x2D, 0xFA, 0x4F, 0x7E, 0xF7, 0x2C, 0x74, 0x08, 0xCF, 0x44, 0xB9, 0x85,
                    0x04, 0x9C, 0x31,
                ])),
            ]),
        );
        expected.insert(
            Cow::Borrowed(b"DocChecksum"),
            Object::Name(Cow::Borrowed(b"55569C181E425D18F7ED4931B469769A")),
        );

        assert_eq!(trailer, Object::Dictionary(expected));
    }
}

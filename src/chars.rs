use crate::error::{Error, Result};

/// Characters which are to be treated as whitespace (excluding comments)
/// (Adobe, 2008, p. 12).
pub const WHITESPACE_CHARACTERS: [u8; 6] = *b"\0\t\n\x0C\r ";

/// Characters which are to be treated as newlines (Adobe, 2008, p. 12).
pub const NEWLINE_CHARACTERS: [u8; 2] = *b"\n\r";

/// Characters which "delimit syntactic entities such as arrays, names, and
/// comments. Any of these characters terminates the entity preceding it and is
/// not included in the entity." (Adobe, 2008, p. 13)
pub const DELIMETER_CHARACTERS: [u8; 10] = *b"()<>[]{}/%";

/// Characters which may be part of a numeric object token.
pub const NUMERIC_CHARACTERS: [u8; 3] = *b"+-.";

/// Returns the next byte in the buffer.
///
/// If the buffer is empty, then returns `Err(Error::EOF)`.
#[inline]
pub fn peek_char(raw: &[u8]) -> Result<u8> {
  raw.iter().next().cloned().ok_or(Error::EOF)
}

/// Returns true if the character constututes whitespace.
#[inline]
pub fn is_whitespace_char(c: u8) -> bool {
  WHITESPACE_CHARACTERS.contains(&c)
}

/// Returns true if the character constututes a newline.
#[inline]
pub fn is_newline_char(c: u8) -> bool {
  NEWLINE_CHARACTERS.contains(&c)
}

/// Returns true if the character is from the roman alphabet.
#[inline]
pub fn is_alphabetic_char(c: u8) -> bool {
  (b'a' <= c && c <= b'z') || (b'A' <= c && c <= b'Z')
}

/// Returns true if the character is may be part of a name object token
/// (excluding the initial `/`)
#[inline]
pub fn is_name_char(c: u8) -> bool {
  !DELIMETER_CHARACTERS.contains(&c) && !is_whitespace_char(c)
}

/// Returns true if the character is may be part of a numeric object token
/// (0-9, +, -, .)
#[inline]
pub fn is_numeric_char(c: u8) -> bool {
  NUMERIC_CHARACTERS.contains(&c) || (b'0' <= c && c <= b'9')
}

#[cfg(test)]
mod tests {
  use super::*;
  use paste::paste;

  #[test]
  fn should_peek_char() {
    assert_eq!(peek_char(b"Hi"), Ok(b'H'));
    assert_eq!(peek_char(b"i"), Ok(b'i'));
    assert_eq!(peek_char(b""), Err(Error::EOF));
  }

  macro_rules! char_detection_test {
    ($type:ident, $should_match:literal) => {
      paste! {
        #[test]
        fn [<should_detect_ $type _char>]() {
          let matches: Vec<u8> = CHARS_TO_TEST
            .iter()
            .filter(|&&c| [<is_ $type _char>](c))
            .cloned()
            .collect();
          let matches = String::from_utf8_lossy(&matches);
          assert_eq!(matches, $should_match);
        }
      }
    };
  }

  const CHARS_TO_TEST: &[u8] = b"\0\t\n\x0C\r /Hi#20+-.";

  char_detection_test!(whitespace, "\0\t\n\x0C\r ");
  char_detection_test!(newline, "\n\r");
  char_detection_test!(alphabetic, "Hi");
  char_detection_test!(name, "Hi#20+-.");
  char_detection_test!(numeric, "20+-.");
}

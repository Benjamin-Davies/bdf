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

  #[test]
  fn should_peek_char() {
    assert_eq!(peek_char(b"Hi"), Ok(b'H'));
    assert_eq!(peek_char(b"i"), Ok(b'i'));
    assert_eq!(peek_char(b""), Err(Error::EOF));
  }

  #[test]
  fn should_detect_whitespace_char() {
    assert_eq!(is_whitespace_char(0), true);
    assert_eq!(is_whitespace_char(b'\t'), true);
    assert_eq!(is_whitespace_char(b'\n'), true);
    assert_eq!(is_whitespace_char(0x0C), true);
    assert_eq!(is_whitespace_char(b'\r'), true);
    assert_eq!(is_whitespace_char(b' '), true);
    assert_eq!(is_whitespace_char(b'/'), false);
    assert_eq!(is_whitespace_char(b'#'), false);
    assert_eq!(is_whitespace_char(b'H'), false);
    assert_eq!(is_whitespace_char(b'i'), false);
    assert_eq!(is_whitespace_char(b'2'), false);
    assert_eq!(is_whitespace_char(b'0'), false);
    assert_eq!(is_whitespace_char(b'+'), false);
    assert_eq!(is_whitespace_char(b'-'), false);
    assert_eq!(is_whitespace_char(b'.'), false);
  }

  #[test]
  fn should_detect_newline_char() {
    assert_eq!(is_newline_char(0), false);
    assert_eq!(is_newline_char(b'\t'), false);
    assert_eq!(is_newline_char(b'\n'), true);
    assert_eq!(is_newline_char(0x0C), false);
    assert_eq!(is_newline_char(b'\r'), true);
    assert_eq!(is_newline_char(b' '), false);
    assert_eq!(is_newline_char(b'/'), false);
    assert_eq!(is_newline_char(b'#'), false);
    assert_eq!(is_newline_char(b'H'), false);
    assert_eq!(is_newline_char(b'i'), false);
    assert_eq!(is_newline_char(b'2'), false);
    assert_eq!(is_newline_char(b'0'), false);
    assert_eq!(is_newline_char(b'+'), false);
    assert_eq!(is_newline_char(b'-'), false);
    assert_eq!(is_newline_char(b'.'), false);
  }

  #[test]
  fn should_detect_alphabetic_char() {
    assert_eq!(is_alphabetic_char(0), false);
    assert_eq!(is_alphabetic_char(b'\t'), false);
    assert_eq!(is_alphabetic_char(b'\n'), false);
    assert_eq!(is_alphabetic_char(0x0C), false);
    assert_eq!(is_alphabetic_char(b'\r'), false);
    assert_eq!(is_alphabetic_char(b' '), false);
    assert_eq!(is_alphabetic_char(b'/'), false);
    assert_eq!(is_alphabetic_char(b'#'), false);
    assert_eq!(is_alphabetic_char(b'H'), true);
    assert_eq!(is_alphabetic_char(b'i'), true);
    assert_eq!(is_alphabetic_char(b'2'), false);
    assert_eq!(is_alphabetic_char(b'0'), false);
    assert_eq!(is_alphabetic_char(b'+'), false);
    assert_eq!(is_alphabetic_char(b'-'), false);
    assert_eq!(is_alphabetic_char(b'.'), false);
  }

  #[test]
  fn should_detect_name_char() {
    assert_eq!(is_name_char(0), false);
    assert_eq!(is_name_char(b'\t'), false);
    assert_eq!(is_name_char(b'\n'), false);
    assert_eq!(is_name_char(0x0C), false);
    assert_eq!(is_name_char(b'\r'), false);
    assert_eq!(is_name_char(b' '), false);
    assert_eq!(is_name_char(b'/'), false);
    assert_eq!(is_name_char(b'#'), true);
    assert_eq!(is_name_char(b'H'), true);
    assert_eq!(is_name_char(b'i'), true);
    assert_eq!(is_name_char(b'2'), true);
    assert_eq!(is_name_char(b'0'), true);
    assert_eq!(is_name_char(b'+'), true);
    assert_eq!(is_name_char(b'-'), true);
    assert_eq!(is_name_char(b'.'), true);
  }

  #[test]
  fn should_detect_numeric_char() {
    assert_eq!(is_numeric_char(0), false);
    assert_eq!(is_numeric_char(b'\t'), false);
    assert_eq!(is_numeric_char(b'\n'), false);
    assert_eq!(is_numeric_char(0x0C), false);
    assert_eq!(is_numeric_char(b'\r'), false);
    assert_eq!(is_numeric_char(b' '), false);
    assert_eq!(is_numeric_char(b'/'), false);
    assert_eq!(is_numeric_char(b'#'), false);
    assert_eq!(is_numeric_char(b'H'), false);
    assert_eq!(is_numeric_char(b'i'), false);
    assert_eq!(is_numeric_char(b'2'), true);
    assert_eq!(is_numeric_char(b'0'), true);
    assert_eq!(is_numeric_char(b'+'), true);
    assert_eq!(is_numeric_char(b'-'), true);
    assert_eq!(is_numeric_char(b'.'), true);
  }
}

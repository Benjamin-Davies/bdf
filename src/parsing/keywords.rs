pub const PDF_HEADER: &[u8] = b"%PDF-";
pub const EOF_MARKER: &[u8] = b"%%EOF\n";

pub const STARTXREF_KEYWORD: &[u8] = b"startxref";
pub const XREF_KEYWORD: &[u8] = b"xref";
pub const STREAM_KEYWORD: &[u8] = b"stream";
pub const ENDSTREAM_KEYWORD: &[u8] = b"endstream";
pub const TRAILER_KEYWORD: &[u8] = b"trailer";
pub const OBJ_KEYWORD: &[u8] = b"obj";
pub const ENDOBJ_KEYWORD: &[u8] = b"endobj";

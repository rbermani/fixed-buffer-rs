#![forbid(unsafe_code)]

/// Convert a byte slice into a string.
/// Includes printable ASCII characters as-is.
/// Converts non-printable or non-ASCII characters to strings like "\n" and "\x19".
///
/// Uses
/// [`core::ascii::escape_default`](https://doc.rust-lang.org/core/ascii/fn.escape_default.html)
/// internally to escape each byte.
///
/// This function is useful for printing byte slices to logs and comparing byte slices in tests.
///
/// Example test:
/// ```
/// use fixed_buffer::{escape_ascii, FixedBuf};
/// let mut buf: FixedBuf<[u8; 16]> = FixedBuf::new([0u8; 16]);
/// buf.write_str("ab");
/// buf.write_str("cd");
/// assert_eq!("abcd", escape_ascii(buf.readable()));
/// ```
pub fn escape_ascii(input: &[u8]) -> String {
    let mut result = String::new();
    for byte in input {
        for ascii_byte in core::ascii::escape_default(*byte) {
            result.push_str(core::str::from_utf8(&[ascii_byte]).unwrap());
        }
    }
    result
}

#[cfg(test)]
mod tests {
    use super::super::*;

    #[test]
    fn test_escape_ascii() {
        assert_eq!("", FixedBuf::filled(b"").escape_ascii());
        assert_eq!("abc", FixedBuf::filled(b"abc").escape_ascii());
        assert_eq!("\\r\\n", FixedBuf::filled(b"\r\n").escape_ascii());
        assert_eq!(
            "\\xe2\\x82\\xac",
            FixedBuf::filled("€".as_bytes()).escape_ascii()
        );
        assert_eq!("\\x01", FixedBuf::filled(b"\x01").escape_ascii());
        let buf = FixedBuf::filled(b"abc");
        assert_eq!("abc", buf.escape_ascii());
        assert_eq!(b"abc", buf.readable());
    }
}

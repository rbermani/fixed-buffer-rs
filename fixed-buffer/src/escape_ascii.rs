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
/// use fixed_buffer::escape_ascii;
/// assert_eq!("abc", escape_ascii(b"abc"));
/// assert_eq!("abc\\n", escape_ascii(b"abc\n"));
/// assert_eq!(
///     "Euro sign: \\xe2\\x82\\xac",
///     escape_ascii("Euro sign: \u{20AC}".as_bytes())
/// );
/// assert_eq!("\\x01\\x02\\x03", escape_ascii(&[1, 2, 3]));
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
#[test]
fn test_escape_ascii() {
    assert_eq!("", escape_ascii(b""));
    assert_eq!("abc", escape_ascii(b"abc"));
    assert_eq!("\\r\\n", escape_ascii(b"\r\n"));
    assert_eq!(
        "\\xe2\\x82\\xac",
        escape_ascii(/* Euro sign */ "\u{20AC}".as_bytes())
    );
    assert_eq!("\\x01", escape_ascii(b"\x01"));
}

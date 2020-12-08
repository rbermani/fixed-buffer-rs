#![forbid(unsafe_code)]

use super::MalformedInputError;

/// Deframes lines that are terminated by `b"\r\n"`.
/// Ignores solitary `b'\n'`.
///
/// See [`read_frame`](struct.FixedBuf.html#method.read_frame).
pub fn deframe_crlf(
    data: &[u8],
) -> Result<Option<(core::ops::Range<usize>, usize)>, MalformedInputError> {
    if data.len() > 1 {
        for n in 1..data.len() {
            if data[n - 1] == b'\r' && data[n] == b'\n' {
                return Ok(Some((0..(n - 1), n + 1)));
            }
        }
    }
    Ok(None)
}

#[cfg(test)]
mod tests {
    use super::super::*;

    #[test]
    fn test_dataframe_crlf() {
        assert_eq!(None, deframe_crlf(b"").unwrap());
        assert_eq!(None, deframe_crlf(b"abc").unwrap());
        assert_eq!(None, deframe_crlf(b"abc\r").unwrap());
        assert_eq!(None, deframe_crlf(b"abc\n").unwrap());
        assert_eq!(Some((0..0, 2)), deframe_crlf(b"\r\n").unwrap());
        assert_eq!(Some((0..3, 5)), deframe_crlf(b"abc\r\n").unwrap());
        assert_eq!(Some((0..3, 5)), deframe_crlf(b"abc\r\ndef").unwrap());
        assert_eq!(Some((0..3, 5)), deframe_crlf(b"abc\r\ndef\r\n").unwrap());
        assert_eq!(Some((0..4, 6)), deframe_crlf(b"abc\n\r\n").unwrap());
    }
}

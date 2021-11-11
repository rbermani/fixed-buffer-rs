#![forbid(unsafe_code)]

use super::MalformedInputError;

/// Deframes lines that are terminated by `b"\0"`.
/// Ignores `b'\n'` or `b"\r"` and `b"\r\n"`.
///
/// See [`read_frame`](struct.FixedBuf.html#method.read_frame).
pub fn deframe_null(
    data: &[u8],
) -> Result<Option<(core::ops::Range<usize>, usize)>, MalformedInputError> {
    for n in 0..data.len() {
        if data[n] == b'\0' {
            return Ok(Some((0..n, n + 1)));
        }
    }
    Ok(None)
}

#[cfg(test)]
mod tests {
    use super::super::*;

    #[test]
    fn test_dataframe_null() {
        assert_eq!(None, deframe_null(b"").unwrap());
        assert_eq!(None, deframe_null(b"abc").unwrap());
        assert_eq!(None, deframe_null(b"abc\r").unwrap());
        assert_eq!(None, deframe_null(b"abc\r\n").unwrap());
        assert_eq!(None, deframe_null(b"abc\n").unwrap());
        assert_eq!(Some((0..0, 1)), deframe_null(b"\0").unwrap());
        assert_eq!(Some((0..3, 4)), deframe_null(b"abc\0").unwrap());
        assert_eq!(Some((0..3, 4)), deframe_null(b"abc\0def").unwrap());
        assert_eq!(Some((0..3, 4)), deframe_null(b"abc\0def\0").unwrap());
        assert_eq!(Some((0..4, 5)), deframe_null(b"abc\r\0").unwrap());
        assert_eq!(Some((0..5, 6)), deframe_null(b"abc\r\n\0").unwrap());
    }
}

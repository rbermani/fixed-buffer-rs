#![forbid(unsafe_code)]

use super::MalformedInputError;

/// Deframes lines that are terminated by either `b'\n'` or `b"\r\n"`.
///
/// See [`read_frame`](struct.FixedBuf.html#method.read_frame).
pub fn deframe_line(
    data: &[u8],
) -> Result<Option<(core::ops::Range<usize>, usize)>, MalformedInputError> {
    for n in 0..data.len() {
        if data[n] == b'\n' {
            let data_start_incl = 0;
            let data_end_excl = if n > 0 && data[n - 1] == b'\r' {
                n - 1
            } else {
                n
            };
            let block_len = n + 1;
            return Ok(Some((data_start_incl..data_end_excl, block_len)));
        }
    }
    Ok(None)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dataframe_crlf() {
        assert_eq!(None, deframe_line(b"").unwrap());
        assert_eq!(None, deframe_line(b"abc").unwrap());
        assert_eq!(None, deframe_line(b"abc\r").unwrap());
        assert_eq!(Some((0..0, 1)), deframe_line(b"\n").unwrap());
        assert_eq!(Some((0..0, 2)), deframe_line(b"\r\n").unwrap());
        assert_eq!(Some((0..3, 4)), deframe_line(b"abc\n").unwrap());
        assert_eq!(Some((0..3, 5)), deframe_line(b"abc\r\n").unwrap());
        assert_eq!(Some((0..3, 4)), deframe_line(b"abc\ndef").unwrap());
        assert_eq!(Some((0..3, 5)), deframe_line(b"abc\r\ndef").unwrap());
        assert_eq!(Some((0..3, 4)), deframe_line(b"abc\ndef\n").unwrap());
        assert_eq!(Some((0..3, 5)), deframe_line(b"abc\r\ndef\r\n").unwrap());
        assert_eq!(Some((0..4, 6)), deframe_line(b"abc\r\r\n").unwrap());
    }
}

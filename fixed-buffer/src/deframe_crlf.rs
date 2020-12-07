#![forbid(unsafe_code)]

use super::MalformedInputError;

// TODO(mleonhard) Test.
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

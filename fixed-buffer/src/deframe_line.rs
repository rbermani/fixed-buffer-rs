#![forbid(unsafe_code)]

use super::MalformedInputError;

// TODO(mleonhard) Test.
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

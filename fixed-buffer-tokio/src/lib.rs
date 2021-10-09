//! [![crates.io version](https://img.shields.io/crates/v/fixed-buffer-tokio.svg)](https://crates.io/crates/fixed-buffer-tokio)
//! [![license: Apache 2.0](https://gitlab.com/leonhard-llc/fixed-buffer-rs/-/raw/main/license-apache-2.0.svg)](http://www.apache.org/licenses/LICENSE-2.0)
//! [![unsafe forbidden](https://gitlab.com/leonhard-llc/fixed-buffer-rs/-/raw/main/unsafe-forbidden-success.svg)](https://github.com/rust-secure-code/safety-dance/)
//! [![pipeline status](https://gitlab.com/leonhard-llc/fixed-buffer-rs/badges/main/pipeline.svg)](https://gitlab.com/leonhard-llc/fixed-buffer-rs/-/pipelines)
//!
//! This is a Rust library with fixed-size buffers,
//! useful for network protocol parsers and file parsers.
//!
//! This is the tokio async version of [`fixed-buffer`](https://crates.io/crates/fixed-buffer).
//!
//! # Features
//! - Write bytes to the buffer and read them back
//! - Does not allocate memory
//! - Use it to read a stream, search for a delimiter,
//!   and save leftover bytes for the next read.
//! - Depends only on
//!   [`std`](https://doc.rust-lang.org/stable/std/),
//!   [`tokio`](https://crates.io/crates/tokio), and
//!   [`fixed-buffer`](https://crates.io/crates/fixed-buffer).
//! - Works with Tokio 1 and Rust `latest`, `beta`, and `nightly`
//! - No macros
//! - Good test coverage (98%)
//! - `forbid(unsafe_code)`
//!
//! # Documentation
//! https://docs.rs/fixed-buffer-tokio
//!
//! # Examples
//! For a complete example, see
//! [`tests/server.rs`](https://gitlab.com/leonhard-llc/fixed-buffer-rs/-/blob/main/fixed-buffer-tokio/tests/server.rs).
//!
//! # Alternatives
//! - [tokio::io::BufReader](https://docs.rs/tokio/latest/tokio/io/struct.BufReader.html)
//! - [tokio::io::BufWriter](https://docs.rs/tokio/latest/tokio/io/struct.BufWriter.html)
//!
//! # Changelog
//! - v0.3.2 - Update docs
//! - v0.3.1 - Support Tokio 1
//! - v0.3.0 - Breaking API changes:
//!   - Change type parameter to const buffer size. Example: `FixedBuf<1024>`.
//!   - Remove `new` arg.
//!   - Remove `capacity`.
//!   - Change `writable` return type to `&mut [u8]`.
//! - v0.1.1 - Add badges to readme
//! - v0.1.0 - First published version
//!
//! # Release Process
//! 1. Edit `Cargo.toml` and bump version number.
//! 1. Run `../release.sh`
#![forbid(unsafe_code)]

use core::pin::Pin;
use core::task::{Context, Poll};
use fixed_buffer::{FixedBuf, MalformedInputError};

mod async_read_write_chain;
pub use async_read_write_chain::*;

mod async_read_write_take;
pub use async_read_write_take::*;

#[cfg(test)]
mod test_utils;
#[cfg(test)]
pub use test_utils::*;

/// A newtype that wraps
/// [`FixedBuf`](https://docs.rs/fixed-buffer/latest/fixed_buffer/struct.FixedBuf.html)
/// and implements
/// [`tokio::io::AsyncRead`](https://docs.rs/tokio/latest/tokio/io/trait.AsyncRead.html)
/// and
/// [`tokio::io::AsyncWrite`](https://docs.rs/tokio/latest/tokio/io/trait.AsyncWrite.html).
///
/// It also has async versions of FixedBuf's io functions.
pub struct AsyncFixedBuf<const SIZE: usize>(FixedBuf<SIZE>);

impl<const SIZE: usize> AsyncFixedBuf<SIZE> {
    /// Creates a new FixedBuf and wraps it in an AsyncFixedBuf.
    ///
    /// See
    /// [`FixedBuf::new`](https://docs.rs/fixed-buffer/latest/fixed_buffer/struct.FixedBuf.html#method.new)
    /// for details.
    pub const fn new() -> Self {
        AsyncFixedBuf(FixedBuf::new())
    }

    /// Drops the struct and returns its internal
    /// [`FixedBuf`](https://docs.rs/fixed-buffer/latest/fixed_buffer/struct.FixedBuf.html).
    pub fn into_inner(self) -> FixedBuf<SIZE> {
        self.0
    }

    /// Makes a new empty buffer.
    ///
    /// Consumes `mem` and uses it as the internal memory array.
    /// ```
    pub fn empty(mem: [u8; SIZE]) -> Self {
        Self(FixedBuf::empty(mem))
    }

    /// Makes a new full buffer containing the bytes in `mem`.
    /// Reading the buffer will return the bytes in `mem`.
    ///
    /// Consumes `mem` and uses it as the internal memory array.
    /// ```
    pub fn filled(mem: [u8; SIZE]) -> Self {
        Self(FixedBuf::filled(mem))
    }

    /// Reads from `reader` once and writes the data into the buffer.
    ///
    /// Returns [`InvalidData`](std::io::ErrorKind::InvalidData)
    /// if there is no empty space in the buffer.
    /// See [`shift`](#method.shift).
    pub async fn copy_once_from<R: tokio::io::AsyncRead + std::marker::Unpin + Send>(
        &mut self,
        reader: &mut R,
    ) -> Result<usize, std::io::Error> {
        let mut writable = self.writable();
        if writable.is_empty() {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "no empty space in buffer",
            ));
        };
        let num_read = tokio::io::AsyncReadExt::read(reader, &mut writable).await?;
        self.wrote(num_read);
        Ok(num_read)
    }

    /// Async version of
    /// [`FixedBuf::read_frame`](https://docs.rs/fixed-buffer/latest/fixed_buffer/struct.FixedBuf.html#method.read_frame).
    pub async fn read_frame<R, F>(
        &mut self,
        reader: &mut R,
        deframer_fn: F,
    ) -> Result<Option<&[u8]>, std::io::Error>
    where
        R: tokio::io::AsyncRead + std::marker::Unpin + Send,
        F: Fn(&[u8]) -> Result<Option<(core::ops::Range<usize>, usize)>, MalformedInputError>,
    {
        loop {
            if !self.is_empty() {
                if let Some(frame_range) = self.deframe(&deframer_fn)? {
                    return Ok(Some(&self.mem()[frame_range]));
                }
                // None case falls through.
            }
            self.shift();
            let writable = self.writable();
            if writable.is_empty() {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::InvalidData,
                    "end of buffer full",
                ));
            };
            let num_read = tokio::io::AsyncReadExt::read(reader, writable).await?;
            if num_read == 0 {
                if self.is_empty() {
                    return Ok(None);
                }
                return Err(std::io::Error::new(
                    std::io::ErrorKind::UnexpectedEof,
                    "eof after reading part of a frame",
                ));
            }
            self.wrote(num_read);
        }
    }
}

impl<const SIZE: usize> Unpin for AsyncFixedBuf<SIZE> {}

impl<const SIZE: usize> std::ops::Deref for AsyncFixedBuf<SIZE> {
    type Target = FixedBuf<SIZE>;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<const SIZE: usize> std::ops::DerefMut for AsyncFixedBuf<SIZE> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl<const SIZE: usize> tokio::io::AsyncRead for AsyncFixedBuf<SIZE> {
    fn poll_read(
        self: Pin<&mut Self>,
        _cx: &mut Context<'_>,
        buf: &mut tokio::io::ReadBuf<'_>,
    ) -> Poll<Result<(), std::io::Error>> {
        let num_read = self
            .get_mut()
            .0
            .read_and_copy_bytes(buf.initialize_unfilled());
        buf.advance(num_read);
        Poll::Ready(Ok(()))
    }
}

impl<const SIZE: usize> tokio::io::AsyncWrite for AsyncFixedBuf<SIZE> {
    fn poll_write(
        self: Pin<&mut Self>,
        _cx: &mut Context<'_>,
        buf: &[u8],
    ) -> Poll<Result<usize, std::io::Error>> {
        Poll::Ready(self.get_mut().0.write_bytes(buf).map_err(|_| {
            std::io::Error::new(std::io::ErrorKind::InvalidData, "no space in buffer")
        }))
    }

    fn poll_flush(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<Result<(), std::io::Error>> {
        Poll::Ready(Ok(()))
    }

    fn poll_shutdown(
        self: Pin<&mut Self>,
        _cx: &mut Context<'_>,
    ) -> Poll<Result<(), std::io::Error>> {
        Poll::Ready(Ok(()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use fixed_buffer::*;

    fn deframe_line_reject_xs(
        data: &[u8],
    ) -> Result<Option<(core::ops::Range<usize>, usize)>, MalformedInputError> {
        if data.contains(&b'x') || data.contains(&b'X') {
            return Err(MalformedInputError::new(String::from("err1")));
        }
        deframe_line(data)
    }

    #[tokio::test]
    async fn test_read_frame_empty_to_eof() {
        let mut buf: AsyncFixedBuf<8> = AsyncFixedBuf::new();
        let mut reader = std::io::Cursor::new(b"");
        assert_eq!(
            None,
            buf.read_frame(&mut reader, deframe_line_reject_xs)
                .await
                .unwrap()
        );
        assert_eq!("", escape_ascii(buf.readable()));
    }

    #[tokio::test]
    async fn test_read_frame_empty_to_incomplete() {
        let mut buf: AsyncFixedBuf<8> = AsyncFixedBuf::new();
        let mut reader = std::io::Cursor::new(b"abc");
        assert_eq!(
            std::io::ErrorKind::UnexpectedEof,
            buf.read_frame(&mut reader, deframe_line_reject_xs)
                .await
                .unwrap_err()
                .kind()
        );
        assert_eq!("abc", escape_ascii(buf.readable()));
    }

    #[tokio::test]
    async fn test_read_frame_empty_to_complete() {
        let mut buf: AsyncFixedBuf<8> = AsyncFixedBuf::new();
        let mut reader = std::io::Cursor::new(b"abc\n");
        assert_eq!(
            "abc",
            escape_ascii(
                buf.read_frame(&mut reader, deframe_line_reject_xs)
                    .await
                    .unwrap()
                    .unwrap()
            )
        );
        assert_eq!("", escape_ascii(buf.readable()));
    }

    #[tokio::test]
    async fn test_read_frame_empty_to_complete_with_leftover() {
        let mut buf: AsyncFixedBuf<8> = AsyncFixedBuf::new();
        let mut reader = std::io::Cursor::new(b"abc\nde");
        assert_eq!(
            "abc",
            escape_ascii(
                buf.read_frame(&mut reader, deframe_line_reject_xs)
                    .await
                    .unwrap()
                    .unwrap()
            )
        );
        assert_eq!("de", escape_ascii(buf.readable()));
    }

    #[tokio::test]
    async fn test_read_frame_empty_to_invalid() {
        let mut buf: AsyncFixedBuf<8> = AsyncFixedBuf::new();
        let mut reader = std::io::Cursor::new(b"x");
        assert_eq!(
            std::io::ErrorKind::InvalidData,
            buf.read_frame(&mut reader, deframe_line_reject_xs)
                .await
                .unwrap_err()
                .kind()
        );
        assert_eq!("x", escape_ascii(buf.readable()));
    }

    #[tokio::test]
    async fn test_read_frame_incomplete_to_eof() {
        let mut buf: AsyncFixedBuf<8> = AsyncFixedBuf::new();
        buf.write_str("a").unwrap();
        let mut reader = std::io::Cursor::new(b"");
        assert_eq!(
            std::io::ErrorKind::UnexpectedEof,
            buf.read_frame(&mut reader, deframe_line_reject_xs)
                .await
                .unwrap_err()
                .kind()
        );
        assert_eq!("a", escape_ascii(buf.readable()));
    }

    #[tokio::test]
    async fn test_read_frame_incomplete_to_incomplete() {
        let mut buf: AsyncFixedBuf<8> = AsyncFixedBuf::new();
        buf.write_str("a").unwrap();
        let mut reader = std::io::Cursor::new(b"bc");
        assert_eq!(
            std::io::ErrorKind::UnexpectedEof,
            buf.read_frame(&mut reader, deframe_line_reject_xs)
                .await
                .unwrap_err()
                .kind()
        );
        assert_eq!("abc", escape_ascii(buf.readable()));
    }

    #[tokio::test]
    async fn test_read_frame_incomplete_to_complete() {
        let mut buf: AsyncFixedBuf<8> = AsyncFixedBuf::new();
        buf.write_str("a").unwrap();
        let mut reader = std::io::Cursor::new(b"bc\n");
        assert_eq!(
            "abc",
            escape_ascii(
                buf.read_frame(&mut reader, deframe_line_reject_xs)
                    .await
                    .unwrap()
                    .unwrap()
            )
        );
        assert_eq!("", escape_ascii(buf.readable()));
    }

    #[tokio::test]
    async fn test_read_frame_incomplete_to_complete_with_leftover() {
        let mut buf: AsyncFixedBuf<8> = AsyncFixedBuf::new();
        buf.write_str("a").unwrap();
        let mut reader = std::io::Cursor::new(b"bc\nde");
        assert_eq!(
            "abc",
            escape_ascii(
                buf.read_frame(&mut reader, deframe_line_reject_xs)
                    .await
                    .unwrap()
                    .unwrap()
            )
        );
        assert_eq!("de", escape_ascii(buf.readable()));
    }

    #[tokio::test]
    async fn test_read_frame_complete_doesnt_read() {
        let mut buf: AsyncFixedBuf<8> = AsyncFixedBuf::new();
        buf.write_str("abc\n").unwrap();
        assert_eq!(
            "abc",
            escape_ascii(
                buf.read_frame(&mut FakeAsyncReadWriter::empty(), deframe_line_reject_xs)
                    .await
                    .unwrap()
                    .unwrap()
            )
        );
        assert_eq!("", escape_ascii(buf.readable()));
    }

    #[tokio::test]
    async fn test_read_frame_complete_leaves_leftovers() {
        let mut buf: AsyncFixedBuf<8> = AsyncFixedBuf::new();
        buf.write_str("abc\nde").unwrap();
        assert_eq!(
            "abc",
            escape_ascii(
                buf.read_frame(&mut FakeAsyncReadWriter::empty(), deframe_line_reject_xs)
                    .await
                    .unwrap()
                    .unwrap()
            )
        );
        assert_eq!("de", escape_ascii(buf.readable()));
    }

    #[tokio::test]
    async fn test_read_frame_invalid_doesnt_read() {
        let mut buf: AsyncFixedBuf<8> = AsyncFixedBuf::new();
        buf.write_str("x").unwrap();
        assert_eq!(
            std::io::ErrorKind::InvalidData,
            buf.read_frame(&mut FakeAsyncReadWriter::empty(), deframe_line_reject_xs)
                .await
                .unwrap_err()
                .kind()
        );
        assert_eq!("x", escape_ascii(buf.readable()));
    }

    #[tokio::test]
    async fn test_read_frame_buffer_full() {
        let mut buf: AsyncFixedBuf<8> = AsyncFixedBuf::new();
        buf.write_str("abcdefgh").unwrap();
        let mut reader = std::io::Cursor::new(b"bc\nde");
        assert_eq!(
            std::io::ErrorKind::InvalidData,
            buf.read_frame(&mut reader, deframe_line_reject_xs)
                .await
                .unwrap_err()
                .kind()
        );
        assert_eq!("abcdefgh", escape_ascii(buf.readable()));
    }

    #[tokio::test]
    async fn test_async_read() {
        let mut buf: AsyncFixedBuf<16> = AsyncFixedBuf::new();
        let mut data = ['.' as u8; 16];
        assert_eq!(
            0,
            tokio::io::AsyncReadExt::read(&mut buf, &mut data)
                .await
                .unwrap()
        );
        assert_eq!("..........", escape_ascii(&data[..10]));
        buf.write_str("abc").unwrap();
        assert_eq!(
            3,
            tokio::io::AsyncReadExt::read(&mut buf, &mut data)
                .await
                .unwrap()
        );
        assert_eq!("abc.......", escape_ascii(&data[..10]));
        assert_eq!(
            0,
            tokio::io::AsyncReadExt::read(&mut buf, &mut data)
                .await
                .unwrap()
        );
        let many_bs = "b".repeat(16);
        buf.write_str(&many_bs).unwrap();
        assert_eq!(
            16,
            tokio::io::AsyncReadExt::read(&mut buf, &mut data)
                .await
                .unwrap()
        );
        assert_eq!(many_bs, escape_ascii(&data[..]));
        assert_eq!(
            0,
            tokio::io::AsyncReadExt::read(&mut buf, &mut data)
                .await
                .unwrap()
        );
    }

    #[tokio::test]
    async fn test_async_write() {
        let mut buf: AsyncFixedBuf<16> = AsyncFixedBuf::new();
        tokio::io::AsyncWriteExt::write_all(&mut buf, b"abc")
            .await
            .unwrap();
        assert_eq!("abc", escape_ascii(buf.readable()));
        tokio::io::AsyncWriteExt::write_all(&mut buf, b"def")
            .await
            .unwrap();
        assert_eq!("abcdef", escape_ascii(buf.readable()));
        buf.read_bytes(1);
        tokio::io::AsyncWriteExt::write_all(&mut buf, b"g")
            .await
            .unwrap();
        assert_eq!("bcdefg", escape_ascii(buf.readable()));
        tokio::io::AsyncWriteExt::write_all(&mut buf, "h".repeat(8).as_bytes())
            .await
            .unwrap();
        tokio::io::AsyncWriteExt::write_all(&mut buf, b"i")
            .await
            .unwrap();
        assert_eq!(
            std::io::ErrorKind::InvalidData,
            tokio::io::AsyncWriteExt::write_all(&mut buf, b"def")
                .await
                .unwrap_err()
                .kind()
        );
    }
}

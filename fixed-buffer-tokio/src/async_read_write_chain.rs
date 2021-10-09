#![forbid(unsafe_code)]

use std::task::Context;
use tokio::io::{AsyncRead, AsyncWrite, ReadBuf};
use tokio::macros::support::{Pin, Poll};

/// A wrapper for a pair of structs.
/// The first implements [`AsyncRead`](https://docs.rs/tokio/latest/tokio/io/trait.AsyncRead.html).
/// The second implements
/// [`AsyncRead`](https://docs.rs/tokio/latest/tokio/io/trait.AsyncRead.html)+[`AsyncWrite`](https://docs.rs/tokio/latest/tokio/io/trait.AsyncWrite.html).
///
/// Passes reads through to the AsyncRead.
/// Once the AsyncRead returns EOF, passes reads to AsyncRead+AsyncWrite.
///
/// Passes all writes through to the AsyncRead+AsyncWrite.
///
/// This is like [`tokio::io::AsyncReadExt::chain`](https://docs.rs/tokio/latest/tokio/io/trait.AsyncReadExt.html#method.chain)
/// that passes through writes.
/// This makes it usable with AsyncRead+AsyncWrite objects like
/// [`tokio::net::TcpStream`](https://docs.rs/tokio/latest/tokio/net/struct.TcpStream.html)
/// and
/// [`tokio_rustls::server::TlsStream`](https://docs.rs/tokio-rustls/latest/tokio_rustls/server/struct.TlsStream.html).
pub struct AsyncReadWriteChain<
    'a,
    R: tokio::io::AsyncRead + Send + Unpin,
    RW: tokio::io::AsyncRead + tokio::io::AsyncWrite + Send + Unpin,
> {
    reader: Option<&'a mut R>,
    read_writer: &'a mut RW,
}

impl<'a, R: AsyncRead + Send + Unpin, RW: AsyncRead + AsyncWrite + Send + Unpin>
    AsyncReadWriteChain<'a, R, RW>
{
    /// See [`AsyncReadWriteChain`](struct.AsyncReadWriteChain.html).
    pub fn new(reader: &'a mut R, read_writer: &'a mut RW) -> AsyncReadWriteChain<'a, R, RW> {
        Self {
            reader: Some(reader),
            read_writer,
        }
    }
}

impl<'a, R: AsyncRead + Send + Unpin, RW: AsyncRead + AsyncWrite + Send + Unpin> AsyncRead
    for AsyncReadWriteChain<'a, R, RW>
{
    fn poll_read(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut ReadBuf<'_>,
    ) -> Poll<Result<(), std::io::Error>> {
        let mut_self = self.get_mut();
        if let Some(ref mut reader) = mut_self.reader {
            let before_len = buf.filled().len();
            match Pin::new(&mut *reader).poll_read(cx, buf) {
                Poll::Pending => return Poll::Pending,
                Poll::Ready(Err(e)) => return Poll::Ready(Err(e)),
                Poll::Ready(Ok(())) => {
                    let num_read = buf.filled().len() - before_len;
                    if num_read > 0 {
                        return Poll::Ready(Ok(()));
                    } else {
                        // EOF
                        mut_self.reader = None;
                        // Fall through.
                    }
                }
            }
        }
        Pin::new(&mut mut_self.read_writer).poll_read(cx, buf)
    }
}

impl<'a, R: AsyncRead + Send + Unpin, RW: AsyncRead + AsyncWrite + Send + Unpin> AsyncWrite
    for AsyncReadWriteChain<'a, R, RW>
{
    fn poll_write(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &[u8],
    ) -> Poll<Result<usize, std::io::Error>> {
        let mut_self = self.get_mut();
        Pin::new(&mut mut_self.read_writer).poll_write(cx, buf)
    }

    fn poll_flush(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), std::io::Error>> {
        let mut_self = self.get_mut();
        Pin::new(&mut mut_self.read_writer).poll_flush(cx)
    }

    fn poll_shutdown(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
    ) -> Poll<Result<(), std::io::Error>> {
        let mut_self = self.get_mut();
        Pin::new(&mut mut_self.read_writer).poll_shutdown(cx)
    }
}

#[cfg(test)]
mod tests {
    use super::super::*;
    use fixed_buffer::escape_ascii;

    #[tokio::test]
    async fn both_empty() {
        let mut reader = std::io::Cursor::new(b"");
        let mut read_writer: AsyncFixedBuf<8> = AsyncFixedBuf::new();
        let mut chain = AsyncReadWriteChain::new(&mut reader, &mut read_writer);
        let mut buf = [b'.'; 8];
        assert_eq!(
            0,
            tokio::io::AsyncReadExt::read(&mut chain, &mut buf)
                .await
                .unwrap()
        );
        assert_eq!("........", escape_ascii(&buf));
    }

    #[tokio::test]
    async fn doesnt_read_second_when_first_has_data() {
        let mut reader = std::io::Cursor::new(b"abc");
        let mut read_writer = FakeAsyncReadWriter::empty();
        let mut chain = AsyncReadWriteChain::new(&mut reader, &mut read_writer);
        let mut buf = [b'.'; 4];
        assert_eq!(
            3,
            tokio::io::AsyncReadExt::read(&mut chain, &mut buf)
                .await
                .unwrap()
        );
        assert_eq!("abc.", escape_ascii(&buf));
    }

    #[tokio::test]
    async fn doesnt_read_second_when_first_returns_error() {
        let mut reader = FakeAsyncReadWriter::new(vec![Err(err1()), Err(err1())]);
        let mut read_writer = FakeAsyncReadWriter::empty();
        let mut chain = AsyncReadWriteChain::new(&mut reader, &mut read_writer);
        let mut buf = [b'.'; 4];
        let err = tokio::io::AsyncReadExt::read(&mut chain, &mut buf)
            .await
            .unwrap_err();
        assert_eq!(std::io::ErrorKind::Other, err.kind());
        assert_eq!("err1", err.to_string());
        assert_eq!("....", escape_ascii(&buf));
        tokio::io::AsyncReadExt::read(&mut chain, &mut buf)
            .await
            .unwrap_err();
    }

    #[tokio::test]
    async fn reads_second_when_first_empty() {
        let mut reader = std::io::Cursor::new(b"");
        let mut read_writer: AsyncFixedBuf<4> = AsyncFixedBuf::new();
        read_writer.write_str("abc").unwrap();
        let mut chain = AsyncReadWriteChain::new(&mut reader, &mut read_writer);
        let mut buf = [b'.'; 4];
        assert_eq!(
            3,
            tokio::io::AsyncReadExt::read(&mut chain, &mut buf)
                .await
                .unwrap()
        );
        assert_eq!("abc.", escape_ascii(&buf));
    }

    #[tokio::test]
    async fn reads_first_then_second() {
        let mut reader = std::io::Cursor::new(b"ab");
        let mut read_writer: AsyncFixedBuf<4> = AsyncFixedBuf::new();
        read_writer.write_str("cd").unwrap();
        let mut chain = AsyncReadWriteChain::new(&mut reader, &mut read_writer);
        let mut buf = [b'.'; 4];
        assert_eq!(
            2,
            tokio::io::AsyncReadExt::read(&mut chain, &mut buf)
                .await
                .unwrap()
        );
        assert_eq!("ab..", escape_ascii(&buf));
        assert_eq!(
            2,
            tokio::io::AsyncReadExt::read(&mut chain, &mut buf)
                .await
                .unwrap()
        );
        assert_eq!("cd..", escape_ascii(&buf));
    }

    #[tokio::test]
    async fn returns_error_from_second() {
        let mut reader = std::io::Cursor::new(b"");
        let mut read_writer = FakeAsyncReadWriter::new(vec![Err(err1()), Err(err1())]);
        let mut chain = AsyncReadWriteChain::new(&mut reader, &mut read_writer);
        let mut buf = [b'.'; 4];
        let err = tokio::io::AsyncReadExt::read(&mut chain, &mut buf)
            .await
            .unwrap_err();
        assert_eq!(std::io::ErrorKind::Other, err.kind());
        assert_eq!("err1", err.to_string());
        assert_eq!("....", escape_ascii(&buf));
        tokio::io::AsyncReadExt::read(&mut chain, &mut buf)
            .await
            .unwrap_err();
    }

    #[tokio::test]
    async fn passes_writes_through() {
        let mut reader = std::io::Cursor::new(b"");
        let mut read_writer: AsyncFixedBuf<4> = AsyncFixedBuf::new();
        let mut chain = AsyncReadWriteChain::new(&mut reader, &mut read_writer);
        assert_eq!(
            3,
            tokio::io::AsyncWriteExt::write(&mut chain, b"abc")
                .await
                .unwrap()
        );
        assert_eq!("abc", read_writer.escape_ascii());
    }
}

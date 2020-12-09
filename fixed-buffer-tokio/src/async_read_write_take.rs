#![forbid(unsafe_code)]

use std::task::Context;
use tokio::io::{AsyncRead, AsyncWrite, ReadBuf};
use tokio::macros::support::{Pin, Poll};

/// Wraps a struct that implements
/// [`AsyncRead`](https://docs.rs/tokio/latest/tokio/io/trait.AsyncRead.html)+[`AsyncWrite`](https://docs.rs/tokio/latest/tokio/io/trait.AsyncWrite.html).
/// Passes through reads and writes to the struct.
/// Limits the number of bytes that can be read.
///
/// This is like [`tokio::io::Take`](https://docs.rs/tokio/latest/tokio/io/struct.Take.html)
/// that passes through writes.
/// This makes it usable with AsyncRead+AsyncWrite objects like
/// [`tokio::net::TcpStream`](https://docs.rs/tokio/latest/tokio/net/struct.TcpStream.html)
/// and
/// [`tokio_rustls::server::TlsStream`](https://docs.rs/tokio-rustls/latest/tokio_rustls/server/struct.TlsStream.html).
pub struct AsyncReadWriteTake<'a, RW: tokio::io::AsyncRead + tokio::io::AsyncWrite + Send + Unpin> {
    read_writer: &'a mut RW,
    remaining_bytes: u64,
}

impl<'a, RW: AsyncRead + AsyncWrite + Send + Unpin> AsyncReadWriteTake<'a, RW> {
    /// See [`AsyncReadWriteTake`](struct.AsyncReadWriteTake.html).
    pub fn new(read_writer: &'a mut RW, len: u64) -> AsyncReadWriteTake<'a, RW> {
        Self {
            read_writer,
            remaining_bytes: len,
        }
    }
}

impl<'a, RW: AsyncRead + AsyncWrite + Send + Unpin> AsyncRead for AsyncReadWriteTake<'a, RW> {
    fn poll_read(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut ReadBuf<'_>,
    ) -> Poll<Result<(), std::io::Error>> {
        let mut_self = self.get_mut();
        if mut_self.remaining_bytes == 0 {
            return Poll::Ready(Ok(()));
        }
        let num_to_read = mut_self.remaining_bytes.min(buf.remaining() as u64) as usize;
        let dest = &mut buf.initialize_unfilled()[0..num_to_read];
        let mut buf2 = ReadBuf::new(dest);
        match Pin::new(&mut mut_self.read_writer).poll_read(cx, &mut buf2) {
            Poll::Ready(Ok(())) => {
                let num_read = buf2.filled().len();
                buf.advance(num_read);
                mut_self.remaining_bytes -= num_read as u64;
                Poll::Ready(Ok(()))
            }
            Poll::Ready(Err(e)) => Poll::Ready(Err(e)),
            Poll::Pending => Poll::Pending,
        }
    }
}

impl<'a, RW: AsyncRead + AsyncWrite + Send + Unpin> AsyncWrite for AsyncReadWriteTake<'a, RW> {
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
    async fn read_error() {
        let mut read_writer = FakeAsyncReadWriter::new(vec![Err(err1()), Ok(2), Ok(0)]);
        let mut take = AsyncReadWriteTake::new(&mut read_writer, 3);
        let mut buf = [b'.'; 4];
        assert_eq!(
            "err1",
            tokio::io::AsyncReadExt::read(&mut take, &mut buf)
                .await
                .unwrap_err()
                .to_string()
        );
        assert_eq!(
            2,
            tokio::io::AsyncReadExt::read(&mut take, &mut buf)
                .await
                .unwrap()
        );
        assert_eq!("ab..", escape_ascii(&buf));
        assert_eq!(
            0,
            tokio::io::AsyncReadExt::read(&mut take, &mut buf)
                .await
                .unwrap()
        );
        assert_eq!("ab..", escape_ascii(&buf));
    }

    #[tokio::test]
    async fn empty() {
        let mut read_writer = FakeAsyncReadWriter::new(vec![Ok(0)]);
        let mut take = AsyncReadWriteTake::new(&mut read_writer, 3);
        let mut buf = [b'.'; 4];
        assert_eq!(
            0,
            tokio::io::AsyncReadExt::read(&mut take, &mut buf)
                .await
                .unwrap()
        );
        assert_eq!("....", escape_ascii(&buf));
    }

    #[tokio::test]
    async fn doesnt_read_when_zero() {
        let mut read_writer = FakeAsyncReadWriter::empty();
        let mut take = AsyncReadWriteTake::new(&mut read_writer, 0);
        let mut buf = [b'.'; 4];
        assert_eq!(
            0,
            tokio::io::AsyncReadExt::read(&mut take, &mut buf)
                .await
                .unwrap()
        );
        assert_eq!("....", escape_ascii(&buf));
    }

    #[tokio::test]
    async fn fewer_than_len() {
        let mut read_writer = FakeAsyncReadWriter::new(vec![Ok(2), Ok(0)]);
        let mut take = AsyncReadWriteTake::new(&mut read_writer, 3);
        let mut buf = [b'.'; 4];
        assert_eq!(
            2,
            tokio::io::AsyncReadExt::read(&mut take, &mut buf)
                .await
                .unwrap()
        );
        assert_eq!("ab..", escape_ascii(&buf));
        assert_eq!(
            0,
            tokio::io::AsyncReadExt::read(&mut take, &mut buf)
                .await
                .unwrap()
        );
        assert_eq!("ab..", escape_ascii(&buf));
    }

    #[tokio::test]
    async fn fewer_than_len_in_multiple_reads() {
        let mut read_writer = FakeAsyncReadWriter::new(vec![Ok(2), Ok(2), Ok(0)]);
        let mut take = AsyncReadWriteTake::new(&mut read_writer, 5);
        let mut buf = [b'.'; 4];
        assert_eq!(
            2,
            tokio::io::AsyncReadExt::read(&mut take, &mut buf)
                .await
                .unwrap()
        );
        assert_eq!("ab..", escape_ascii(&buf));
        assert_eq!(
            2,
            tokio::io::AsyncReadExt::read(&mut take, &mut buf)
                .await
                .unwrap()
        );
        assert_eq!("cd..", escape_ascii(&buf));
        assert_eq!(
            0,
            tokio::io::AsyncReadExt::read(&mut take, &mut buf)
                .await
                .unwrap()
        );
        assert_eq!("cd..", escape_ascii(&buf));
    }

    #[tokio::test]
    async fn exactly_len() {
        let mut read_writer = FakeAsyncReadWriter::new(vec![Ok(3), Ok(0)]);
        let mut take = AsyncReadWriteTake::new(&mut read_writer, 3);
        let mut buf = [b'.'; 4];
        assert_eq!(
            3,
            tokio::io::AsyncReadExt::read(&mut take, &mut buf)
                .await
                .unwrap()
        );
        assert_eq!("abc.", escape_ascii(&buf));
        assert_eq!(
            0,
            tokio::io::AsyncReadExt::read(&mut take, &mut buf)
                .await
                .unwrap()
        );
        assert_eq!("abc.", escape_ascii(&buf));
    }

    #[tokio::test]
    async fn exactly_len_in_multiple_reads() {
        let mut read_writer = FakeAsyncReadWriter::new(vec![Ok(2), Ok(1), Ok(0)]);
        let mut take = AsyncReadWriteTake::new(&mut read_writer, 3);
        let mut buf = [b'.'; 4];
        assert_eq!(
            2,
            tokio::io::AsyncReadExt::read(&mut take, &mut buf)
                .await
                .unwrap()
        );
        assert_eq!("ab..", escape_ascii(&buf));
        assert_eq!(
            1,
            tokio::io::AsyncReadExt::read(&mut take, &mut buf)
                .await
                .unwrap()
        );
        assert_eq!("cb..", escape_ascii(&buf));
        assert_eq!(
            0,
            tokio::io::AsyncReadExt::read(&mut take, &mut buf)
                .await
                .unwrap()
        );
        assert_eq!("cb..", escape_ascii(&buf));
    }

    #[tokio::test]
    async fn doesnt_call_read_after_len_reached() {
        let mut read_writer = FakeAsyncReadWriter::new(vec![Ok(3)]);
        let mut take = AsyncReadWriteTake::new(&mut read_writer, 3);
        let mut buf = [b'.'; 4];
        assert_eq!(
            3,
            tokio::io::AsyncReadExt::read(&mut take, &mut buf)
                .await
                .unwrap()
        );
        assert_eq!("abc.", escape_ascii(&buf));
        assert_eq!(
            0,
            tokio::io::AsyncReadExt::read(&mut take, &mut buf)
                .await
                .unwrap()
        );
        assert_eq!("abc.", escape_ascii(&buf));
    }

    #[tokio::test]
    async fn doesnt_call_read_after_len_reached_in_multiple_reads() {
        let mut read_writer = FakeAsyncReadWriter::new(vec![Ok(2), Ok(1)]);
        let mut take = AsyncReadWriteTake::new(&mut read_writer, 3);
        let mut buf = [b'.'; 4];
        assert_eq!(
            2,
            tokio::io::AsyncReadExt::read(&mut take, &mut buf)
                .await
                .unwrap()
        );
        assert_eq!("ab..", escape_ascii(&buf));
        assert_eq!(
            1,
            tokio::io::AsyncReadExt::read(&mut take, &mut buf)
                .await
                .unwrap()
        );
        assert_eq!("cb..", escape_ascii(&buf));
        assert_eq!(
            0,
            tokio::io::AsyncReadExt::read(&mut take, &mut buf)
                .await
                .unwrap()
        );
        assert_eq!("cb..", escape_ascii(&buf));
    }

    #[tokio::test]
    async fn passes_writes_through() {
        let mut read_writer = FakeAsyncReadWriter::new(vec![Ok(3)]);
        let mut take = AsyncReadWriteTake::new(&mut read_writer, 2);
        assert_eq!(
            3,
            tokio::io::AsyncWriteExt::write(&mut take, b"abc")
                .await
                .unwrap()
        );
        assert!(read_writer.is_empty());
    }
}

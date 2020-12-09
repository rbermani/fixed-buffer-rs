#![forbid(unsafe_code)]

use std::task::Context;
use tokio::io::ReadBuf;
use tokio::macros::support::{Pin, Poll};

pub fn err1() -> std::io::Error {
    std::io::Error::new(std::io::ErrorKind::Other, "err1")
}

pub struct FakeAsyncReadWriter {
    results: Vec<Result<usize, std::io::Error>>,
    byte_iter: Box<dyn std::iter::Iterator<Item = u8> + Send>,
}

impl FakeAsyncReadWriter {
    fn make_byte_iter() -> Box<dyn std::iter::Iterator<Item = u8> + Send> {
        Box::new(
            std::iter::repeat(b"abcdefghijklmnopqrstuvwxyz".iter().map(|ref_u8| *ref_u8)).flatten(),
        )
    }

    pub fn new(results: Vec<Result<usize, std::io::Error>>) -> Self {
        Self {
            results,
            byte_iter: Self::make_byte_iter(),
        }
    }

    pub fn empty() -> Self {
        Self {
            results: Vec::new(),
            byte_iter: Self::make_byte_iter(),
        }
    }

    pub fn is_empty(&self) -> bool {
        self.results.is_empty()
    }
}

impl tokio::io::AsyncRead for FakeAsyncReadWriter {
    fn poll_read(
        self: Pin<&mut Self>,
        _cx: &mut Context<'_>,
        buf: &mut ReadBuf<'_>,
    ) -> Poll<Result<(), std::io::Error>> {
        if self.results.is_empty() {
            panic!("unexpected read");
        }
        let mut_self = self.get_mut();
        match mut_self.results.remove(0) {
            Ok(num_read) => {
                if num_read > 0 {
                    let src: Vec<u8> =
                        std::iter::Iterator::take(&mut mut_self.byte_iter, num_read).collect();
                    let dest = &mut buf.initialize_unfilled()[0..num_read];
                    dest.copy_from_slice(src.as_slice());
                    buf.advance(num_read)
                }
                Poll::Ready(Ok(()))
            }
            Err(e) => Poll::Ready(Err(e)),
        }
    }
}

impl tokio::io::AsyncWrite for FakeAsyncReadWriter {
    fn poll_write(
        self: Pin<&mut Self>,
        _cx: &mut Context<'_>,
        _buf: &[u8],
    ) -> Poll<Result<usize, std::io::Error>> {
        let mut_self = self.get_mut();
        if mut_self.results.is_empty() {
            panic!("unexpected write");
        }
        Poll::Ready(mut_self.results.remove(0))
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

//! Example server that uses `fixed_buffer` crate to parse a simple line-based protocol.
//!
//! ```
//! $ cargo test --package fixed-buffer-tokio --test server -- --nocapture
//! running 1 test
//! SERVER listening on 127.0.0.1:49379
//! CLIENT connecting
//! CLIENT sending two requests at the same time: CRC('aaaa') and HELLO
//! SERVER handling connection
//! CLIENT got response "ad98e545"
//! CLIENT got response "HI"
//! test main ... ok
//!
//! test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out
//! ```
#![forbid(unsafe_code)]

use crc::Hasher32;
use fixed_buffer_tokio::{AsyncFixedBuf, AsyncReadWriteChain, AsyncReadWriteTake};
use std::io::ErrorKind;
use std::net::{Shutdown, SocketAddr};
use std::println;
use std::task::Context;
use std::time::Duration;
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};
use tokio::macros::support::{Pin, Poll};
use tokio::net::{TcpListener, TcpStream};

async fn handle_hello<W: AsyncWrite + Send + Unpin>(mut writer: W) -> Result<(), std::io::Error> {
    writer.write_all("HI\n".as_bytes()).await
}

struct Hasher32Writer<'a, T: Hasher32>(&'a mut T);
impl<'a, T: Hasher32> AsyncWrite for Hasher32Writer<'a, T> {
    fn poll_write(
        self: Pin<&mut Self>,
        _cx: &mut Context<'_>,
        buf: &[u8],
    ) -> Poll<Result<usize, std::io::Error>> {
        self.get_mut().0.write(buf);
        Poll::Ready(Ok(buf.len()))
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

async fn handle_crc32<RW: AsyncRead + AsyncWrite + Send + Unpin>(
    mut read_writer: &mut RW,
    len: u64,
) -> Result<(), std::io::Error> {
    let mut digest = crc::crc32::Digest::new(crc::crc32::IEEE);
    let mut payload = AsyncReadWriteTake::new(&mut read_writer, len);
    tokio::io::copy(&mut payload, &mut Hasher32Writer(&mut digest)).await?;
    let response = format!("{:x}\n", digest.sum32());
    read_writer.write_all(response.as_bytes()).await
}

#[derive(Debug, PartialEq)]
enum Request {
    Hello,
    Crc32(u64),
}

impl Request {
    pub fn parse(line_bytes: &[u8]) -> Option<Request> {
        // println!("SERVER parsing {:?}", fixed_buffer::escape_ascii(line_bytes));
        let line = std::str::from_utf8(line_bytes).ok()?;
        let mut parts = line.splitn(2, " ");
        let method = parts.next().unwrap();
        let arg = parts.next();
        match (method, arg) {
            ("HELLO", None) => Some(Request::Hello),
            ("CRC32", Some(arg)) => {
                let len: u64 = std::str::FromStr::from_str(arg).ok()?;
                if len <= 1024 * 1024 {
                    Some(Request::Crc32(len))
                } else {
                    None
                }
            }
            _ => None,
        }
    }
}

async fn handle_conn(mut tcp_stream: TcpStream) -> Result<(), std::io::Error> {
    println!("SERVER handling connection");
    let mut buf: AsyncFixedBuf<4096> = AsyncFixedBuf::new();
    loop {
        let line_bytes = match buf
            .read_frame(&mut tcp_stream, fixed_buffer::deframe_line)
            .await?
        {
            Some(line_bytes) => line_bytes,
            None => return Ok(()),
        };
        match Request::parse(line_bytes) {
            Some(Request::Hello) => handle_hello(&mut tcp_stream).await?,
            Some(Request::Crc32(len)) => {
                let mut read_writer = AsyncReadWriteChain::new(&mut buf, &mut tcp_stream);
                handle_crc32(&mut read_writer, len).await?
            }
            _ => tcp_stream.write_all("ERROR\n".as_bytes()).await?,
        };
    }
}

#[tokio::test]
pub async fn main() {
    let listener = TcpListener::bind(SocketAddr::from(([127, 0, 0, 1], 0)))
        .await
        .unwrap();
    let addr = listener.local_addr().unwrap();
    println!("SERVER listening on {}", addr);
    tokio::spawn(async move {
        loop {
            match listener.accept().await {
                Ok((tcp_stream, _addr)) => {
                    tokio::spawn(async move {
                        if let Err(e) = handle_conn(tcp_stream).await {
                            if e.kind() != ErrorKind::NotFound {
                                println!("SERVER error: {:?}", e);
                            }
                        }
                    });
                }
                Err(e) => {
                    println!("SERVER error accepting connection: {:?}", e);
                    tokio::time::sleep(Duration::from_secs(1)).await;
                }
            }
        }
    });

    println!("CLIENT connecting");
    let mut tcp_stream = TcpStream::connect(addr).await.unwrap();
    println!("CLIENT sending two requests at the same time: CRC('aaaa') and HELLO");
    tcp_stream.write_all(b"CRC32 4\naaaaHELLO\n").await.unwrap();
    let mut response = String::new();
    tcp_stream.shutdown(Shutdown::Write).unwrap();
    tcp_stream.read_to_string(&mut response).await.unwrap();
    for line in response.lines() {
        println!("CLIENT got response {:?}", line);
    }
}

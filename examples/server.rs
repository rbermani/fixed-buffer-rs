// Example server that uses `fixed_buffer` crate to parse a simple line-based protocol.

use crc::Hasher32;
use fixed_buffer::FixedBuf;
use std::io::{Error, ErrorKind};
use std::net::SocketAddr;
use std::println;
use std::task::Context;
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};
use tokio::macros::support::{Pin, Poll};
use tokio::net::{TcpListener, TcpStream};

async fn handle_hello<W: AsyncWrite + Unpin>(mut output: W) -> Result<(), Error> {
    AsyncWriteExt::write_all(&mut output, "HI\n".as_bytes()).await
}

struct Hasher32AsyncWriter<'a, T: Hasher32>(&'a mut T);

impl<'a, T: Hasher32> AsyncWrite for Hasher32AsyncWriter<'a, T> {
    fn poll_write(
        self: Pin<&mut Self>,
        _cx: &mut Context<'_>,
        buf: &[u8],
    ) -> Poll<Result<usize, Error>> {
        self.get_mut().0.write(buf);
        Poll::Ready(Ok(buf.len()))
    }

    fn poll_flush(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<Result<(), Error>> {
        Poll::Ready(Ok(()))
    }

    fn poll_shutdown(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<Result<(), Error>> {
        Poll::Ready(Ok(()))
    }
}

async fn handle_crc32<W: AsyncWrite + Unpin, R: AsyncRead + Unpin>(
    mut output: W,
    input: R,
    len: u64,
) -> Result<(), Error> {
    let mut digest = crc::crc32::Digest::new(crc::crc32::IEEE);
    let mut payload = input.take(len);
    tokio::io::copy(&mut payload, &mut Hasher32AsyncWriter(&mut digest)).await?;
    let response = format!("{:x}\n", digest.sum32());
    AsyncWriteExt::write_all(&mut output, response.as_bytes()).await?;
    Ok(())
}

#[derive(Debug, PartialEq)]
enum Request {
    Hello,
    Crc32(u64),
}

impl Request {
    pub fn parse(line_bytes: &[u8]) -> Option<Request> {
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

async fn handle_conn(mut tcp_stream: TcpStream) -> Result<(), Error> {
    println!("SERVER handling connection");
    let (mut input, mut output) = tcp_stream.split();
    let mut buf: FixedBuf = FixedBuf::new();
    loop {
        let line_bytes = buf.read_delimited(&mut input, b"\n").await?;
        match Request::parse(line_bytes) {
            Some(Request::Hello) => handle_hello(&mut output).await?,
            Some(Request::Crc32(len)) => {
                let payload_reader = tokio::io::AsyncReadExt::chain(&mut buf, &mut input);
                handle_crc32(&mut output, payload_reader, len).await?
            }
            _ => AsyncWriteExt::write_all(&mut output, "ERROR\n".as_bytes()).await?,
        };
    }
}

#[tokio::main]
pub async fn main() -> Result<(), Error> {
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
                    tokio::time::sleep(std::time::Duration::from_secs(1)).await;
                }
            }
        }
    });

    println!("CLIENT connecting");
    let mut tcp_stream = TcpStream::connect(addr).await.unwrap();
    println!("CLIENT sending two requests at the same time: CRC('aaaa') and HELLO");
    AsyncWriteExt::write_all(&mut tcp_stream, b"CRC32 4\naaaaHELLO\n")
        .await
        .unwrap();
    let mut response = String::new();
    tcp_stream.shutdown(std::net::Shutdown::Write).unwrap();
    AsyncReadExt::read_to_string(&mut tcp_stream, &mut response)
        .await
        .unwrap();
    for line in response.lines() {
        println!("CLIENT got response {:?}", line);
    }
    Ok(())
}

// $ cargo run --package fixed-buffer --example server
// SERVER listening on 127.0.0.1:61779
// CLIENT connecting
// CLIENT sending requests
// SERVER handling connection
// CLIENT got response "ad98e545"
// CLIENT got response "HI"

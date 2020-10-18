# fixed-buffer
![Apache 2.0 licensed](https://img.shields.io/github/license/leonhard-llc/fixed-buffer-rs)

This is a Rust library with fixed-size buffers, useful for network protocol parsers.

## Features
- Write bytes to the buffer and read them back
- Lives on the stack
- Does not allocate memory
- Supports tokio's AsyncRead and AsyncWrite
- Use it to read a stream, search for a delimiter, and save leftover bytes for the next read.

## Limitations
- Not a circular buffer.
  You can call `shift()` periodically to move unread bytes to the front of the buffer.

## Example
```rust
async fn handle_conn(mut tcp_stream: TcpStream) -> Result<(), Error> {
    println!("SERVER handling connection");
    let mut buf: FixedBuf = FixedBuf::new();
    loop {
        let line_bytes = buf.read_delimited(&mut tcp_stream, b"\n").await?;
        match Request::parse(line_bytes) {
            Some(Request::Hello) => handle_hello(&mut tcp_stream).await?,
            Some(Request::Crc32(len)) => handle_crc32(&mut tcp_stream, &mut buf, len).await?,
            _ => AsyncWriteExt::write_all(&mut tcp_stream, "ERROR\n".as_bytes()).await?,
        };
    }
}
```
See [examples/server.rs](examples/server.rs)

## Documentation
https://docs.rs/fixed-buffer

## Alternatives
- [bytes](https://docs.rs/bytes/0.5.6/bytes/index.html)
- [buf_redux](https://crates.io/crates/buf_redux), circular buffer support
- [std::io::BufReader](https://doc.rust-lang.org/std/io/struct.BufReader.html)
- [std::io::BufWriter](https://doc.rust-lang.org/std/io/struct.BufWriter.html)
- [static-buffer](https://crates.io/crates/static-buffer), updated in 2016
- [block-buffer](https://crates.io/crates/block-buffer), for processing fixed-length blocks of data

## TODO
- Try to make this crate comply with the [Rust API Guidelines](https://rust-lang.github.io/api-guidelines/).
- Set up continuous integration tests and banner.
  - https://github.com/actions-rs/example
  - https://alican.codes/rust-github-actions/
- Make the repo public
- Find out how to include Readme.md info in the crate's docs.
- Add some documentation tests
  - https://doc.rust-lang.org/stable/rust-by-example/testing/doc_testing.html
- Get a code review from an experienced rustacean
- Publish to creates.io
- Add features: std, tokio, async-std
- Custom buffer length.
  - https://crates.io/crates/generic-array
  - https://crates.io/crates/block-buffer
  - https://crates.io/crates/string-wrapper
- Simplify `read_delimited()`
- Implement FixedBuf::chain(AsyncRead) which buffers reads like [tokio::io::ReadBuf](https://docs.rs/tokio/0.3.0/tokio/io/struct.ReadBuf.html).

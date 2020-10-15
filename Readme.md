# fixed-buffer
![Apache 2.0 licensed](https://img.shields.io/github/license/leonhard-llc/fixed-buffer-rs)

This is a Rust library with fixed-size buffers, useful for network protocol parsers.

## Features
- Write bytes to the buffer and read them back
- Lives on the stack
- Does not allocate memory
- Supports tokio's AsyncRead and AsyncWrite
- Use it to read a socket, search for a delimiter, and buffer unused bytes for the next read.

## Limitations
- Not a circular buffer.
  You can call `shift()` periodically to move unread bytes to the front of the buffer.

## Example
```rust
async fn handle_conn(mut tcp_stream: tokio::net::TcpStream) -> Result<(), std::io::Error> {
    println!("SERVER handling connection");
    let mut buf: fixed_buffer::FixedBuf = fixed_buffer::FixedBuf::new();
    loop {
        let line_bytes = buf.read_delimited(&mut tcp_stream, b"\n").await?;
        let line = std::str::from_utf8(line_bytes)
            .map_err(|_e| std::io::Error::from(std::io::ErrorKind::InvalidData))?;
        println!("SERVER got request {:?}", line);
        handle_request(&mut tcp_stream, line).await?;
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

## TODO
- Try to make this crate comply with the [Rust API Guidelines](https://rust-lang.github.io/api-guidelines/).
- Set up continuous integration tests and banner.
  - https://github.com/actions-rs/example
  - https://alican.codes/rust-github-actions/
- Make the repo public
- Find out how to include Readme.md info in the crate's docs.
- Get a code review from an experienced rustacean
- Publish to creates.io
- Add feature flags: nostd, std, tokio, async-std
- Custom buffer length.
  See how [string-wrapper](https://crates.io/crates/string-wrapper) does it.
- Simplify `read_delimited()`

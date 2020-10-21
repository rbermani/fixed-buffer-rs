# fixed-buffer
<!---
license-apache-2.0.svg made with https://shields.io/
https://img.shields.io/badge/license-Apache--2.0-blue

Both coveralls.io and codecov.io demand access to lots of unnecessary data from my
GitHub account.  No thanks.
--->
![Apache 2.0 licensed](license-apache-2.0.svg) [![Latest Version](https://img.shields.io/crates/v/fixed-buffer.svg)](https://crates.io/crates/fixed-buffer)

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

## Examples
Read and process records:
```rust
let mut buf: FixedBuf = FixedBuf::new();
loop {
    // Read a chunk into the buffer.
    let mut writable = buf.writable()
        .ok_or(Error::new(ErrorKind::InvalidData, "record too long, buffer full"))?;
    let bytes_written = AsyncReadExt::read(&mut input, &mut writable)?;
    if bytes_written == 0 {
        return Err(Error::from(ErrorKind::UnexpectedEof));
    }
    buf.wrote(bytes_written);

    // Process records in the buffer.
    loop {
        let bytes_read = try_process_record(buf.readable())?;
        if bytes_read == 0 {
            break;
        }
        buf.read(bytes_read);
    }
    // Shift data in the buffer to free up space at the end for writing.
    buf.shift();
}
```

Read and handle requests from a remote client:
```rust
use fixed_buffer::FixedBuf;
use std::io::Error;
use tokio::io::AsyncWriteExt;
use tokio::net::TcpStream;

async fn handle_conn(mut tcp_stream: TcpStream) -> Result<(), Error> {
    let (mut input, mut output) = tcp_stream.split();
    let mut buf: FixedBuf = FixedBuf::new();
    loop {
        // Read a line and leave leftover bytes in `buf`.
        let line_bytes: &[u8] = buf.read_delimited(&mut input, b"\n").await?;
        let request = Request::parse(line_bytes)?;
        // Read any request payload from `buf` + `TcpStream`.
        let payload_reader = tokio::io::AsyncReadExt::chain(&mut buf, &mut input);
        handle_request(&mut output, payload_reader, request).await?;
    }
}
```

For a runnable example, see [examples/server.rs](examples/server.rs).

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
- DONE - Try to make this crate comply with the [Rust API Guidelines](https://rust-lang.github.io/api-guidelines/).
- DONE - Find out how to include Readme.md info in the crate's docs.
- DONE - Make the repo public
- DONE - Set up continuous integration tests and banner.
  - https://github.com/actions-rs/example
  - https://alican.codes/rust-github-actions/
- DONE - Add some documentation tests
  - https://doc.rust-lang.org/rustdoc/documentation-tests.html
  - https://doc.rust-lang.org/stable/rust-by-example/testing/doc_testing.html
- Switch to Gitlab because GitHub code reviews suck [[1](https://github.com/isaacs/github/issues/284), [2](https://github.community/t/review-comments-not-shown-in-files-changed-if-a-commit-affects-the-comments-associated-line/1867)]
  - https://gitlab.com/mattdark/firebase-example/blob/master/.gitlab-ci.yml
  - https://medium.com/astraol/optimizing-ci-cd-pipeline-for-rust-projects-gitlab-docker-98df64ae3bc4
  - https://hub.docker.com/_/rust
- Get a code review from an experienced rustacean
- Publish to creates.io
- Add features: std, tokio, async-std
- Custom buffer length.
  - https://crates.io/crates/generic-array
  - https://crates.io/crates/block-buffer
  - https://crates.io/crates/string-wrapper
- Simplify `read_delimited()`
- Make a more generic read_frame that takes a frame detector function.
  Make `read_delimited` use that.
- Implement FixedBuf::chain(AsyncRead) which buffers reads like [tokio::io::ReadBuf](https://docs.rs/tokio/0.3.0/tokio/io/struct.ReadBuf.html).

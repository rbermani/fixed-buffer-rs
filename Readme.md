# fixed-buffer
<!---
Most GitHub projects get license badges from shields.io, which uses GitHub's API
https://docs.github.com/en/free-pro-team@latest/rest/reference/licenses ,
which uses the Ruby Gem `licensee`, which doesn't detect the LICENSE file as
Apache-2.0.  The gem expects a boilerplate Apache-2.0 license file which includes
instructions on applying the license.  It doesn't expect a license file that was
created by actually following the instructions in the boilerplate.  I guess
Apache-2.0 license and the gem were created when everyone put the license at the top
of every source file.

$ curl -sS -H "Accept: application/vnd.github.v3+json" https://api.github.com/repos/apache/flink/license |grep key
    "key": "apache-2.0",
$ curl -sS -H "Accept: application/vnd.github.v3+json" https://api.github.com/repos/leonhard-llc/fixed-buffer-rs/license |grep key
    "key": "other",

So I just made the badge manually with https://shields.io/
https://img.shields.io/badge/license-Apache--2.0-blue
and added the SVG file to the project.
--->
![Apache 2.0 licensed](license-apache-2.0.svg)

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

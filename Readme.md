# fixed-buffer
This is a Rust library with fixed-size buffers, useful for network protocol parsers.

## Features
- Write bytes to the buffer and read them back
- Lives on the stack
- Does not allocate memory
- Supports tokio's AsyncRead and AsyncWrite
- Use it to read a stream, search for a delimiter, and save leftover bytes for the next read.
- Easy to learn & use.  Easy to maintain code that uses it.
- Works with Rust `latest`, `beta`, and `nightly`

## Limitations
- Not a circular buffer.
  You can call `shift()` periodically to move unread bytes to the front of the buffer.
- There is no `iterate_delimited(AsyncRead)`.
  Because of borrowing rules, such a function would need to return non-borrowed (allocated and copied) data.

## Examples
Read and handle requests from a remote client:
```rust
use fixed_buffer::FixedBuf;
use std::io::Error;
use tokio::io::AsyncWriteExt;
use tokio::net::TcpStream;

async fn handle_conn(mut tcp_stream: TcpStream) -> Result<(), Error> {
    let (mut input, mut output) = tcp_stream.split();
    let mut buf: FixedBuf<[u8; 4096]> = FixedBuf::new([0; 4096]);
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

Read and process records:
```rust
fn try_process_record(b: &[u8]) -> Result<usize, Error> {
    if b.len() < 2 {
        return Ok(0)
    }
    if b.starts_with("ab".as_bytes()) {
        println!("found record");
        Ok(2)
    } else {
        Err(Error::new(ErrorKind::InvalidData, "bad record"))
    }
}

async fn read_and_process<R: tokio::io::AsyncRead + Unpin>(mut input: R)
    -> Result<(), Error> {
    let mut buf: FixedBuf<[u8; 1024]> = FixedBuf::new([0; 1024]);
    loop {
        // Read a chunk into the buffer.
        let mut writable = buf.writable()
            .ok_or(Error::new(ErrorKind::InvalidData, "record too long, buffer full"))?;
        let bytes_written = AsyncReadExt::read(&mut input, &mut writable).await?;
        if bytes_written == 0 {
            return if buf.len() == 0 {
                Ok(())  // EOF at record boundary
            } else {
                // EOF in the middle of a record
                Err(Error::from(ErrorKind::UnexpectedEof))
            };
        }
        buf.wrote(bytes_written);

        // Process records in the buffer.
        loop {
            let bytes_read = try_process_record(buf.readable())?;
            if bytes_read == 0 {
                break;
            }
            buf.read_bytes(bytes_read);
        }
        // Shift data in the buffer to free up space at the end for writing.
        buf.shift();
    }
}
```

## Documentation
https://docs.rs/fixed-buffer

## Alternatives
- [bytes](https://docs.rs/bytes/0.5.6/bytes/index.html)
- [buf_redux](https://crates.io/crates/buf_redux), circular buffer support
- [std::io::BufReader](https://doc.rust-lang.org/std/io/struct.BufReader.html)
- [std::io::BufWriter](https://doc.rust-lang.org/std/io/struct.BufWriter.html)
- [static-buffer](https://crates.io/crates/static-buffer), updated in 2016
- [block-buffer](https://crates.io/crates/block-buffer), for processing fixed-length blocks of data

## Release Process
1. Edit `Cargo.toml` and bump version number.
1. Run `./release.sh`

## Changelog
- v0.1.3
  - Thanks to [freax13](https://gitlab.com/Freax13) for these changes:
    - Support any buffer size.  Now you can make `FixedBuf<[u8; 42]>`.
    - Support any AsRef<[u8]> + AsMut<[u8]> value for internal memory:
      - `[u8; N]`
      - `Box<[u8; N]>`
      - `&mut [u8]`
      - `Vec<u8>`
  - Renamed `new_with_mem` to `new`.
    Use `FixedBuf::default()` to construct any `FixedBuf<T: Default>`, which includes
    [arrays of sizes up to 32](https://doc.rust-lang.org/std/primitive.array.html).
- v0.1.2 - Updated documentation.
- v0.1.1 - First published version

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
- DONE - Set up public repository on Gitlab.com
  - https://gitlab.com/mattdark/firebase-example/blob/master/.gitlab-ci.yml
  - https://medium.com/astraol/optimizing-ci-cd-pipeline-for-rust-projects-gitlab-docker-98df64ae3bc4
  - https://hub.docker.com/_/rust
- DONE - Custom buffer length.
  - https://crates.io/crates/generic-array
  - https://crates.io/crates/block-buffer
  - https://crates.io/crates/string-wrapper
- DONE - Publish to creates.io
- DONE - Read through https://crate-ci.github.io/index.html
- DONE - Get a code review from an experienced rustacean
- Add features: std, tokio, async-std
- Simplify `read_delimited()`
- Make a more generic read_frame that takes a frame detector function.
  Make `read_delimited` use that.
- Implement FixedBuf::chain(AsyncRead) which buffers reads like [tokio::io::ReadBuf](https://docs.rs/tokio/0.3.0/tokio/io/struct.ReadBuf.html).
- Fix FixedBuf rustdoc link to box_benchmark.
- Switch to const generics once they are stable:
  - https://github.com/rust-lang/rust/issues/44580
  - https://stackoverflow.com/a/56543462
- Set up CI on:
  - DONE - Linux x86 64-bit
  - [macOS](https://gitlab.com/gitlab-org/gitlab/-/issues/269756)
  - [Windows](https://about.gitlab.com/blog/2020/01/21/windows-shared-runner-beta/)
  - https://crate-ci.github.io/pr/testing.html#travisci
  - Linux ARM 64-bit (Raspberry Pi 3 and newer)
  - Linux ARM 32-bit (Raspberry Pi 2)
  - RISCV & ESP32 firmware?
- Add and update a changelog
  - https://crate-ci.github.io/release/changelog.html

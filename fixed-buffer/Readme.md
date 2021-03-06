# fixed-buffer

[![crates.io version](https://img.shields.io/crates/v/fixed-buffer.svg)](https://crates.io/crates/fixed-buffer)
[![license: Apache 2.0](https://gitlab.com/leonhard-llc/fixed-buffer-rs/-/raw/main/license-apache-2.0.svg)](http://www.apache.org/licenses/LICENSE-2.0)
[![unsafe forbidden](https://gitlab.com/leonhard-llc/fixed-buffer-rs/-/raw/main/unsafe-forbidden-success.svg)](https://github.com/rust-secure-code/safety-dance/)
[![pipeline status](https://gitlab.com/leonhard-llc/fixed-buffer-rs/badges/main/pipeline.svg)](https://gitlab.com/leonhard-llc/fixed-buffer-rs/-/pipelines)

This is a Rust library with fixed-size buffers,
useful for network protocol parsers and file parsers.

## Features
- No `unsafe`
- Depends only on `std`
- Write bytes to the buffer and read them back
- Lives on the stack
- Does not allocate memory
- Use it to read a stream, search for a delimiter,
  and save leftover bytes for the next read.
- Easy to learn & use.  Easy to maintain code that uses it.
- Works with Rust `latest`, `beta`, and `nightly`
- No macros
- Good test coverage (99%)
- [fixed_buffer_tokio](https://crates.io/crates/fixed-buffer-tokio) adds async functions

## Limitations
- Not a circular buffer.
  You can call `shift()` periodically
  to move unread bytes to the front of the buffer.

## Documentation
https://docs.rs/fixed-buffer

## Examples
Read and handle requests from a remote client:
```rust
use fixed_buffer::{
    deframe_line, FixedBuf, ReadWriteChain};
use std::io::{Error, Read, Write};
use std::net::TcpStream;

fn handle_request<RW: Read + Write>(
    reader_writer: &mut RW,
    request: Request,
) -> Result<(), Error> {
    // ...
    Ok(())
}

fn handle_conn(mut tcp_stream: TcpStream
) -> Result<(), Error> {
    let mut buf: FixedBuf<4096> = FixedBuf::new();
    loop {
        // Read a line
        // and leave leftover bytes in `buf`.
        let line_bytes = match buf.read_frame(
            &mut tcp_stream, deframe_line)? {
                Some(line_bytes) => line_bytes,
                None => return Ok(()),
            };
        let request = Request::parse(line_bytes)?;
        // Read any request payload
        // from `buf` + `TcpStream`.
        let mut reader_writer = ReadWriteChain::new(
            &mut buf, &mut tcp_stream);
        handle_request(&mut reader_writer, request)?;
    }
}
```
For a complete example, see
[`tests/server.rs`](https://gitlab.com/leonhard-llc/fixed-buffer-rs/-/blob/main/fixed-buffer/tests/server.rs).

Read and process records:
```rust
use fixed_buffer::FixedBuf;
use std::io::{Error, ErrorKind, Read};
use std::net::TcpStream;

fn try_process_record(b: &[u8]) -> Result<usize, Error> {
    if b.len() < 2 {
        return Ok(0);
    }
    if b.starts_with("ab".as_bytes()) {
        println!("found record");
        Ok(2)
    } else {
        Err(Error::new(ErrorKind::InvalidData, "bad record"))
    }
}

fn read_and_process<R: Read>(mut input: R)
    -> Result<(), Error> {
    let mut buf: FixedBuf<1024> = FixedBuf::new();
    loop {
        // Read a chunk into the buffer.
        if buf.copy_once_from(&mut input)? == 0 {
            return if buf.len() == 0 {
                // EOF at record boundary
                Ok(())
            } else {
                // EOF in the middle of a record
                Err(Error::from(
                    ErrorKind::UnexpectedEof))
            };
        }
        // Process records in the buffer.
        loop {
            let bytes_read =
                try_process_record(buf.readable())?;
            if bytes_read == 0 {
                break;
            }
            buf.read_bytes(bytes_read);
        }
        // Shift data in the buffer to free up
        // space at the end for writing.
        buf.shift();
    }
}
#
```

The [`filled`](https://docs.rs/fixed-buffer/latest/fixed_buffer/struct.FixedBuf.html#method.filled)
constructor is useful in tests.

## Alternatives
- [bytes](https://docs.rs/bytes/0.5.6/bytes/index.html)
- [buf_redux](https://crates.io/crates/buf_redux), circular buffer support
- [std::io::BufReader](https://doc.rust-lang.org/std/io/struct.BufReader.html)
- [std::io::BufWriter](https://doc.rust-lang.org/std/io/struct.BufWriter.html)
- [static-buffer](https://crates.io/crates/static-buffer), updated in 2016
- [block-buffer](https://crates.io/crates/block-buffer), for processing fixed-length blocks of data
- [arrayvec](https://crates.io/crates/arrayvec), vector with fixed capacity.

## Changelog
- v0.3.0 - Breaking API changes:
  - Change type parameter to const buffer size. Example: `FixedBuf<1024>`.
  - Remove `new` arg.
  - Remove `capacity`.
  - Remove `Copy` impl.
  - Change `writable` return type to `&mut [u8]`.
- v0.2.3
  - Add
    [`read_byte`](https://docs.rs/fixed-buffer/latest/fixed_buffer/struct.FixedBuf.html#method.read_byte),
    [`try_read_byte`](https://docs.rs/fixed-buffer/latest/fixed_buffer/struct.FixedBuf.html#method.try_read_byte),
    [`try_read_bytes`](https://docs.rs/fixed-buffer/latest/fixed_buffer/struct.FixedBuf.html#method.try_read_bytes),
    [`try_read_exact`](https://docs.rs/fixed-buffer/latest/fixed_buffer/struct.FixedBuf.html#method.try_read_exact),
    [`try_parse`](https://docs.rs/fixed-buffer/latest/fixed_buffer/struct.FixedBuf.html#method.try_parse).
  - Implement [`UnwindSafe`](https://doc.rust-lang.org/std/panic/trait.UnwindSafe.html)
- v0.2.2 - Add badges to readme
- v0.2.1 - Add
  [`deframe`](https://docs.rs/fixed-buffer/latest/fixed_buffer/struct.FixedBuf.html#method.deframe)
  and
  [`mem`](https://docs.rs/fixed-buffer/latest/fixed_buffer/struct.FixedBuf.html#method.mem),
  needed by `AsyncFixedBuf::read_frame`.
- v0.2.0
  - Move tokio support to [fixed_buffer_tokio](https://crates.io/crates/fixed-buffer-tokio).
  - Add
    [`copy_once_from`](https://docs.rs/fixed-buffer/latest/fixed_buffer/struct.FixedBuf.html#method.copy_once_from),
    [`read_block`](https://docs.rs/fixed-buffer/latest/fixed_buffer/struct.FixedBuf.html#method.read_block),
    [`ReadWriteChain`](https://docs.rs/fixed-buffer/latest/fixed_buffer/struct.ReadWriteChain.html),
    and
    [`ReadWriteTake`](https://docs.rs/fixed-buffer/latest/fixed_buffer/struct.ReadWriteTake.html).
- v0.1.7 - Add [`FixedBuf::escape_ascii`](https://docs.rs/fixed-buffer/latest/fixed_buffer/struct.FixedBuf.html#method.escape_ascii).
- v0.1.6 - Add [`filled`](https://docs.rs/fixed-buffer/latest/fixed_buffer/struct.FixedBuf.html#method.filled)
  constructor.
- v0.1.5 - Change [`read_delimited`](https://docs.rs/fixed-buffer/latest/fixed_buffer/struct.FixedBuf.html#method.read_delimited)
  to return `Option<&[u8]>`, for clean EOF handling.
- v0.1.4 - Add [`clear()`](https://docs.rs/fixed-buffer/latest/fixed_buffer/struct.FixedBuf.html#method.clear).
- v0.1.3
  - Thanks to [freax13](https://gitlab.com/Freax13) for these changes:
    - Support any buffer size.  Now you can make `FixedBuf<[u8; 42]>`.
    - Support any `AsRef<[u8]> + AsMut<[u8]>` value for internal memory:
      - `[u8; N]`
      - `Box<[u8; N]>`
      - `&mut [u8]`
      - `Vec<u8>`
  - Renamed `new_with_mem` to `new`.
    Use `FixedBuf::default()` to construct any `FixedBuf<T: Default>`, which includes
    [arrays of sizes up to 32](https://doc.rust-lang.org/std/primitive.array.html).
- v0.1.2 - Updated documentation.
- v0.1.1 - First published version

## TO DO
- Change deframer function type to allow consuming bytes without returning a block.
- Implement async-std read & write traits
- Add an `frame_copy_iter` function.
  Because of borrowing rules, this function must return non-borrowed (allocated and copied) data.
- Set up CI on:
  - DONE - Linux x86 64-bit
  - [macOS](https://gitlab.com/gitlab-org/gitlab/-/issues/269756)
  - [Windows](https://about.gitlab.com/blog/2020/01/21/windows-shared-runner-beta/)
  - https://crate-ci.github.io/pr/testing.html#travisci
  - Linux ARM 64-bit (Raspberry Pi 3 and newer)
  - Linux ARM 32-bit (Raspberry Pi 2)
  - RISCV & ESP32 firmware?
- DONE - Switch to const generics once they are stable:
  - https://github.com/rust-lang/rust/issues/44580
  - https://stackoverflow.com/a/56543462
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
- DONE - Publish to creates.io
- DONE - Read through https://crate-ci.github.io/index.html
- DONE - Get a code review from an experienced rustacean
- DONE - Add and update a changelog
  - Update it manually
  - https://crate-ci.github.io/release/changelog.html

## Release Process
1. Edit `Cargo.toml` and bump version number.
1. Run `../release.sh`

License: Apache-2.0

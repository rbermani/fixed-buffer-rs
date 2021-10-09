# fixed-buffer-tokio

[![crates.io version](https://img.shields.io/crates/v/fixed-buffer-tokio.svg)](https://crates.io/crates/fixed-buffer-tokio)
[![license: Apache 2.0](https://gitlab.com/leonhard-llc/fixed-buffer-rs/-/raw/main/license-apache-2.0.svg)](http://www.apache.org/licenses/LICENSE-2.0)
[![unsafe forbidden](https://gitlab.com/leonhard-llc/fixed-buffer-rs/-/raw/main/unsafe-forbidden-success.svg)](https://github.com/rust-secure-code/safety-dance/)
[![pipeline status](https://gitlab.com/leonhard-llc/fixed-buffer-rs/badges/main/pipeline.svg)](https://gitlab.com/leonhard-llc/fixed-buffer-rs/-/pipelines)

This is a Rust library with fixed-size buffers,
useful for network protocol parsers and file parsers.

This is the tokio async version of [`fixed-buffer`](https://crates.io/crates/fixed-buffer).

## Features
- Write bytes to the buffer and read them back
- Lives on the stack
- Does not allocate memory
- Use it to read a stream, search for a delimiter,
  and save leftover bytes for the next read.
- Easy to learn & use.  Easy to maintain code that uses it.
- Depends only on
  [`std`](https://doc.rust-lang.org/stable/std/),
  [`tokio 0.3`](https://crates.io/crates/tokio), and
  [`fixed-buffer`](https://crates.io/crates/fixed-buffer).
- Works with Tokio 0.3 and Rust `latest`, `beta`, and `nightly`
- No macros
- Good test coverage (98%)

## Documentation
https://docs.rs/fixed-buffer-tokio

## Examples
For a complete example, see
[`tests/server.rs`](https://gitlab.com/leonhard-llc/fixed-buffer-rs/-/blob/main/fixed-buffer-tokio/tests/server.rs).

## Alternatives
- [tokio::io::BufReader](https://docs.rs/tokio/latest/tokio/io/struct.BufReader.html)
- [tokio::io::BufWriter](https://docs.rs/tokio/latest/tokio/io/struct.BufWriter.html)

## Changelog
- v0.3.0 - Breaking API changes:
  - Change type parameter to const buffer size. Example: `FixedBuf<1024>`.
  - Remove `new` arg.
  - Remove `capacity`.
  - Change `writable` return type to `&mut [u8]`.
- v0.1.1 - Add badges to readme
- v0.1.0 - First published version

## Release Process
1. Edit `Cargo.toml` and bump version number.
1. Run `../release.sh`

License: Apache-2.0

# fixed-buffer-tokio

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

## Release Process
1. Edit `Cargo.toml` and bump version number.
1. Run `../release.sh`

## Changelog
- v0.1.1 - First published version


License: Apache-2.0

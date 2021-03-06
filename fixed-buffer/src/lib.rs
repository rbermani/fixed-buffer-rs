//! [![crates.io version](https://img.shields.io/crates/v/fixed-buffer.svg)](https://crates.io/crates/fixed-buffer)
//! [![license: Apache 2.0](https://gitlab.com/leonhard-llc/fixed-buffer-rs/-/raw/main/license-apache-2.0.svg)](http://www.apache.org/licenses/LICENSE-2.0)
//! [![unsafe forbidden](https://gitlab.com/leonhard-llc/fixed-buffer-rs/-/raw/main/unsafe-forbidden-success.svg)](https://github.com/rust-secure-code/safety-dance/)
//! [![pipeline status](https://gitlab.com/leonhard-llc/fixed-buffer-rs/badges/main/pipeline.svg)](https://gitlab.com/leonhard-llc/fixed-buffer-rs/-/pipelines)
//!
//! This is a Rust library with fixed-size buffers,
//! useful for network protocol parsers and file parsers.
//!
//! # Features
//! - No `unsafe`
//! - Depends only on `std`
//! - Write bytes to the buffer and read them back
//! - Lives on the stack
//! - Does not allocate memory
//! - Use it to read a stream, search for a delimiter,
//!   and save leftover bytes for the next read.
//! - Easy to learn & use.  Easy to maintain code that uses it.
//! - Works with Rust `latest`, `beta`, and `nightly`
//! - No macros
//! - Good test coverage (99%)
//! - [fixed_buffer_tokio](https://crates.io/crates/fixed-buffer-tokio) adds async functions
//!
//! # Limitations
//! - Not a circular buffer.
//!   You can call `shift()` periodically
//!   to move unread bytes to the front of the buffer.
//!
//! # Documentation
//! https://docs.rs/fixed-buffer
//!
//! # Examples
//! Read and handle requests from a remote client:
//! ```rust
//! # struct Request {}
//! # impl Request {
//! #     pub fn parse(line_bytes: &[u8]
//! #     ) -> Result<Request, std::io::Error> {
//! #         Ok(Request{})
//! #     }
//! # }
//! use fixed_buffer::{
//!     deframe_line, FixedBuf, ReadWriteChain};
//! use std::io::{Error, Read, Write};
//! use std::net::TcpStream;
//!
//! fn handle_request<RW: Read + Write>(
//!     reader_writer: &mut RW,
//!     request: Request,
//! ) -> Result<(), Error> {
//!     // ...
//!     Ok(())
//! }
//!
//! fn handle_conn(mut tcp_stream: TcpStream
//! ) -> Result<(), Error> {
//!     let mut buf: FixedBuf<4096> = FixedBuf::new();
//!     loop {
//!         // Read a line
//!         // and leave leftover bytes in `buf`.
//!         let line_bytes = match buf.read_frame(
//!             &mut tcp_stream, deframe_line)? {
//!                 Some(line_bytes) => line_bytes,
//!                 None => return Ok(()),
//!             };
//!         let request = Request::parse(line_bytes)?;
//!         // Read any request payload
//!         // from `buf` + `TcpStream`.
//!         let mut reader_writer = ReadWriteChain::new(
//!             &mut buf, &mut tcp_stream);
//!         handle_request(&mut reader_writer, request)?;
//!     }
//! }
//! ```
//! For a complete example, see
//! [`tests/server.rs`](https://gitlab.com/leonhard-llc/fixed-buffer-rs/-/blob/main/fixed-buffer/tests/server.rs).
//!
//! Read and process records:
//! ```rust
//! use fixed_buffer::FixedBuf;
//! use std::io::{Error, ErrorKind, Read};
//! use std::net::TcpStream;
//!
//! fn try_process_record(b: &[u8]) -> Result<usize, Error> {
//!     if b.len() < 2 {
//!         return Ok(0);
//!     }
//!     if b.starts_with("ab".as_bytes()) {
//!         println!("found record");
//!         Ok(2)
//!     } else {
//!         Err(Error::new(ErrorKind::InvalidData, "bad record"))
//!     }
//! }
//!
//! fn read_and_process<R: Read>(mut input: R)
//!     -> Result<(), Error> {
//!     let mut buf: FixedBuf<1024> = FixedBuf::new();
//!     loop {
//!         // Read a chunk into the buffer.
//!         if buf.copy_once_from(&mut input)? == 0 {
//!             return if buf.len() == 0 {
//!                 // EOF at record boundary
//!                 Ok(())
//!             } else {
//!                 // EOF in the middle of a record
//!                 Err(Error::from(
//!                     ErrorKind::UnexpectedEof))
//!             };
//!         }
//!         // Process records in the buffer.
//!         loop {
//!             let bytes_read =
//!                 try_process_record(buf.readable())?;
//!             if bytes_read == 0 {
//!                 break;
//!             }
//!             buf.read_bytes(bytes_read);
//!         }
//!         // Shift data in the buffer to free up
//!         // space at the end for writing.
//!         buf.shift();
//!     }
//! }
//! #
//! # fn main() {
//! #     read_and_process(std::io::Cursor::new(b"abab")).unwrap();
//! #     read_and_process(std::io::Cursor::new(b"ababc")).unwrap_err();
//! # }
//! ```
//!
//! The [`filled`](https://docs.rs/fixed-buffer/latest/fixed_buffer/struct.FixedBuf.html#method.filled)
//! constructor is useful in tests.
//!
//! # Alternatives
//! - [bytes](https://docs.rs/bytes/0.5.6/bytes/index.html)
//! - [buf_redux](https://crates.io/crates/buf_redux), circular buffer support
//! - [std::io::BufReader](https://doc.rust-lang.org/std/io/struct.BufReader.html)
//! - [std::io::BufWriter](https://doc.rust-lang.org/std/io/struct.BufWriter.html)
//! - [static-buffer](https://crates.io/crates/static-buffer), updated in 2016
//! - [block-buffer](https://crates.io/crates/block-buffer), for processing fixed-length blocks of data
//! - [arrayvec](https://crates.io/crates/arrayvec), vector with fixed capacity.
//!
//! # Changelog
//! - v0.3.0 - Breaking API changes:
//!   - Change type parameter to const buffer size. Example: `FixedBuf<1024>`.
//!   - Remove `new` arg.
//!   - Remove `capacity`.
//!   - Remove `Copy` impl.
//!   - Change `writable` return type to `&mut [u8]`.
//! - v0.2.3
//!   - Add
//!     [`read_byte`](https://docs.rs/fixed-buffer/latest/fixed_buffer/struct.FixedBuf.html#method.read_byte),
//!     [`try_read_byte`](https://docs.rs/fixed-buffer/latest/fixed_buffer/struct.FixedBuf.html#method.try_read_byte),
//!     [`try_read_bytes`](https://docs.rs/fixed-buffer/latest/fixed_buffer/struct.FixedBuf.html#method.try_read_bytes),
//!     [`try_read_exact`](https://docs.rs/fixed-buffer/latest/fixed_buffer/struct.FixedBuf.html#method.try_read_exact),
//!     [`try_parse`](https://docs.rs/fixed-buffer/latest/fixed_buffer/struct.FixedBuf.html#method.try_parse).
//!   - Implement [`UnwindSafe`](https://doc.rust-lang.org/std/panic/trait.UnwindSafe.html)
//! - v0.2.2 - Add badges to readme
//! - v0.2.1 - Add
//!   [`deframe`](https://docs.rs/fixed-buffer/latest/fixed_buffer/struct.FixedBuf.html#method.deframe)
//!   and
//!   [`mem`](https://docs.rs/fixed-buffer/latest/fixed_buffer/struct.FixedBuf.html#method.mem),
//!   needed by `AsyncFixedBuf::read_frame`.
//! - v0.2.0
//!   - Move tokio support to [fixed_buffer_tokio](https://crates.io/crates/fixed-buffer-tokio).
//!   - Add
//!     [`copy_once_from`](https://docs.rs/fixed-buffer/latest/fixed_buffer/struct.FixedBuf.html#method.copy_once_from),
//!     [`read_block`](https://docs.rs/fixed-buffer/latest/fixed_buffer/struct.FixedBuf.html#method.read_block),
//!     [`ReadWriteChain`](https://docs.rs/fixed-buffer/latest/fixed_buffer/struct.ReadWriteChain.html),
//!     and
//!     [`ReadWriteTake`](https://docs.rs/fixed-buffer/latest/fixed_buffer/struct.ReadWriteTake.html).
//! - v0.1.7 - Add [`FixedBuf::escape_ascii`](https://docs.rs/fixed-buffer/latest/fixed_buffer/struct.FixedBuf.html#method.escape_ascii).
//! - v0.1.6 - Add [`filled`](https://docs.rs/fixed-buffer/latest/fixed_buffer/struct.FixedBuf.html#method.filled)
//!   constructor.
//! - v0.1.5 - Change [`read_delimited`](https://docs.rs/fixed-buffer/latest/fixed_buffer/struct.FixedBuf.html#method.read_delimited)
//!   to return `Option<&[u8]>`, for clean EOF handling.
//! - v0.1.4 - Add [`clear()`](https://docs.rs/fixed-buffer/latest/fixed_buffer/struct.FixedBuf.html#method.clear).
//! - v0.1.3
//!   - Thanks to [freax13](https://gitlab.com/Freax13) for these changes:
//!     - Support any buffer size.  Now you can make `FixedBuf<[u8; 42]>`.
//!     - Support any `AsRef<[u8]> + AsMut<[u8]>` value for internal memory:
//!       - `[u8; N]`
//!       - `Box<[u8; N]>`
//!       - `&mut [u8]`
//!       - `Vec<u8>`
//!   - Renamed `new_with_mem` to `new`.
//!     Use `FixedBuf::default()` to construct any `FixedBuf<T: Default>`, which includes
//!     [arrays of sizes up to 32](https://doc.rust-lang.org/std/primitive.array.html).
//! - v0.1.2 - Updated documentation.
//! - v0.1.1 - First published version
//!
//! # TO DO
//! - Change deframer function type to allow consuming bytes without returning a block.
//! - Implement async-std read & write traits
//! - Add an `frame_copy_iter` function.
//!   Because of borrowing rules, this function must return non-borrowed (allocated and copied) data.
//! - Set up CI on:
//!   - DONE - Linux x86 64-bit
//!   - [macOS](https://gitlab.com/gitlab-org/gitlab/-/issues/269756)
//!   - [Windows](https://about.gitlab.com/blog/2020/01/21/windows-shared-runner-beta/)
//!   - https://crate-ci.github.io/pr/testing.html#travisci
//!   - Linux ARM 64-bit (Raspberry Pi 3 and newer)
//!   - Linux ARM 32-bit (Raspberry Pi 2)
//!   - RISCV & ESP32 firmware?
//! - DONE - Switch to const generics once they are stable:
//!   - https://github.com/rust-lang/rust/issues/44580
//!   - https://stackoverflow.com/a/56543462
//! - DONE - Try to make this crate comply with the [Rust API Guidelines](https://rust-lang.github.io/api-guidelines/).
//! - DONE - Find out how to include Readme.md info in the crate's docs.
//! - DONE - Make the repo public
//! - DONE - Set up continuous integration tests and banner.
//!   - https://github.com/actions-rs/example
//!   - https://alican.codes/rust-github-actions/
//! - DONE - Add some documentation tests
//!   - https://doc.rust-lang.org/rustdoc/documentation-tests.html
//!   - https://doc.rust-lang.org/stable/rust-by-example/testing/doc_testing.html
//! - DONE - Set up public repository on Gitlab.com
//!   - https://gitlab.com/mattdark/firebase-example/blob/master/.gitlab-ci.yml
//!   - https://medium.com/astraol/optimizing-ci-cd-pipeline-for-rust-projects-gitlab-docker-98df64ae3bc4
//!   - https://hub.docker.com/_/rust
//! - DONE - Publish to creates.io
//! - DONE - Read through https://crate-ci.github.io/index.html
//! - DONE - Get a code review from an experienced rustacean
//! - DONE - Add and update a changelog
//!   - Update it manually
//!   - https://crate-ci.github.io/release/changelog.html
//!
//! # Release Process
//! 1. Edit `Cargo.toml` and bump version number.
//! 1. Run `../release.sh`
#![forbid(unsafe_code)]

mod escape_ascii;
pub use escape_ascii::escape_ascii;

mod deframe_crlf;
pub use deframe_crlf::deframe_crlf;

mod deframe_line;
pub use deframe_line::deframe_line;

mod deframe_null;
pub use deframe_null::deframe_null;

mod read_write_chain;
pub use read_write_chain::*;

mod read_write_take;
pub use read_write_take::*;

#[cfg(test)]
mod test_utils;
#[cfg(test)]
pub use test_utils::*;

#[derive(Copy, Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct NotEnoughSpaceError {}
impl From<NotEnoughSpaceError> for std::io::Error {
    fn from(_: NotEnoughSpaceError) -> Self {
        std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            "not enough space in buffer",
        )
    }
}

#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct MalformedInputError(String);
impl MalformedInputError {
    pub fn new(msg: String) -> Self {
        Self(msg)
    }
}
impl From<MalformedInputError> for std::io::Error {
    fn from(e: MalformedInputError) -> Self {
        std::io::Error::new(std::io::ErrorKind::InvalidData, format!("{:?}", e))
    }
}

/// FixedBuf is a fixed-length byte buffer.
/// You can write bytes to it and then read them back.
///
/// It is not a circular buffer.  Call [`shift`](#method.shift) periodically to
/// move unread bytes to the front of the buffer.
#[derive(Copy, Clone, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct FixedBuf<const SIZE: usize> {
    mem: [u8; SIZE],
    read_index: usize,
    write_index: usize,
}

impl<const SIZE: usize> std::panic::UnwindSafe for FixedBuf<SIZE> {}

impl<const SIZE: usize> FixedBuf<SIZE> {
    /// Makes a new empty buffer with space for `SIZE` bytes.
    ///
    /// Be careful of stack overflows!
    pub const fn new() -> Self {
        Self {
            mem: [0_u8; SIZE],
            write_index: 0,
            read_index: 0,
        }
    }

    /// Makes a new empty buffer.
    ///
    /// Consumes `mem` and uses it as the internal memory array.
    /// ```
    pub fn empty(mem: [u8; SIZE]) -> Self {
        Self {
            mem,
            read_index: 0,
            write_index: 0,
        }
    }

    /// Makes a new full buffer containing the bytes in `mem`.
    /// Reading the buffer will return the bytes in `mem`.
    ///
    /// Consumes `mem` and uses it as the internal memory array.
    /// ```
    pub fn filled(mem: [u8; SIZE]) -> Self {
        Self {
            mem,
            read_index: 0,
            write_index: SIZE,
        }
    }

    /// Drops the struct and returns its internal array.
    pub fn into_inner(self) -> [u8; SIZE] {
        self.mem
    }

    /// Returns the number of unread bytes in the buffer.
    ///
    /// Example:
    /// ```
    /// # use fixed_buffer::FixedBuf;
    /// let mut buf: FixedBuf<16> = FixedBuf::new();
    /// assert_eq!(0, buf.len());
    /// buf.write_str("abc");
    /// assert_eq!(3, buf.len());
    /// buf.read_bytes(2);
    /// assert_eq!(1, buf.len());
    /// buf.shift();
    /// assert_eq!(1, buf.len());
    /// buf.read_all();
    /// assert_eq!(0, buf.len());
    /// ```
    pub fn len(&self) -> usize {
        self.write_index - self.read_index
    }

    /// Returns true if there are unread bytes in the buffer.
    ///
    /// Example:
    /// ```
    /// # use fixed_buffer::FixedBuf;
    /// let mut buf: FixedBuf<16> = FixedBuf::new();
    /// assert!(buf.is_empty());
    /// buf.write_str("abc").unwrap();
    /// assert!(!buf.is_empty());
    /// buf.read_all();
    /// assert!(buf.is_empty());
    /// ```
    pub fn is_empty(&self) -> bool {
        self.write_index == self.read_index
    }

    /// Discards all data in the buffer.
    pub fn clear(&mut self) {
        self.read_index = 0;
        self.write_index = 0;
    }

    /// Copies all readable bytes to a string.
    /// Includes printable ASCII characters as-is.
    /// Converts non-printable characters to strings like "\n" and "\x19".
    ///
    /// Uses
    /// [`core::ascii::escape_default`](https://doc.rust-lang.org/core/ascii/fn.escape_default.html
    /// internally to escape each byte.
    ///
    /// This function is useful for printing byte slices to logs and comparing byte slices in tests.
    ///
    /// Example test:
    /// ```
    /// use fixed_buffer::FixedBuf;
    /// let mut buf: FixedBuf<8> = FixedBuf::new();
    /// buf.write_str("abc");
    /// buf.write_str("???");
    /// assert_eq!("abc\\xe2\\x82\\xac", buf.escape_ascii());
    /// ```
    pub fn escape_ascii(&self) -> String {
        escape_ascii(self.readable())
    }

    /// Borrows the entire internal memory buffer.
    /// This is a low-level function.
    pub fn mem(&self) -> &[u8] {
        self.mem.as_ref()
    }

    /// Returns the slice of readable bytes in the buffer.
    /// After processing some bytes from the front of the slice,
    /// call [`read`](#method.read) to consume the bytes.
    ///
    /// This is a low-level method.
    /// You probably want to use
    /// [`std::io::Read::read`](https://doc.rust-lang.org/std/io/trait.Read.html#tymethod.read)
    /// or
    /// [`tokio::io::AsyncReadExt::read`](https://docs.rs/tokio/0.3.0/tokio/io/trait.AsyncReadExt.html#method.reade)
    /// , implemented for FixedBuffer in
    /// [`fixed_buffer_tokio::AsyncReadExt`](https://docs.rs/fixed-buffer-tokio/latest/fixed_buffer_tokio/trait.AsyncReadExt.html).
    pub fn readable(&self) -> &[u8] {
        &self.mem.as_ref()[self.read_index..self.write_index]
    }

    /// Reads a single byte from the buffer.
    ///
    /// Panics if the buffer is empty.
    pub fn read_byte(&mut self) -> u8 {
        self.read_bytes(1)[0]
    }

    /// Reads a single byte from the buffer.
    ///
    /// Returns `None` if the buffer is empty.
    pub fn try_read_byte(&mut self) -> Option<u8> {
        if self.is_empty() {
            None
        } else {
            Some(self.read_byte())
        }
    }

    /// Reads bytes from the buffer.
    ///
    /// Panics if the buffer does not contain enough bytes.
    pub fn read_bytes(&mut self, num_bytes: usize) -> &[u8] {
        let new_read_index = self.read_index + num_bytes;
        assert!(new_read_index <= self.write_index, "read would underflow");
        let old_read_index = self.read_index;
        // We update `read_index` after any possible panic.
        // This keeps the struct consistent even when a panic happens.
        // This complies with the contract of std::panic::UnwindSafe.
        self.read_index = new_read_index;
        if self.read_index == self.write_index {
            // All data has been read.  Reset the buffer.
            self.write_index = 0;
            self.read_index = 0;
        }
        &self.mem.as_ref()[old_read_index..new_read_index]
    }

    /// Reads bytes from the buffer.
    ///
    /// Returns `None` if the buffer does not contain `num_bytes` bytes.
    pub fn try_read_bytes(&mut self, num_bytes: usize) -> Option<&[u8]> {
        if self.len() < num_bytes {
            None
        } else {
            Some(self.read_bytes(num_bytes))
        }
    }

    /// Reads all the bytes from the buffer.
    ///
    /// The buffer becomes empty and subsequent writes can fill the whole buffer.
    pub fn read_all(&mut self) -> &[u8] {
        self.read_bytes(self.len())
    }

    /// Reads bytes from the buffer and copies them into `dest`.
    ///
    /// Returns the number of bytes copied.
    ///
    /// Returns `0` when the buffer is empty or `dest` is zero-length.
    pub fn read_and_copy_bytes(&mut self, dest: &mut [u8]) -> usize {
        let readable = self.readable();
        let len = core::cmp::min(dest.len(), readable.len());
        if len == 0 {
            return 0;
        }
        let src = &readable[..len];
        let copy_dest = &mut dest[..len];
        copy_dest.copy_from_slice(src);
        self.read_bytes(len);
        len
    }

    /// Reads byte from the buffer and copies them into `dest`, filling it,
    /// and returns `()`.
    ///
    /// Returns `None` if the buffer does not contain enough bytes tp fill `dest`.
    ///
    /// Returns `()` if `dest` is zero-length.
    pub fn try_read_exact(&mut self, dest: &mut [u8]) -> Option<()> {
        if self.len() < dest.len() {
            return None;
        }
        self.read_and_copy_bytes(dest);
        Some(())
    }

    /// Try to parse the buffer contents with `f`.
    ///
    /// `f` must return `None` to indicate that the buffer
    /// contains an incomplete data record.
    /// When `try_parse` returns None, the caller should add more data to the
    /// buffer and call `try_parse` again.
    ///
    /// Undoes any reads that `f` made to the buffer if it returns `None`.
    ///
    /// `f` should not write to the buffer.
    /// If `f` writes to the buffer and then returns `None`,
    /// the buffer's contents will become corrupted.
    ///
    /// # Example
    /// ```
    /// # use fixed_buffer::FixedBuf;
    /// #[derive(Debug)]
    /// enum ReadError {
    ///     BufferFull,
    ///     EOF,
    ///     TooLong,
    ///     NotUtf8,
    /// }
    /// # fn main() -> Result<(), ReadError> {
    /// # let mut input: Vec<&[u8]> = vec![
    /// #     &[0_u8, 1, b'a', 3, b'a'][..],
    /// #     &[b'b', b'c',
    /// #         // 17, // TooLong
    /// #         // 1, // EOF
    /// #     ][..],
    /// #     // b"\x101234567890ABCDEF" // BufferFull
    /// # ];
    /// # let mut read_input = |buf: &mut FixedBuf<16>| {
    /// #     if input.is_empty() {
    /// #         return Err(ReadError::EOF);
    /// #     }
    /// #     buf.shift();
    /// #     let writable = buf.writable();
    /// #     let chunk = input.remove(0);
    /// #     if chunk.len() > writable.len() {
    /// #         return Err(ReadError::BufferFull);
    /// #     }
    /// #     let dest = &mut writable[..chunk.len()];
    /// #     dest.copy_from_slice(&chunk);
    /// #     buf.wrote(chunk.len());
    /// #     Ok(())
    /// # };
    /// # let mut output: Vec<String> = Vec::new();
    /// # let mut process_record = |record| { output.push(record); Ok(()) };
    ///
    /// fn parse_record<const SIZE: usize>(buf: &mut FixedBuf<SIZE>)
    /// -> Option<Result<String, ReadError>>
    /// {
    ///     let len = buf.try_read_byte()? as usize;
    ///     if len > SIZE - 1 {
    ///         return Some(Err(ReadError::TooLong));
    ///     }
    ///     let bytes = buf.try_read_bytes(len)?.to_vec();
    ///     Some(String::from_utf8(bytes)
    ///         .map_err(|_| ReadError::NotUtf8))
    /// }
    ///
    /// let mut buf: FixedBuf<16> = FixedBuf::new();
    /// loop {
    ///     // Try reading bytes into the buffer.
    ///     match read_input(&mut buf) {
    ///         Err(ReadError::EOF) if buf.is_empty() => break,
    ///         other => other?,
    ///     }
    ///     // Try parsing bytes into records.
    ///     loop {
    ///         if let Some(result) = buf.try_parse(parse_record) {
    ///             let record = result?;
    ///             process_record(record)?;
    ///         } else {
    ///             // Stop parsing and try reading more bytes.
    ///             break;
    ///         }
    ///     }
    /// }
    /// # assert_eq!(vec!["", "a", "abc"], output);
    /// # Ok(())
    /// # }
    /// ```
    pub fn try_parse<R, F>(&mut self, f: F) -> Option<R>
    where
        F: FnOnce(&mut FixedBuf<SIZE>) -> Option<R>,
    {
        let original_read_index = self.read_index;
        let original_write_index = self.write_index;
        if let Some(value) = f(self) {
            Some(value)
        } else {
            self.read_index = original_read_index;
            self.write_index = original_write_index;
            None
        }
    }

    /// Reads from `reader` once and writes the data into the buffer.
    ///
    /// Returns [`InvalidData`](std::io::ErrorKind::InvalidData)
    /// if there is no empty space in the buffer.
    /// See [`shift`](#method.shift).
    pub fn copy_once_from<R: std::io::Read>(
        &mut self,
        reader: &mut R,
    ) -> Result<usize, std::io::Error> {
        let writable = self.writable();
        if writable.is_empty() {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "no empty space in buffer",
            ));
        };
        let num_read = reader.read(writable)?;
        self.wrote(num_read);
        Ok(num_read)
    }

    /// Writes `s` into the buffer, after any unread bytes.
    ///
    /// Returns `Err` if the buffer doesn't have enough free space at the end
    /// for the whole string.
    ///
    /// See [`shift`](#method.shift).
    ///
    /// Example:
    /// ```
    /// # use fixed_buffer::{escape_ascii, FixedBuf};
    /// let mut buf: FixedBuf<8> = FixedBuf::new();
    /// buf.write_str("123").unwrap();
    /// buf.write_str("456").unwrap();
    /// assert_eq!("123456", buf.escape_ascii());
    /// buf.write_str("78").unwrap();
    /// buf.write_str("9").unwrap_err();  // End of buffer is full.
    /// ```
    pub fn write_str(&mut self, s: &str) -> Result<(), NotEnoughSpaceError> {
        self.write_bytes(s.as_bytes()).map(|_| ())
    }

    /// Tries to write `data` into the buffer, after any unread bytes.
    ///
    /// Returns `Ok(data.len())` if it wrote all of the bytes.
    ///
    /// Returns `NotEnoughSpaceError`
    /// if the buffer doesn't have enough free space at the end for all of the bytes.
    ///
    /// See [`shift`](#method.shift).
    ///
    /// Example:
    /// ```
    /// # use fixed_buffer::{escape_ascii, FixedBuf};
    /// let mut buf: FixedBuf<8> = FixedBuf::new();
    /// assert_eq!(3, buf.write_bytes("123".as_bytes()).unwrap());
    /// assert_eq!(3, buf.write_bytes("456".as_bytes()).unwrap());
    /// assert_eq!("123456", buf.escape_ascii());
    /// assert_eq!(2, buf.write_bytes("78".as_bytes()).unwrap());  // Fills buffer.
    /// buf.write_bytes("9".as_bytes()).unwrap_err();  // Error, buffer is full.
    /// ```
    pub fn write_bytes(&mut self, data: &[u8]) -> core::result::Result<usize, NotEnoughSpaceError> {
        let writable = self.writable();
        if writable.len() < data.len() {
            return Err(NotEnoughSpaceError {});
        }
        let dest = &mut writable[..data.len()];
        dest.copy_from_slice(data);
        self.wrote(data.len());
        Ok(data.len())
    }

    /// Returns the writable part of the buffer.
    ///
    /// To use this, first modify bytes at the beginning of the slice.
    /// Then call [`wrote(usize)`](#method.wrote)
    /// to commit those bytes into the buffer and make them available for reading.
    ///
    /// Returns an empty slice when the end of the buffer is full.
    ///
    /// See [`shift`](#method.shift).
    ///
    /// This is a low-level method.
    /// You probably want to use
    /// [`std::io::Write::write`](https://doc.rust-lang.org/std/io/trait.Write.html#tymethod.write)
    /// or
    /// [`tokio::io::AsyncWriteExt::write`](https://docs.rs/tokio/0.3.0/tokio/io/trait.AsyncWriteExt.html#method.write),
    /// implemented for FixedBuffer in
    /// [`fixed_buffer_tokio::AsyncWriteExt`](https://docs.rs/fixed-buffer-tokio/latest/fixed_buffer_tokio/trait.AsyncWriteExt.html).
    ///
    /// Example:
    /// ```
    /// # use fixed_buffer::{escape_ascii, FixedBuf};
    /// let mut buf: FixedBuf<8> = FixedBuf::new();
    /// buf.writable()[0] = 'a' as u8;
    /// buf.writable()[1] = 'b' as u8;
    /// buf.writable()[2] = 'c' as u8;
    /// buf.wrote(3);
    /// assert_eq!("abc", buf.escape_ascii());
    /// ```
    pub fn writable(&mut self) -> &mut [u8] {
        &mut self.mem.as_mut()[self.write_index..]
    }

    /// Commits bytes into the buffer.
    /// Call this after writing to the front of the
    /// [`writable`](#method.writable) slice.
    ///
    /// This is a low-level method.
    ///
    /// Panics when there is not `num_bytes` free at the end of the buffer.
    ///
    /// See [`writable()`](#method.writable).
    pub fn wrote(&mut self, num_bytes: usize) {
        if num_bytes == 0 {
            return;
        }
        let new_write_index = self.write_index + num_bytes;
        assert!(
            new_write_index <= self.mem.as_mut().len(),
            "write would overflow"
        );
        self.write_index = new_write_index;
    }

    /// Recovers buffer space.
    ///
    /// The buffer is not circular.
    /// After you read bytes, the space at the beginning of the buffer is unused.
    /// Call this method to move unread data to the beginning of the buffer and recover the space.
    /// This makes the free space available for writes, which go at the end of the buffer.
    pub fn shift(&mut self) {
        if self.read_index == 0 {
            return;
        }
        // As long as read_bytes performs this check and is the only way to
        // advance read_index, this block can never execute.
        // if self.read_index == self.write_index {
        //     self.write_index = 0;
        //     self.read_index = 0;
        //     return;
        // }
        self.mem
            .as_mut()
            .copy_within(self.read_index..self.write_index, 0);
        self.write_index -= self.read_index;
        self.read_index = 0;
    }

    /// This is a low-level function.
    /// Use [`read_frame`](#method.read_frame) instead.
    ///
    /// Calls `deframer_fn` to check if the buffer contains a complete frame.
    /// Consumes the frame bytes from the buffer
    /// and returns the range of the frame's contents in the internal memory.
    ///
    /// Use [`mem`](#method.mem) to immutably borrow the internal memory and
    /// construct the slice with `&buf.mem()[range]`.
    /// This is necessary because `deframe` borrows `self` mutably but
    /// `read_frame` needs to borrow it immutably and return a slice.
    ///
    /// Returns `None` if the buffer is empty or contains an incomplete frame.
    ///
    /// Returns [`InvalidData`](std::io::ErrorKind::InvalidData)
    /// when `deframer_fn` returns an error.
    pub fn deframe<F>(
        &mut self,
        deframer_fn: F,
    ) -> Result<Option<core::ops::Range<usize>>, std::io::Error>
    where
        F: Fn(&[u8]) -> Result<Option<(core::ops::Range<usize>, usize)>, MalformedInputError>,
    {
        if self.is_empty() {
            return Ok(None);
        }
        match deframer_fn(self.readable())? {
            Some((data_range, block_len)) => {
                let mem_start = self.read_index + data_range.start;
                let mem_end = self.read_index + data_range.end;
                self.read_bytes(block_len);
                Ok(Some(mem_start..mem_end))
            }
            None => Ok(None),
        }
    }

    /// Reads from `reader` into the buffer.
    ///
    /// After each read, calls `deframer_fn`
    /// to check if the buffer now contains a complete frame.
    /// Consumes the frame bytes from the buffer
    /// and returns a slice with the frame contents.
    ///
    /// Returns `None` when `reader` reaches EOF and the buffer is empty.
    ///
    /// Returns [`UnexpectedEof`](std::io::ErrorKind::UnexpectedEof)
    /// when `reader` reaches EOF and the buffer contains an incomplete frame.
    ///
    /// Returns [`InvalidData`](std::io::ErrorKind::InvalidData)
    /// when `deframer_fn` returns an error or the buffer fills up.
    ///
    /// Calls [`shift`](#method.shift) before reading.
    ///
    /// Provided deframer functions:
    /// - [deframe_line](https://docs.rs/fixed-buffer/latest/fixed_buffer/fn.deframe_line.html)
    /// - [deframe_crlf](https://docs.rs/fixed-buffer/latest/fixed_buffer/fn.deframe_crlf.html)
    ///
    /// # Example
    /// ```
    /// # use fixed_buffer::{escape_ascii, FixedBuf, deframe_line};
    /// let mut buf: FixedBuf<32> = FixedBuf::new();
    /// let mut input = std::io::Cursor::new(b"aaa\r\nbbb\n\nccc\n");
    /// # let mut output: Vec<String> = Vec::new();
    /// loop {
    ///   if let Some(line) =
    ///       buf.read_frame(&mut input, deframe_line).unwrap() {
    ///     println!("{}", escape_ascii(line));
    /// #   output.push(escape_ascii(line));
    ///   } else {
    ///     // EOF.
    ///     break;
    ///   }
    /// }
    /// // Prints:
    /// // aaa
    /// // bbb
    /// //
    /// // ccc
    /// # assert_eq!(
    /// #     vec!["aaa".to_string(), "bbb".to_string(),"".to_string(), "ccc".to_string()],
    /// #     output
    /// # );
    /// ```
    ///
    /// # Deframer Function `deframe_fn`
    /// Checks if `data` contains an entire frame.
    ///
    /// Never panics.
    ///
    /// Returns `Err(MalformedInputError)` if `data` contains a malformed frame.
    ///
    /// Returns `Ok(None)` if `data` contains an incomplete frame.
    /// The caller will read more data and call `deframe` again.
    ///
    /// Returns `Ok((range, len))` when `data` contains a complete well-formed
    /// frame of length `len` and contents `&data[range]`.
    ///
    /// A frame is a sequence of bytes containing a payload
    /// and metadata prefix or suffix bytes.
    /// The metadata define where the payload starts and ends.
    /// Popular frame protocols include line-delimiting
    /// ([CSV](https://en.wikipedia.org/wiki/Comma-separated_values)),
    /// hexadecimal length prefix
    /// ([HTTP chunked transfer encoding](https://tools.ietf.org/html/rfc7230#section-4.1)),
    /// and binary length prefix
    /// ([TLS](https://tools.ietf.org/html/rfc5246#section-6.2.1)).
    ///
    /// If the caller removes `len` bytes from the front of `data`, then
    /// `data` should start with the next frame, ready to call `deframe` again.
    ///
    /// A line-delimited deframer returns `range`
    /// so that `&data[range]` contains the entire line without the
    /// final `b'\n'` or `b"\r\n"`.
    /// It returns `len` that counts the bytes of
    /// the entire line and the final `b'\n'` or `b"\r\n"`.
    ///
    /// # Example
    /// A CRLF-terminated line deframer:
    /// ```
    /// # use fixed_buffer::deframe_crlf;
    /// assert_eq!(Ok(None), deframe_crlf(b""));
    /// assert_eq!(Ok(None), deframe_crlf(b"abc"));
    /// assert_eq!(Ok(None), deframe_crlf(b"abc\r"));
    /// assert_eq!(Ok(None), deframe_crlf(b"abc\n"));
    /// assert_eq!(Ok(Some((0..3, 5))), deframe_crlf(b"abc\r\n"));
    /// assert_eq!(Ok(Some((0..3, 5))), deframe_crlf(b"abc\r\nX"));
    /// ```
    pub fn read_frame<R, F>(
        &mut self,
        reader: &mut R,
        deframer_fn: F,
    ) -> Result<Option<&[u8]>, std::io::Error>
    where
        R: std::io::Read,
        F: Fn(&[u8]) -> Result<Option<(core::ops::Range<usize>, usize)>, MalformedInputError>,
    {
        loop {
            if !self.is_empty() {
                if let Some(frame_range) = self.deframe(&deframer_fn)? {
                    return Ok(Some(&self.mem()[frame_range]));
                }
                // None case falls through.
            }
            self.shift();
            let writable = self.writable();
            if writable.is_empty() {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::InvalidData,
                    "end of buffer full",
                ));
            };
            let num_read = reader.read(writable)?;
            if num_read == 0 {
                if self.is_empty() {
                    return Ok(None);
                }
                return Err(std::io::Error::new(
                    std::io::ErrorKind::UnexpectedEof,
                    "eof after reading part of a frame",
                ));
            }
            self.wrote(num_read);
        }
    }
}

impl<const SIZE: usize> std::io::Write for FixedBuf<SIZE> {
    fn write(&mut self, data: &[u8]) -> std::io::Result<usize> {
        Ok(self.write_bytes(data)?)
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

impl<const SIZE: usize> std::io::Read for FixedBuf<SIZE> {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        Ok(self.read_and_copy_bytes(buf))
    }
}

impl<const SIZE: usize> core::fmt::Debug for FixedBuf<SIZE> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::result::Result<(), core::fmt::Error> {
        write!(
            f,
            "FixedBuf<{}>{{{} writable, {} readable: \"{}\"}}",
            SIZE,
            SIZE - self.write_index,
            self.len(),
            self.escape_ascii()
        )
    }
}

impl<const SIZE: usize> Default for FixedBuf<SIZE> {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use once_cell::sync::Lazy;
    use std::sync::Mutex;

    static STATIC_FIXED_BUF: Lazy<Mutex<FixedBuf<131072>>> =
        Lazy::new(|| Mutex::new(FixedBuf::new()));

    fn deframe_line_reject_xs(
        data: &[u8],
    ) -> Result<Option<(core::ops::Range<usize>, usize)>, MalformedInputError> {
        if data.contains(&b'x') || data.contains(&b'X') {
            return Err(MalformedInputError::new("err1".to_string()));
        }
        deframe_line(data)
    }

    #[test]
    fn test_static_fixed_buf() {
        let mut buf = STATIC_FIXED_BUF.lock().unwrap();
        assert_eq!(0, buf.len());
        buf.write_str("abc").unwrap();
        assert_eq!("abc", buf.escape_ascii());
    }

    #[test]
    fn test_new() {
        let buf: FixedBuf<1> = FixedBuf::new();
        assert_eq!("", buf.escape_ascii());
        let _: FixedBuf<42> = FixedBuf::new();
        let _: FixedBuf<262144> = FixedBuf::new();
        // Larger sizes will overflow the stack.
    }

    #[test]
    fn test_empty() {
        let mut buf: FixedBuf<8> = FixedBuf::empty([48_u8; 8] /* "aaaaaaaa" */);
        assert_eq!("", buf.escape_ascii());
        buf.write_str("aaaa").unwrap();
        assert_eq!("aaaa", buf.escape_ascii());
        buf.write_str("bbbb").unwrap();
        assert_eq!("aaaabbbb", buf.escape_ascii());
    }

    #[test]
    fn test_filled() {
        let mut buf: FixedBuf<8> = FixedBuf::filled([97_u8; 8] /* "aaaaaaaa" */);
        assert_eq!("aaaaaaaa", buf.escape_ascii());
        buf.read_byte();
        buf.shift();
        buf.write_str("b").unwrap();
        assert_eq!("aaaaaaab", buf.escape_ascii());
    }

    #[test]
    fn test_into_inner() {
        let mut buf: FixedBuf<8> = FixedBuf::new();
        buf.write_str("abc").unwrap();
        let mem = buf.into_inner();
        assert_eq!(&[97_u8, 98, 99, 0, 0, 0, 0, 0], &mem);
    }

    #[test]
    fn test_len() {
        let mut buf: FixedBuf<16> = FixedBuf::new();
        assert_eq!(0, buf.len());
        buf.write_str("abc").unwrap();
        assert_eq!(3, buf.len());
        buf.read_bytes(2);
        assert_eq!(1, buf.len());
        buf.shift();
        assert_eq!(1, buf.len());
        buf.read_all();
        assert_eq!(0, buf.len());
    }

    #[test]
    fn test_is_empty() {
        let mut buf: FixedBuf<16> = FixedBuf::new();
        assert!(buf.is_empty());
        buf.write_str("abc").unwrap();
        assert!(!buf.is_empty());
        buf.read_all();
        assert!(buf.is_empty());
    }

    #[test]
    fn test_clear() {
        let mut buf: FixedBuf<16> = FixedBuf::new();
        buf.write_str("abc").unwrap();
        assert_eq!(3, buf.len());
        buf.clear();
        assert!(buf.is_empty());
    }

    #[test]
    fn test_write_str() {
        let mut buf: FixedBuf<4> = FixedBuf::new();
        buf.write_str("a").unwrap();
        buf.write_str("bc").unwrap();
        assert_eq!("abc", buf.escape_ascii());
        assert_eq!(Err(NotEnoughSpaceError {}), buf.write_str("de"));
        assert_eq!("abc", buf.escape_ascii());
        buf.write_str("d").unwrap();
        assert_eq!("abcd", buf.escape_ascii());
        assert_eq!(Err(NotEnoughSpaceError {}), buf.write_str("e"));
    }

    #[test]
    fn test_write_bytes() {
        let mut buf: FixedBuf<4> = FixedBuf::new();
        assert_eq!(Ok(1usize), buf.write_bytes(b"a"));
        assert_eq!(Ok(2), buf.write_bytes(b"bc"));
        assert_eq!("abc", buf.escape_ascii());
        assert_eq!(Err(NotEnoughSpaceError {}), buf.write_bytes(b"de"));
        assert_eq!(Ok(1), buf.write_bytes(b"d"));
        assert_eq!("abcd", buf.escape_ascii());
        assert_eq!(Err(NotEnoughSpaceError {}), buf.write_bytes(b"e"));
    }

    #[test]
    fn test_writable_and_wrote() {
        let mut buf: FixedBuf<16> = FixedBuf::new();
        assert_eq!(16, buf.writable().len());
        buf.writable()[0] = 'a' as u8;
        buf.wrote(1);
        assert_eq!("a", buf.escape_ascii());
        let many_bs = "b".repeat(15);
        assert_eq!(many_bs.len(), buf.writable().len());
        buf.writable().copy_from_slice(many_bs.as_bytes());
        buf.wrote(many_bs.len());
        assert_eq!("a".to_string() + &many_bs, buf.escape_ascii());
        assert!(buf.writable().is_empty());
    }

    #[test]
    #[should_panic]
    fn test_wrote_too_much() {
        let mut buf: FixedBuf<16> = FixedBuf::new();
        buf.wrote(17);
    }

    #[test]
    fn test_shift_empty() {
        let mut buf: FixedBuf<4> = FixedBuf::new();
        buf.shift();
        buf.write_str("abcd").unwrap();
        assert_eq!("abcd", buf.escape_ascii());
    }

    #[test]
    fn test_shift_half_filled() {
        let mut buf: FixedBuf<6> = FixedBuf::new();
        buf.write_str("abcd").unwrap();
        buf.shift();
        buf.read_bytes(2);
        buf.write_str("ef").unwrap();
        assert_eq!(NotEnoughSpaceError {}, buf.write_str("gh").unwrap_err());
        buf.shift();
        buf.write_str("gh").unwrap();
        assert_eq!("cdefgh", buf.escape_ascii());
    }

    #[test]
    fn test_shift_full() {
        let mut buf: FixedBuf<4> = FixedBuf::new();
        buf.write_str("abcd").unwrap();
        buf.shift();
        buf.read_bytes(2);
        assert_eq!(NotEnoughSpaceError {}, buf.write_str("e").unwrap_err());
        buf.shift();
        buf.write_str("e").unwrap();
        buf.write_str("f").unwrap();
        assert_eq!(NotEnoughSpaceError {}, buf.write_str("g").unwrap_err());
        assert_eq!("cdef", buf.escape_ascii());
    }

    #[test]
    fn test_shift_read_all() {
        let mut buf: FixedBuf<4> = FixedBuf::new();
        buf.write_str("abcd").unwrap();
        buf.read_bytes(4);
        buf.shift();
        buf.write_str("efgh").unwrap();
        assert_eq!("efgh", buf.escape_ascii());
    }

    #[test]
    fn test_deframe() {
        let mut buf: FixedBuf<8> = FixedBuf::new();
        // Empty
        assert_eq!(None, buf.deframe(deframe_line_reject_xs).unwrap());
        // Incomplete
        buf.write_str("abc").unwrap();
        assert_eq!(None, buf.deframe(deframe_line_reject_xs).unwrap());
        assert_eq!("abc", buf.escape_ascii());
        // Complete
        buf.write_str("\n").unwrap();
        assert_eq!(Some(0..3), buf.deframe(deframe_line_reject_xs).unwrap());
        assert_eq!("", buf.escape_ascii());
        // Error
        buf.write_str("x").unwrap();
        assert_eq!(
            std::io::ErrorKind::InvalidData,
            buf.deframe(deframe_line_reject_xs).unwrap_err().kind()
        );
    }

    #[test]
    fn test_read_frame_empty_to_eof() {
        let mut buf: FixedBuf<8> = FixedBuf::new();
        let mut reader = std::io::Cursor::new(b"");
        assert_eq!(
            None,
            buf.read_frame(&mut reader, deframe_line_reject_xs).unwrap()
        );
        assert_eq!("", buf.escape_ascii());
    }

    #[test]
    fn test_read_frame_empty_to_incomplete() {
        let mut buf: FixedBuf<8> = FixedBuf::new();
        let mut reader = std::io::Cursor::new(b"abc");
        assert_eq!(
            std::io::ErrorKind::UnexpectedEof,
            buf.read_frame(&mut reader, deframe_line_reject_xs)
                .unwrap_err()
                .kind()
        );
        assert_eq!("abc", buf.escape_ascii());
    }

    #[test]
    fn test_read_frame_empty_to_complete() {
        let mut buf: FixedBuf<8> = FixedBuf::new();
        let mut reader = std::io::Cursor::new(b"abc\n");
        assert_eq!(
            "abc",
            escape_ascii(
                buf.read_frame(&mut reader, deframe_line_reject_xs)
                    .unwrap()
                    .unwrap()
            )
        );
        assert_eq!("", buf.escape_ascii());
    }

    #[test]
    fn test_read_frame_empty_to_complete_with_leftover() {
        let mut buf: FixedBuf<8> = FixedBuf::new();
        let mut reader = std::io::Cursor::new(b"abc\nde");
        assert_eq!(
            "abc",
            escape_ascii(
                buf.read_frame(&mut reader, deframe_line_reject_xs)
                    .unwrap()
                    .unwrap()
            )
        );
        assert_eq!("de", buf.escape_ascii());
    }

    #[test]
    fn test_read_frame_empty_to_invalid() {
        let mut buf: FixedBuf<8> = FixedBuf::new();
        let mut reader = std::io::Cursor::new(b"x");
        assert_eq!(
            std::io::ErrorKind::InvalidData,
            buf.read_frame(&mut reader, deframe_line_reject_xs)
                .unwrap_err()
                .kind()
        );
        assert_eq!("x", buf.escape_ascii());
    }

    #[test]
    fn test_read_frame_incomplete_to_eof() {
        let mut buf: FixedBuf<8> = FixedBuf::new();
        buf.write_str("a").unwrap();
        let mut reader = std::io::Cursor::new(b"");
        assert_eq!(
            std::io::ErrorKind::UnexpectedEof,
            buf.read_frame(&mut reader, deframe_line_reject_xs)
                .unwrap_err()
                .kind()
        );
        assert_eq!("a", buf.escape_ascii());
    }

    #[test]
    fn test_read_frame_incomplete_to_incomplete() {
        let mut buf: FixedBuf<8> = FixedBuf::new();
        buf.write_str("a").unwrap();
        let mut reader = std::io::Cursor::new(b"bc");
        assert_eq!(
            std::io::ErrorKind::UnexpectedEof,
            buf.read_frame(&mut reader, deframe_line_reject_xs)
                .unwrap_err()
                .kind()
        );
        assert_eq!("abc", buf.escape_ascii());
    }

    #[test]
    fn test_read_frame_incomplete_to_complete() {
        let mut buf: FixedBuf<8> = FixedBuf::new();
        buf.write_str("a").unwrap();
        let mut reader = std::io::Cursor::new(b"bc\n");
        assert_eq!(
            "abc",
            escape_ascii(
                buf.read_frame(&mut reader, deframe_line_reject_xs)
                    .unwrap()
                    .unwrap()
            )
        );
        assert_eq!("", buf.escape_ascii());
    }

    #[test]
    fn test_read_frame_incomplete_to_complete_with_leftover() {
        let mut buf: FixedBuf<8> = FixedBuf::new();
        buf.write_str("a").unwrap();
        let mut reader = std::io::Cursor::new(b"bc\nde");
        assert_eq!(
            "abc",
            escape_ascii(
                buf.read_frame(&mut reader, deframe_line_reject_xs)
                    .unwrap()
                    .unwrap()
            )
        );
        assert_eq!("de", buf.escape_ascii());
    }

    #[test]
    fn test_read_frame_complete_doesnt_read() {
        let mut buf: FixedBuf<8> = FixedBuf::new();
        buf.write_str("abc\n").unwrap();
        assert_eq!(
            "abc",
            escape_ascii(
                buf.read_frame(&mut FakeReadWriter::empty(), deframe_line_reject_xs)
                    .unwrap()
                    .unwrap()
            )
        );
        assert_eq!("", buf.escape_ascii());
    }

    #[test]
    fn test_read_frame_complete_leaves_leftovers() {
        let mut buf: FixedBuf<8> = FixedBuf::new();
        buf.write_str("abc\nde").unwrap();
        assert_eq!(
            "abc",
            escape_ascii(
                buf.read_frame(&mut FakeReadWriter::empty(), deframe_line_reject_xs)
                    .unwrap()
                    .unwrap()
            )
        );
        assert_eq!("de", buf.escape_ascii());
    }

    #[test]
    fn test_read_frame_invalid_doesnt_read() {
        let mut buf: FixedBuf<8> = FixedBuf::new();
        buf.write_str("x").unwrap();
        assert_eq!(
            std::io::ErrorKind::InvalidData,
            buf.read_frame(&mut FakeReadWriter::empty(), deframe_line_reject_xs)
                .unwrap_err()
                .kind()
        );
        assert_eq!("x", buf.escape_ascii());
    }

    #[test]
    fn test_read_frame_buffer_full() {
        let mut buf: FixedBuf<8> = FixedBuf::new();
        buf.write_str("abcdefgh").unwrap();
        let mut reader = std::io::Cursor::new(b"bc\nde");
        assert_eq!(
            std::io::ErrorKind::InvalidData,
            buf.read_frame(&mut reader, deframe_line_reject_xs)
                .unwrap_err()
                .kind()
        );
        assert_eq!("abcdefgh", buf.escape_ascii());
    }

    #[test]
    fn test_readable() {
        let mut buf: FixedBuf<16> = FixedBuf::new();
        assert_eq!("", escape_ascii(buf.readable()));
        buf.write_str("abc").unwrap();
        assert_eq!("abc", escape_ascii(buf.readable()));
        buf.read_bytes(2);
        assert_eq!("c", escape_ascii(buf.readable()));
        buf.read_bytes(1);
        assert_eq!("", escape_ascii(buf.readable()));
        buf.write_str("d").unwrap();
        assert_eq!("d", escape_ascii(buf.readable()));
        buf.shift();
        assert_eq!("d", escape_ascii(buf.readable()));
    }

    #[test]
    fn test_read_byte() {
        let mut buf: FixedBuf<8> = FixedBuf::new();
        buf.write_str("abc").unwrap();
        assert_eq!(b'a', buf.read_byte());
        assert_eq!("bc", buf.escape_ascii());
        assert_eq!(b'b', buf.read_byte());
        assert_eq!("c", buf.escape_ascii());
        assert_eq!(b'c', buf.read_byte());
        assert_eq!("", buf.escape_ascii());
    }

    #[test]
    fn test_read_byte_mut() {
        let mut buf: FixedBuf<16> = FixedBuf::new();
        assert_eq!("", buf.escape_ascii());
        buf.write_str("ab").unwrap();
        assert_eq!(b'a', buf.read_byte());
        assert_eq!("b", buf.escape_ascii());
        buf.write_str("c").unwrap();
        assert_eq!("bc", buf.escape_ascii());
        assert_eq!(b'b', buf.read_byte());
        assert_eq!("c", buf.escape_ascii());
        assert_eq!(b'c', buf.read_byte());
        assert_eq!("", buf.escape_ascii());
        buf.write_str("d").unwrap();
        assert_eq!("d", buf.escape_ascii());
        assert_eq!(b'd', buf.read_byte());
        assert_eq!("", buf.escape_ascii());
    }

    #[test]
    fn test_read_bytes() {
        let mut buf: FixedBuf<8> = FixedBuf::new();
        buf.write_str("abc").unwrap();
        assert_eq!("a", escape_ascii(buf.read_bytes(1)));
        assert_eq!("bc", buf.escape_ascii());
        assert_eq!("bc", escape_ascii(buf.read_bytes(2)));
        assert_eq!("", buf.escape_ascii());
    }

    #[test]
    fn test_read_bytes_mut() {
        let mut buf: FixedBuf<16> = FixedBuf::new();
        buf.write_str("abc").unwrap();
        assert_eq!("a", escape_ascii(buf.read_bytes(1)));
        assert_eq!("bc", buf.escape_ascii());
        assert_eq!("bc", escape_ascii(buf.read_bytes(2)));
        assert_eq!("", buf.escape_ascii());
        buf.write_str("d").unwrap();
        buf.shift();
        buf.write_str("e").unwrap();
        assert_eq!("d", escape_ascii(buf.read_bytes(1)));
        assert_eq!("e", buf.escape_ascii());
    }

    #[test]
    #[should_panic]
    fn test_read_bytes_underflow() {
        let mut buf: FixedBuf<16> = FixedBuf::new();
        buf.write_str("a").unwrap();
        buf.read_bytes(2);
    }

    #[test]
    fn test_try_read_bytes() {
        let mut buf: FixedBuf<8> = FixedBuf::new();
        buf.write_str("abc").unwrap();
        assert_eq!(Some("a".as_bytes()), buf.try_read_bytes(1));
        assert_eq!("bc", buf.escape_ascii());
        assert_eq!(None, buf.try_read_bytes(3));
        assert_eq!("bc", buf.escape_ascii());
        assert_eq!(Some("bc".as_bytes()), buf.try_read_bytes(2));
        assert_eq!("", buf.escape_ascii());
        assert_eq!(None, buf.try_read_bytes(1));
        assert_eq!("", buf.escape_ascii());
    }

    #[test]
    fn test_try_read_bytes_mut() {
        let mut buf: FixedBuf<16> = FixedBuf::new();
        buf.write_str("abc").unwrap();
        assert_eq!(Some("a".as_bytes()), buf.try_read_bytes(1));
        assert_eq!("bc", buf.escape_ascii());
        assert_eq!(None, buf.try_read_bytes(3));
        assert_eq!("bc", buf.escape_ascii());
        assert_eq!(Some("bc".as_bytes()), buf.try_read_bytes(2));
        assert_eq!("", buf.escape_ascii());
        assert_eq!(None, buf.try_read_bytes(1));
        assert_eq!("", buf.escape_ascii());
        buf.write_str("d").unwrap();
        buf.shift();
        buf.write_str("e").unwrap();
        assert_eq!(Some("d".as_bytes()), buf.try_read_bytes(1));
        assert_eq!("e", buf.escape_ascii());
    }

    #[test]
    fn test_read_all() {
        let mut buf: FixedBuf<16> = FixedBuf::new();
        assert_eq!("", escape_ascii(buf.read_all()));
        buf.write_str("abc").unwrap();
        assert_eq!("abc", escape_ascii(buf.read_all()));
        buf.write_str("def").unwrap();
        assert_eq!("def", escape_ascii(buf.read_all()));
        assert_eq!("", escape_ascii(buf.read_all()));
    }

    #[test]
    fn test_read_and_copy_bytes() {
        let mut buf: FixedBuf<8> = FixedBuf::new();
        buf.write_str("abc").unwrap();
        let mut one = [0_u8; 1];
        assert_eq!(1, buf.read_and_copy_bytes(&mut one));
        assert_eq!(b"a", &one);
        assert_eq!("bc", buf.escape_ascii());
        let mut four = [0_u8; 4];
        assert_eq!(2, buf.read_and_copy_bytes(&mut four));
        assert_eq!(&[b'b', b'c', 0, 0], &four);
        assert_eq!("", buf.escape_ascii());
    }

    #[test]
    fn test_read_and_copy_bytes_mut() {
        let mut buf: FixedBuf<16> = FixedBuf::new();
        buf.write_str("abc").unwrap();
        let mut one = [0_u8; 1];
        assert_eq!(1, buf.read_and_copy_bytes(&mut one));
        assert_eq!(b"a", &one);
        assert_eq!("bc", buf.escape_ascii());
        let mut four = [0_u8; 4];
        assert_eq!(2, buf.read_and_copy_bytes(&mut four));
        assert_eq!(&[b'b', b'c', 0, 0], &four);
        assert_eq!("", buf.escape_ascii());
        buf.write_str("d").unwrap();
        buf.shift();
        buf.write_str("e").unwrap();
        assert_eq!(1, buf.read_and_copy_bytes(&mut one));
        assert_eq!(b"d", &one);
        assert_eq!("e", buf.escape_ascii());
    }

    #[test]
    fn test_try_read_exact() {
        let mut buf: FixedBuf<8> = FixedBuf::new();
        buf.write_str("abc").unwrap();
        let mut one = [0_u8; 1];
        buf.try_read_exact(&mut one).unwrap();
        assert_eq!(b"a", &one);
        assert_eq!("bc", buf.escape_ascii());
        let mut three = [0_u8; 3];
        assert_eq!(None, buf.try_read_exact(&mut three));
        assert_eq!(&[0, 0, 0], &three);
        let mut two = [0_u8; 2];
        buf.try_read_exact(&mut two).unwrap();
        assert_eq!(b"bc", &two);
        assert_eq!("", buf.escape_ascii());
    }

    #[test]
    fn test_try_read_exact_mut() {
        let mut buf: FixedBuf<16> = FixedBuf::new();
        buf.write_str("abc").unwrap();
        let mut one = [0_u8; 1];
        buf.try_read_exact(&mut one).unwrap();
        assert_eq!(b"a", &one);
        assert_eq!("bc", buf.escape_ascii());
        let mut three = [0_u8; 3];
        assert_eq!(None, buf.try_read_exact(&mut three));
        assert_eq!(&[0, 0, 0], &three);
        assert_eq!("bc", buf.escape_ascii());
        buf.write_str("d").unwrap();
        buf.shift();
        buf.write_str("e").unwrap();
        buf.try_read_exact(&mut three).unwrap();
        assert_eq!(b"bcd", &three);
        assert_eq!("e", buf.escape_ascii());
    }

    #[test]
    fn test_try_parse() {
        #[derive(Debug, PartialEq)]
        enum CountError {
            BadValue,
            TooShort,
            TooLong,
        }
        impl core::fmt::Display for CountError {
            fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> Result<(), core::fmt::Error> {
                core::fmt::Debug::fmt(self, f)
            }
        }
        impl std::error::Error for CountError {}

        fn count_as<const SIZE: usize>(
            buf: &mut FixedBuf<SIZE>,
        ) -> Option<Result<usize, CountError>> {
            let mut count: usize = 0;
            loop {
                match buf.try_read_byte()? {
                    b'a' => {
                        count += 1;
                        if count > 3 {
                            return Some(Err(CountError::TooLong));
                        }
                    }
                    b'b' if count < 1 => return Some(Err(CountError::TooShort)),
                    b'b' => return Some(Ok(count)),
                    _ => return Some(Err(CountError::BadValue)),
                }
            }
        }
        assert_eq!(None, FixedBuf::empty([0]).try_parse(count_as));
        assert_eq!(None, FixedBuf::filled([b'a']).try_parse(count_as));
        assert_eq!(None, FixedBuf::filled([b'a', b'a']).try_parse(count_as));
        assert_eq!(
            None,
            FixedBuf::filled([b'a', b'a', b'a']).try_parse(count_as)
        );
        assert_eq!(
            Some(Err(CountError::TooLong)),
            FixedBuf::filled([b'a', b'a', b'a', b'a']).try_parse(count_as)
        );
        assert_eq!(
            Some(Err(CountError::TooShort)),
            FixedBuf::filled([b'b']).try_parse(count_as)
        );
        assert_eq!(
            Some(Ok(1)),
            FixedBuf::filled([b'a', b'b']).try_parse(count_as)
        );
        assert_eq!(
            Some(Ok(2)),
            FixedBuf::filled([b'a', b'a', b'b']).try_parse(count_as)
        );
        assert_eq!(
            Some(Ok(3)),
            FixedBuf::filled([b'a', b'a', b'a', b'b']).try_parse(count_as)
        );
        assert_eq!(
            Some(Err(CountError::TooLong)),
            FixedBuf::filled([b'a', b'a', b'a', b'a', b'b']).try_parse(count_as)
        );
        assert_eq!(
            Some(Err(CountError::BadValue)),
            FixedBuf::filled([b'a', b'c']).try_parse(count_as)
        );

        let mut buf: FixedBuf<16> = FixedBuf::new();
        assert_eq!(None, buf.try_parse(count_as));
        buf.write_str("a").unwrap();
        assert_eq!(None, buf.try_parse(count_as));
        assert_eq!("a", buf.escape_ascii());
        buf.write_str("baaba").unwrap();
        assert_eq!(Some(Ok(1)), buf.try_parse(count_as));
        assert_eq!("aaba", buf.escape_ascii());
        assert_eq!(Some(Ok(2)), buf.try_parse(count_as));
        assert_eq!("a", buf.escape_ascii());
        assert_eq!(None, buf.try_parse(count_as));
        buf.write_str("cbab").unwrap();
        assert_eq!(Some(Err(CountError::BadValue)), buf.try_parse(count_as));
        assert_eq!("bab", buf.escape_ascii());
        buf.clear();
        buf.write_str("b").unwrap();
        assert_eq!(Some(Err(CountError::TooShort)), buf.try_parse(count_as));
        assert_eq!("", buf.escape_ascii());

        // Cannot borrow.
        // error: lifetime may not live long enough
        // let mut borrowable_buf = FixedBuf::filled(b"a");
        // assert_eq!(
        //     Some(Ok(&b"a"[..])),
        //     borrowable_buf.try_parse(|buf| Some(Ok::<&[u8], ()>(buf.readable())))
        // );
    }

    #[test]
    fn test_copy_once_from() {
        let mut buf: FixedBuf<4> = FixedBuf::new();

        // Appends existing data.
        buf.write_str("a").unwrap();
        assert_eq!(
            2,
            buf.copy_once_from(&mut std::io::Cursor::new(b"12"))
                .unwrap()
        );
        assert_eq!("a12", escape_ascii(buf.read_all()));

        // EOF, doesn't fill buffer.
        assert_eq!(
            3,
            buf.copy_once_from(&mut std::io::Cursor::new(b"123"))
                .unwrap()
        );
        assert_eq!("123", escape_ascii(buf.read_all()));

        // EOF, fills buffer.
        assert_eq!(
            4,
            buf.copy_once_from(&mut std::io::Cursor::new(b"1234"))
                .unwrap()
        );
        assert_eq!("1234", escape_ascii(buf.read_all()));

        // Fills buffer.
        assert_eq!(
            4,
            buf.copy_once_from(&mut std::io::Cursor::new(b"12345"))
                .unwrap()
        );
        assert_eq!("1234", escape_ascii(buf.read_all()));

        // Buffer already full
        buf.write_str("abcd").unwrap();
        assert_eq!(
            std::io::ErrorKind::InvalidData,
            buf.copy_once_from(&mut std::io::Cursor::new(b"1"))
                .unwrap_err()
                .kind()
        );
        assert_eq!("abcd", escape_ascii(buf.read_all()));

        // Reads only once.
        assert_eq!(
            2,
            buf.copy_once_from(&mut FakeReadWriter::new(vec![Ok(2)]))
                .unwrap()
        );
        assert_eq!("ab", escape_ascii(buf.read_all()));
    }

    #[test]
    fn test_escape_ascii() {
        assert_eq!("", FixedBuf::filled([]).escape_ascii());
        assert_eq!("abc", FixedBuf::filled([b'a', b'b', b'c']).escape_ascii());
        assert_eq!("\\r\\n", FixedBuf::filled([b'\r', b'\n']).escape_ascii());
        let mut buf: FixedBuf<4> = FixedBuf::new();
        buf.write_str("???").unwrap();
        assert_eq!("\\xe2\\x82\\xac", buf.escape_ascii());
        assert_eq!("\\x01", FixedBuf::filled([1]).escape_ascii());
        let buf = FixedBuf::filled([b'a', b'b', b'c']);
        assert_eq!("abc", buf.escape_ascii());
        assert_eq!(b"abc", buf.readable());
    }

    #[test]
    fn test_std_io_write() {
        let mut buf: FixedBuf<16> = FixedBuf::new();
        std::io::Write::write(&mut buf, b"abc").unwrap();
        assert_eq!("abc", buf.escape_ascii());
        std::io::Write::write(&mut buf, b"def").unwrap();
        assert_eq!("abcdef", buf.escape_ascii());
        buf.read_bytes(1);
        std::io::Write::write(&mut buf, b"g").unwrap();
        assert_eq!("bcdefg", buf.escape_ascii());
        std::io::Write::write(&mut buf, "h".repeat(8).as_bytes()).unwrap();
        std::io::Write::write(&mut buf, b"i").unwrap();
        assert_eq!(
            std::io::ErrorKind::InvalidData,
            std::io::Write::write(&mut buf, b"def").unwrap_err().kind()
        );
    }

    #[test]
    fn test_std_io_read() {
        let mut buf: FixedBuf<16> = FixedBuf::new();
        let mut data = ['.' as u8; 16];
        assert_eq!(0, std::io::Read::read(&mut buf, &mut data).unwrap());
        assert_eq!("..........", escape_ascii(&data[..10]));
        buf.write_str("abc").unwrap();
        assert_eq!(3, std::io::Read::read(&mut buf, &mut data).unwrap());
        assert_eq!("abc.......", escape_ascii(&data[..10]));
        assert_eq!(0, std::io::Read::read(&mut buf, &mut data).unwrap());
        let many_bs = "b".repeat(16);
        buf.write_str(&many_bs).unwrap();
        assert_eq!(16, std::io::Read::read(&mut buf, &mut data).unwrap());
        assert_eq!(many_bs, escape_ascii(&data[..]));
        assert_eq!(0, std::io::Read::read(&mut buf, &mut data).unwrap());
    }

    #[test]
    fn test_default() {
        let _: FixedBuf<8> = FixedBuf::default();
        let _: FixedBuf<16> = FixedBuf::default();
        let mut buf: FixedBuf<32> = FixedBuf::default();
        buf.write_str("abc").unwrap();
        assert_eq!("abc", buf.escape_ascii());
    }

    #[test]
    fn test_debug() {
        let mut buf: FixedBuf<8> = FixedBuf::new();
        buf.write_str("abc").unwrap();
        buf.read_bytes(1);
        assert_eq!(
            "FixedBuf<8>{5 writable, 2 readable: \"bc\"}",
            format!("{:?}", buf)
        );
    }
}

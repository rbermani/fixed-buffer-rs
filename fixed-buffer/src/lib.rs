//! This is a Rust library with fixed-size buffers, useful for network protocol parsers.
//!
//! # Features
//! - Write bytes to the buffer and read them back
//! - Lives on the stack
//! - Does not allocate memory
//! - Supports tokio's AsyncRead and AsyncWrite
//! - Use it to read a stream, search for a delimiter, and save leftover bytes for the next read.
//! - Easy to learn & use.  Easy to maintain code that uses it.
//! - Works with Rust `latest`, `beta`, and `nightly`
//! - No macros
//!
//! # Limitations
//! - Not a circular buffer.
//!   You can call `shift()` periodically to move unread bytes to the front of the buffer.
//! - There is no `iterate_delimited(AsyncRead)`.
//!   Because of borrowing rules, such a function would need to return non-borrowed (allocated and copied) data.
//!
//! # Examples
//! TODO(mleonhard) Remove tokio from examples.
//! Read and handle requests from a remote client:
//! ```rust
//! use fixed_buffer::FixedBuf;
//! use std::io::Error;
//! use tokio::io::AsyncWriteExt;
//! use tokio::net::TcpStream;
//!
//! async fn handle_conn(mut tcp_stream: TcpStream) -> Result<(), Error> {
//!     let (mut input, mut output) = tcp_stream.split();
//!     let mut buf: FixedBuf<[u8; 4096]> = FixedBuf::new([0; 4096]);
//!     loop {
//!         // Read a line and leave leftover bytes in `buf`.
//!         let line_bytes = match buf.read_delimited(&mut input, b"\n").await? {
//!             Some(line_bytes) => line_bytes,
//!             None => return Ok(()),
//!         };
//!         let request = Request::parse(line_bytes)?;
//!         // Read any request payload from `buf` + `TcpStream`.
//!         let payload_reader = tokio::io::AsyncReadExt::chain(&mut buf, &mut input);
//!         handle_request(&mut output, payload_reader, request).await?;
//!     }
//! }
//! ```
//! For a runnable example, see [examples/server.rs](examples/server.rs).
//!
//! Read and process records:
//! ```rust
//! fn try_process_record(b: &[u8]) -> Result<usize, Error> {
//!     if b.len() < 2 {
//!         return Ok(0)
//!     }
//!     if b.starts_with("ab".as_bytes()) {
//!         println!("found record");
//!         Ok(2)
//!     } else {
//!         Err(Error::new(ErrorKind::InvalidData, "bad record"))
//!     }
//! }
//!
//! async fn read_and_process<R: tokio::io::AsyncRead + Unpin>(mut input: R)
//!     -> Result<(), Error> {
//!     let mut buf: FixedBuf<[u8; 1024]> = FixedBuf::new([0; 1024]);
//!     loop {
//!         // Read a chunk into the buffer.
//!         let mut writable = buf.writable()
//!             .ok_or(Error::new(ErrorKind::InvalidData, "record too long, buffer full"))?;
//!         let bytes_written = AsyncReadExt::read(&mut input, &mut writable).await?;
//!         if bytes_written == 0 {
//!             return if buf.len() == 0 {
//!                 Ok(())  // EOF at record boundary
//!             } else {
//!                 // EOF in the middle of a record
//!                 Err(Error::from(ErrorKind::UnexpectedEof))
//!             };
//!         }
//!         buf.wrote(bytes_written);
//!
//!         // Process records in the buffer.
//!         loop {
//!             let bytes_read = try_process_record(buf.readable())?;
//!             if bytes_read == 0 {
//!                 break;
//!             }
//!             buf.read_bytes(bytes_read);
//!         }
//!         // Shift data in the buffer to free up space at the end for writing.
//!         buf.shift();
//!     }
//! }
//! ```
//!
//! # Documentation
//! https://docs.rs/fixed-buffer
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
//! # Release Process
//! 1. Edit `Cargo.toml` and bump version number.
//! 1. Run `./release.sh`
//!
//! # Changelog
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
//! - DONE - Custom buffer length.
//!   - https://crates.io/crates/generic-array
//!   - https://crates.io/crates/block-buffer
//!   - https://crates.io/crates/string-wrapper
//! - DONE - Publish to creates.io
//! - DONE - Read through https://crate-ci.github.io/index.html
//! - DONE - Get a code review from an experienced rustacean
//! - DONE - Add and update a changelog
//!   - Update it manually
//!   - https://crate-ci.github.io/release/changelog.html
//! - Add features: std, tokio, async-std
//! - Simplify `read_delimited()`
//! - Make a more generic read_frame that takes a frame detector function.
//!   Make `read_delimited` use that.
//! - Implement FixedBuf::chain(AsyncRead) which buffers reads like [tokio::io::ReadBuf](https://docs.rs/tokio/0.3.0/tokio/io/struct.ReadBuf.html).
//! - Fix FixedBuf rustdoc link to box_benchmark.
//! - Switch to const generics once they are stable:
//!   - https://github.com/rust-lang/rust/issues/44580
//!   - https://stackoverflow.com/a/56543462
//! - Set up CI on:
//!   - DONE - Linux x86 64-bit
//!   - [macOS](https://gitlab.com/gitlab-org/gitlab/-/issues/269756)
//!   - [Windows](https://about.gitlab.com/blog/2020/01/21/windows-shared-runner-beta/)
//!   - https://crate-ci.github.io/pr/testing.html#travisci
//!   - Linux ARM 64-bit (Raspberry Pi 3 and newer)
//!   - Linux ARM 32-bit (Raspberry Pi 2)
//!   - RISCV & ESP32 firmware?
#![forbid(unsafe_code)]

/// Convert a byte slice into a string.
/// Includes printable ASCII characters as-is.
/// Converts non-printable or non-ASCII characters to strings like "\n" and "\x19".
///
/// Uses
/// [`core::ascii::escape_default`](https://doc.rust-lang.org/core/ascii/fn.escape_default.html)
/// internally to escape each byte.
///
/// This function is useful for printing byte slices to logs and comparing byte slices in tests.
///
/// Example test:
/// ```
/// use fixed_buffer::{escape_ascii, FixedBuf};
/// let mut buf: FixedBuf<[u8; 16]> = FixedBuf::new([0u8; 16]);
/// buf.write_str("ab");
/// buf.write_str("cd");
/// assert_eq!("abcd", escape_ascii(buf.readable()));
/// ```
pub fn escape_ascii(input: &[u8]) -> String {
    let mut result = String::new();
    for byte in input {
        for ascii_byte in core::ascii::escape_default(*byte) {
            result.push_str(core::str::from_utf8(&[ascii_byte]).unwrap());
        }
    }
    result
}

#[derive(Clone, PartialEq, Debug)]
pub struct NotEnoughSpaceError {}

impl core::fmt::Display for NotEnoughSpaceError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> Result<(), core::fmt::Error> {
        core::fmt::Debug::fmt(self, f)
    }
}

/// FixedBuf is a fixed-length byte buffer.
/// You can write bytes to it and then read them back.
///
/// It is not a circular buffer.  Call [`shift`](#method.shift) periodically to
/// move unread bytes to the front of the buffer.
///
/// Use [`new`](#method.new) to create
/// `FixedBuf<[u8; N]>`, `FixedBuf<Box<[u8]>>`, and `FixedBuf<&mut [u8]>` structs.
///
/// Note that `FixedBuf<Box<[u8]>>` uses less memory than `Box<FixedBuf<[u8; N]>>`.
/// See [`new`](#method.new) for details.
#[derive(Default, Copy, Clone, Eq, Hash, PartialEq)]
pub struct FixedBuf<T> {
    mem: T,
    read_index: usize,
    write_index: usize,
}

impl<T> FixedBuf<T> {
    /// Makes a new empty buffer, consuming or borrowing `mem`
    /// and using it as the internal memory array.
    ///
    /// Creates `FixedBuf<[u8; N]>`, `FixedBuf<Box<[u8]>>`, and `FixedBuf<&mut [u8]>` structs.
    ///
    /// This function is the inverse of [`into_inner`](#method.into_inner).
    ///
    /// `FixedBuf<&mut [u8]>` uses borrowed memory.
    /// Create one like this:
    ///
    /// ```
    /// # use fixed_buffer::FixedBuf;
    /// let mut mem = [0u8; 42];
    /// let mut buf: FixedBuf<&mut [u8]> = FixedBuf::new(&mut mem);
    /// ```
    ///
    /// `FixedBuf<[u8; N]>` can live on the stack.  Be careful of stack overflows!
    /// Create one like this:
    ///
    /// ```
    /// # use fixed_buffer::FixedBuf;
    /// let mut buf: FixedBuf<[u8; 42]> = FixedBuf::new([0u8; 42]);
    /// ```
    ///
    /// `FixedBuf<Box<[u8]>>` stores its memory block on the heap.
    /// `Box` supports [coercion](https://doc.rust-lang.org/std/ops/trait.CoerceUnsized.html)
    /// of `Box<[T; N]>` to `Box<[T]>`.  Use that to create a boxed buffer:
    ///
    /// ```
    /// # use fixed_buffer::FixedBuf;
    /// let mut buf: FixedBuf<Box<[u8]>> = FixedBuf::new(Box::new([0u8; 42]));
    /// // Your editor may incorrectly report "mismatched types [E0308]".
    /// ```
    ///
    /// Note that `FixedBuf<Box<[u8]>>` is 10-25% more memory efficient than
    /// `Box<FixedBuf<[u8; N]>>`.  Explanation:
    ///
    /// Standard heaps allocate memory in blocks.  The block sizes are powers of two and
    /// some intervening sizes.  For example,
    /// [jemalloc's block sizes](http://jemalloc.net/jemalloc.3.html#size_classes) are
    /// 8, 16, 32, 48, 64, 80, 96, 112, 128, 160, 192, 224, 256 bytes, and so on.
    /// Thus jemalloc uses 160 bytes to store a 129 byte value.
    ///
    /// Every `FixedBuf<[u8; N]>` contains two `usize` index values
    /// in addition to its buffer memory.
    /// If you create a buffer with a power-of-two size,
    /// the struct is always a few bytes larger than a power of two,
    /// and takes up the next larger block size on the heap.
    /// For example, in a 64-bit program using jemalloc,
    /// Box<`FixedBuf<[u8; 128]>`> uses 128 + 8 + 8 = 144 bytes,
    /// and gets stored in a 160 byte block, wasting an extra 11% of memory.
    ///
    /// By comparison, `FixedBuf<Box<[u8]>>` keeps the buffer memory separate
    /// from the index values and therefore wastes no memory.
    /// This is because `Box<[u8; 128]>` uses exactly 128-bytes on the heap.
    ///
    /// Run the program
    /// [`examples/main.rs`](https://gitlab.com/leonhard-llc/fixed-buffer-rs/-/blob/main/fixed-buffer/examples/box_benchmark.rs)
    /// to see the memory usage difference.
    pub const fn new(mem: T) -> Self {
        Self {
            mem,
            write_index: 0,
            read_index: 0,
        }
    }

    /// Drops the struct and returns its internal array.
    ///
    /// This function is the inverse of [`new`](#method.new).
    pub fn into_inner(self) -> T {
        self.mem
    }

    /// Returns the number of unread bytes in the buffer.
    ///
    /// Example:
    /// ```
    /// # use fixed_buffer::FixedBuf;
    /// let mut buf: FixedBuf<[u8; 16]> = FixedBuf::new([0u8; 16]);
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
    /// let mut buf: FixedBuf<[u8; 16]> = FixedBuf::new([0u8; 16]);
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
}

impl<T: AsRef<[u8]>> FixedBuf<T> {
    /// Makes a new full buffer, consuming or borrowing `mem`
    /// and using it as the internal memory array.
    /// Reading the buffer will return the bytes in `mem`.
    ///
    /// You can write to the returned buf if `mem` implements `AsMut<[u8]`.
    ///
    /// For details, see [`new`](#method.new).
    ///
    /// Examples:
    /// ```
    /// # use fixed_buffer::FixedBuf;
    /// // Readable, not writable:
    /// let mut buf1 = FixedBuf::filled(b"abc");
    ///
    /// // Readable and writable:
    /// let mut buf2 = FixedBuf::filled([0u8; 42]);
    /// let mut buf3: FixedBuf<[u8; 42]> = FixedBuf::filled([0u8; 42]);
    /// let mut buf4: FixedBuf<Box<[u8]>> = FixedBuf::filled(Box::new([0u8; 42]));
    /// // Your editor may incorrectly report "mismatched types [E0308]" --^
    /// ```
    pub fn filled(mem: T) -> Self {
        Self {
            write_index: mem.as_ref().len(),
            read_index: 0,
            mem,
        }
    }

    /// Returns the maximum number of bytes that can be stored in the buffer.
    ///
    /// Example:
    /// ```
    /// # use fixed_buffer::FixedBuf;
    /// let mut buf: FixedBuf<[u8; 16]> = FixedBuf::new([0u8; 16]);
    /// assert_eq!(16, buf.capacity());
    /// buf.write_str("abc").unwrap();
    /// assert_eq!(16, buf.capacity());
    /// ```
    pub fn capacity(&self) -> usize {
        self.mem.as_ref().len()
    }

    /// Copies all readable bytes to a string.
    /// Includes printable ASCII characters as-is.
    /// Converts non-printable characters to strings like "\n" and "\x19".
    ///
    /// Leaves the buffer unchanged.
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
    /// let mut buf = FixedBuf::new([0u8; 16]);
    /// buf.write_str("abc");
    /// buf.write_str("€");
    /// assert_eq!("abc\\xe2\\x82\\xac", buf.escape_ascii());
    /// ```
    pub fn escape_ascii(&self) -> String {
        escape_ascii(self.readable())
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
    ///
    /// TODO(mleonhard) Remove tokio deps from example.
    /// Example:
    /// ```
    /// # use fixed_buffer::FixedBuf;
    /// # use std::io::{Error, ErrorKind};
    /// # use tokio::io::AsyncReadExt;
    /// fn try_process_record(b: &[u8]) -> Result<usize, Error> {
    ///     if b.len() < 2 {
    ///         return Ok(0);
    ///     }
    ///     if b.starts_with("ab".as_bytes()) {
    ///         println!("found record");
    ///         Ok(2)
    ///     } else {
    ///         Err(Error::new(ErrorKind::InvalidData, "bad record"))
    ///     }
    /// }
    ///
    /// async fn read_and_process<R: tokio::io::AsyncRead + Unpin>(mut input: R)
    ///     -> Result<(), Error> {
    ///     let mut buf: FixedBuf<[u8; 1024]> = FixedBuf::new([0; 1024]);
    ///     loop {
    ///         // Read a chunk into the buffer.
    ///         let mut writable = buf.writable()
    ///             .ok_or(Error::new(ErrorKind::InvalidData, "record too long, buffer full"))?;
    ///         let bytes_written = AsyncReadExt::read(&mut input, &mut writable).await?;
    ///         if bytes_written == 0 {
    ///             return if buf.len() == 0 {
    ///                 Ok(())  // EOF at record boundary
    ///             } else {
    ///                 // EOF in the middle of a record
    ///                 Err(Error::from(ErrorKind::UnexpectedEof))
    ///             };
    ///         }
    ///         buf.wrote(bytes_written);
    ///
    ///         // Process records in the buffer.
    ///         loop {
    ///             let bytes_read = try_process_record(buf.readable())?;
    ///             if bytes_read == 0 {
    ///                 break;
    ///             }
    ///             buf.read_bytes(bytes_read);
    ///         }
    ///         // Shift data in the buffer to free up space at the end for writing.
    ///         buf.shift();
    ///     }
    /// }
    ///
    /// # tokio_test::block_on(async {
    /// read_and_process(std::io::Cursor::new(b"")).await.unwrap();
    /// read_and_process(std::io::Cursor::new(b"abab")).await.unwrap();
    /// assert_eq!(
    ///     std::io::ErrorKind::UnexpectedEof,
    ///     read_and_process(std::io::Cursor::new(b"aba")).await.unwrap_err().kind()
    /// );
    /// # })
    /// ```
    pub fn readable(&self) -> &[u8] {
        &self.mem.as_ref()[self.read_index..self.write_index]
    }

    /// Read bytes from the buffer.
    ///
    /// Panics if the buffer does not contain enough bytes.
    pub fn read_bytes(&mut self, num_bytes: usize) -> &[u8] {
        let new_read_index = self.read_index + num_bytes;
        if new_read_index > self.write_index {
            panic!("read would underflow");
        }
        let old_read_index = self.read_index;
        self.read_index = new_read_index;
        if self.read_index == self.write_index {
            // All data has been read.  Reset the buffer.
            self.write_index = 0;
            self.read_index = 0;
        }
        &self.mem.as_ref()[old_read_index..new_read_index]
    }

    /// Read all the bytes from the buffer.
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
}

impl<T: AsMut<[u8]>> FixedBuf<T> {
    /// Writes `s` into the buffer, after any unread bytes.
    ///
    /// Returns [`Err`] if the buffer doesn't have enough free space at the end
    /// for the whole string.
    ///
    /// See [`shift`](#method.shift).
    ///
    /// Example:
    /// ```
    /// # use fixed_buffer::{escape_ascii, FixedBuf};
    /// let mut buf: FixedBuf<[u8; 8]> = FixedBuf::new([0u8; 8]);
    /// buf.write_str("123").unwrap();
    /// buf.write_str("456").unwrap();
    /// assert_eq!("1234", escape_ascii(buf.read_bytes(4)));
    /// buf.write_str("78").unwrap();
    /// buf.write_str("9").unwrap_err();  // End of buffer is full.
    /// ```
    ///
    /// [`Ok`]: https://doc.rust-lang.org/std/result/enum.Result.html#variant.Ok
    /// [`shift`]: #method.shift
    pub fn write_str(&mut self, s: &str) -> Result<(), NotEnoughSpaceError> {
        self.write_bytes(s.as_bytes()).map(|_| ())
    }

    /// Try to write `data` into the buffer, after any unread bytes.
    ///
    /// Returns `Ok(data.len())` if it wrote all of the bytes.
    ///
    /// Returns [`core::result::Result::Err`](https://doc.rust-lang.org/core/result/enum.Result.html#variant.Err)
    /// if the buffer doesn't have enough free space at the end for all of the bytes.
    ///
    /// See [`shift`](#method.shift).
    ///
    /// Example:
    /// ```
    /// # use fixed_buffer::{escape_ascii, FixedBuf};
    /// let mut buf: FixedBuf<[u8; 8]> = FixedBuf::new([0u8; 8]);
    /// assert_eq!(3 as usize, buf.write_bytes("123".as_bytes()).unwrap());
    /// assert_eq!(3 as usize, buf.write_bytes("456".as_bytes()).unwrap());
    /// assert_eq!("1234", escape_ascii(buf.read_bytes(4)));
    /// assert_eq!(2 as usize, buf.write_bytes("78".as_bytes()).unwrap());  // Fills buffer.
    /// buf.write_bytes("9".as_bytes()).unwrap_err();  // Error, buffer is full.
    /// ```
    pub fn write_bytes(&mut self, data: &[u8]) -> core::result::Result<usize, NotEnoughSpaceError> {
        let writable = self.writable().ok_or_else(|| {
            NotEnoughSpaceError {}
        })?;
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
    /// Returns [`None`](https://doc.rust-lang.org/std/option/enum.Option.html)
    /// when the end of the buffer is full.
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
    /// let mut buf: FixedBuf<[u8; 8]> = FixedBuf::new([0u8; 8]);
    /// buf.writable().unwrap()[0] = 'a' as u8;
    /// buf.writable().unwrap()[1] = 'b' as u8;
    /// buf.writable().unwrap()[2] = 'c' as u8;
    /// buf.wrote(3);
    /// assert_eq!("abc", escape_ascii(buf.read_bytes(3)));
    /// ```
    pub fn writable(&mut self) -> Option<&mut [u8]> {
        if self.write_index >= self.mem.as_mut().len() {
            // Ran out of space.
            return None;
        }
        Some(&mut self.mem.as_mut()[self.write_index..])
    }

    /// Commit bytes into the buffer.
    /// Call this after writing to the front of the
    /// [`writable`](#method.writable) slice.
    ///
    /// This is a low-level method.
    ///
    /// Panics when there is not `num_bytes` free at the end of the buffer.
    ///
    /// See example in [`writable()`](#method.writable).
    pub fn wrote(&mut self, num_bytes: usize) {
        if num_bytes == 0 {
            return;
        }
        let new_write_index = self.write_index + num_bytes;
        if new_write_index > self.mem.as_mut().len() {
            panic!("write would overflow");
        }
        self.write_index = new_write_index;
    }

    /// Recovers buffer space.
    ///
    /// The buffer is not circular.
    /// After you read bytes, the space at the beginning of the buffer is unused.
    /// Call this method to move unread data to the beginning of the buffer and recover the space.
    /// This makes the free space available for writes, which go at the end of the buffer.
    ///
    /// For an example, see [`readable`](#method.readable).
    pub fn shift(&mut self) {
        if self.read_index == 0 {
            return;
        }
        if self.read_index == self.write_index {
            self.write_index = 0;
            self.read_index = 0;
            return;
        }
        self.mem
            .as_mut()
            .copy_within(self.read_index..self.write_index, 0);
        self.write_index -= self.read_index;
        self.read_index = 0;
    }
}

impl<T> Unpin for FixedBuf<T> {}

impl<T: AsMut<[u8]>> std::io::Write for FixedBuf<T> {
    fn write(&mut self, data: &[u8]) -> std::io::Result<usize> {
        self.write_bytes(data).map_err(|_|
            std::io::Error::new(std::io::ErrorKind::InvalidData, "not enough space in buffer")
        )
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

impl<T: AsRef<[u8]>> std::io::Read for FixedBuf<T> {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        Ok(self.read_and_copy_bytes(buf))
    }
}

impl<T: AsRef<[u8]>> core::fmt::Debug for FixedBuf<T> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::result::Result<(), core::fmt::Error> {
        write!(
            f,
            "FixedBuf{{{} writable, {} readable: \"{}\"}}",
            self.capacity() - self.write_index,
            self.len(),
            self.escape_ascii()
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use once_cell::sync::Lazy;
    use std::sync::Mutex;

    static STATIC_FIXED_BUF: Lazy<Mutex<FixedBuf<[u8; 128 * 1024]>>> =
        Lazy::new(|| Mutex::new(FixedBuf::new([0; 128 * 1024])));

    #[test]
    fn test_static_fixed_buf() {
        let mut buf = STATIC_FIXED_BUF.lock().unwrap();
        assert_eq!(0, buf.len());
        buf.write_str("abc").unwrap();
        assert_eq!("abc", escape_ascii(buf.read_all()));
    }

    #[test]
    fn test_new() {
        let _: FixedBuf<[u8; 1]> = FixedBuf::new([0; 1]);
        let _: FixedBuf<[u8; 42]> = FixedBuf::new([0; 42]);
        let _: FixedBuf<[u8; 256 * 1024]> = FixedBuf::new([0; 256 * 1024]);
        // Larger sizes will overflow the stack.
    }

    #[test]
    fn test_new_box_slice() {
        let _: FixedBuf<Box<[u8]>> = FixedBuf::new(Box::new([0; 1]));
        let _: FixedBuf<Box<[u8]>> = FixedBuf::new(Box::new([0; 42]));
        let _: FixedBuf<Box<[u8]>> = FixedBuf::new(Box::new([0; 512 * 1024]));
        // Larger sizes will overflow the stack.
    }

    #[test]
    fn test_array_constructors() {
        let mut buf: FixedBuf<[u8; 16]> = FixedBuf::default();
        buf.write_str("abc").unwrap();
        assert_eq!("abc", escape_ascii(buf.readable()));
        let mem: [u8; 16] = buf.into_inner();
        buf = FixedBuf::new(mem);
        assert_eq!("", escape_ascii(buf.readable()));
        buf.wrote(3);
        assert_eq!("abc", escape_ascii(buf.read_all()));
        assert_eq!("", escape_ascii(buf.readable()));
    }

    #[test]
    fn test_box_array_constructors() {
        let mut buf: FixedBuf<Box<[u8]>> = FixedBuf::new(Box::new([0u8; 16]) as Box<[u8]>);
        buf.write_str("abc").unwrap();
        assert_eq!("abc", escape_ascii(buf.readable()));
        let mem = buf.into_inner();
        buf = FixedBuf::new(mem);
        assert_eq!("", escape_ascii(buf.readable()));
        buf.wrote(3);
        assert_eq!("abc", escape_ascii(buf.read_all()));
        assert_eq!("", escape_ascii(buf.readable()));
    }

    #[test]
    fn test_slice_constructor() {
        let mut mem = [0u8; 15];
        let mut buf = FixedBuf::new(&mut mem);
        buf.write_str("abc").unwrap();
        assert_eq!("abc", escape_ascii(buf.readable()));
        buf = FixedBuf::new(&mut mem);
        assert_eq!("", escape_ascii(buf.readable()));
        buf.wrote(3);
        assert_eq!("abc", escape_ascii(buf.read_all()));
        assert_eq!("", escape_ascii(buf.readable()));
    }

    #[test]
    fn test_filled_const() {
        let mut buf = FixedBuf::filled(b"abc");
        assert_eq!("abc", escape_ascii(buf.read_all()));
    }

    #[test]
    fn test_filled_array() {
        let mut buf = FixedBuf::filled([7u8; 10]);
        assert_eq!(&[7], buf.read_bytes(1));
        buf.write_bytes(&[42u8]).unwrap_err();
        buf.shift();
        buf.write_bytes(&[42u8]).unwrap();
    }

    #[test]
    fn test_filled_box_array() {
        let mut buf: FixedBuf<Box<[u8]>> = FixedBuf::filled(Box::new([7u8; 10]));
        assert_eq!(&[7], buf.read_bytes(1));
        buf.write_bytes(&[42u8]).unwrap_err();
        buf.shift();
        buf.write_bytes(&[42u8]).unwrap();
    }

    #[test]
    fn empty() {
        let mut buf: FixedBuf<[u8; 16]> = FixedBuf::default();
        assert_eq!("", escape_ascii(buf.readable()));
        assert_eq!("", escape_ascii(buf.read_all()));
        buf.shift();
        assert_eq!("", escape_ascii(buf.readable()));
        assert_eq!("", escape_ascii(buf.read_all()));
    }

    #[test]
    fn test_len() {
        let mut buf: FixedBuf<[u8; 16]> = FixedBuf::default();
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
        let mut buf: FixedBuf<[u8; 16]> = FixedBuf::default();
        assert!(buf.is_empty());
        buf.write_str("abc").unwrap();
        assert!(!buf.is_empty());
        buf.read_all();
        assert!(buf.is_empty());
    }

    #[test]
    fn test_clear() {
        let mut buf = FixedBuf::filled(b"abc");
        assert_eq!(3, buf.len());
        buf.clear();
        assert!(buf.is_empty());
    }

    #[test]
    fn test_write_str() {
        let mut buf: FixedBuf<[u8; 16]> = FixedBuf::default();
        buf.write_str("a").unwrap();
        buf.write_str("b").unwrap();
        assert_eq!("ab", escape_ascii(buf.readable()));
        let many_cs = "c".repeat(13);
        buf.write_str(&many_cs).unwrap();
        buf.write_str("d").unwrap();
        assert_eq!(
            "ab".to_string() + &many_cs + "d",
            escape_ascii(buf.readable())
        );
        buf.write_str("e").unwrap_err();
    }

    #[test]
    fn test_writable_and_wrote() {
        let mut buf: FixedBuf<[u8; 16]> = FixedBuf::default();
        assert_eq!(16, buf.writable().unwrap().len());
        buf.writable().unwrap()[0] = 'a' as u8;
        buf.wrote(1);
        assert_eq!("a", escape_ascii(buf.readable()));
        let many_bs = "b".repeat(15);
        assert_eq!(many_bs.len(), buf.writable().unwrap().len());
        buf.writable().unwrap().copy_from_slice(many_bs.as_bytes());
        buf.wrote(many_bs.len());
        assert_eq!("a".to_string() + &many_bs, escape_ascii(buf.readable()));
        assert_eq!(None, buf.writable());
    }

    #[test]
    #[should_panic]
    fn test_wrote_too_much() {
        let mut buf: FixedBuf<[u8; 16]> = FixedBuf::default();
        buf.wrote(17);
    }

    #[test]
    fn test_readable_and_read() {
        let mut buf: FixedBuf<[u8; 16]> = FixedBuf::default();
        assert_eq!("", escape_ascii(buf.readable()));
        buf.write_str("abc").unwrap();
        assert_eq!("abc", escape_ascii(buf.readable()));
        buf.read_bytes(1);
        assert_eq!("bc", escape_ascii(buf.readable()));
        buf.read_bytes(2);
        assert_eq!("", escape_ascii(buf.readable()));
        buf.write_str("d").unwrap();
        assert_eq!("d", escape_ascii(buf.readable()));
        buf.read_bytes(1);
        assert_eq!("", escape_ascii(buf.readable()));
    }

    #[test]
    #[should_panic]
    fn test_read_too_much() {
        let mut buf: FixedBuf<[u8; 16]> = FixedBuf::default();
        buf.write_str("a").unwrap();
        buf.read_bytes(2);
    }

    #[test]
    fn test_read_all() {
        let mut buf: FixedBuf<[u8; 16]> = FixedBuf::default();
        assert_eq!("", escape_ascii(buf.read_all()));
        buf.write_str("abc").unwrap();
        assert_eq!("abc", escape_ascii(buf.read_all()));
        buf.write_str("def").unwrap();
        assert_eq!("def", escape_ascii(buf.read_all()));
        assert_eq!("", escape_ascii(buf.read_all()));
    }

    #[test]
    fn test_escape_ascii() {
        assert_eq!("", FixedBuf::filled(b"").escape_ascii());
        assert_eq!("abc", FixedBuf::filled(b"abc").escape_ascii());
        assert_eq!("\\r\\n", FixedBuf::filled(b"\r\n").escape_ascii());
        assert_eq!(
            "\\xe2\\x82\\xac",
            FixedBuf::filled("€".as_bytes()).escape_ascii()
        );
        assert_eq!("\\x01", FixedBuf::filled(b"\x01").escape_ascii());
        let buf = FixedBuf::filled(b"abc");
        assert_eq!("abc", buf.escape_ascii());
        assert_eq!(b"abc", buf.readable());
    }

    #[test]
    fn test_capacity() {
        assert_eq!(0, FixedBuf::new([0u8; 0]).capacity());
        assert_eq!(16, FixedBuf::new([0u8; 16]).capacity());
        let buf: FixedBuf<Box<[u8]>> = FixedBuf::new(Box::new([0u8; 16]));
        assert_eq!(16, buf.capacity());
        let mut mem = [0u8; 16];
        assert_eq!(16, FixedBuf::new(&mut mem).capacity());
        assert_eq!(16, FixedBuf::filled([0u8; 16]).capacity());
    }

    #[test]
    fn test_std_io_write() {
        let mut buf: FixedBuf<[u8; 16]> = FixedBuf::default();
        std::io::Write::write(&mut buf, b"abc").unwrap();
        assert_eq!("abc", escape_ascii(buf.readable()));
        std::io::Write::write(&mut buf, b"def").unwrap();
        assert_eq!("abcdef", escape_ascii(buf.readable()));
        buf.read_bytes(1);
        std::io::Write::write(&mut buf, b"g").unwrap();
        assert_eq!("bcdefg", escape_ascii(buf.readable()));
        std::io::Write::write(&mut buf, "h".repeat(8).as_bytes()).unwrap();
        std::io::Write::write(&mut buf, b"i").unwrap();
        assert_eq!(
            std::io::ErrorKind::InvalidData,
            std::io::Write::write(&mut buf, b"def").unwrap_err().kind()
        );
    }

    #[test]
    fn test_std_io_read() {
        let mut buf: FixedBuf<[u8; 16]> = FixedBuf::default();
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
        let _: FixedBuf<[u8; 8]> = FixedBuf::default();
        let _: FixedBuf<[u8; 16]> = FixedBuf::default();
        let mut array_buf: FixedBuf<[u8; 32]> = FixedBuf::default();
        array_buf.write_str("abc").unwrap();
        assert_eq!("abc", escape_ascii(array_buf.readable()));
        // Default box buf has empty slice and cannot be read or written.
        let mut box_buf: FixedBuf<Box<[u8]>> = FixedBuf::default();
        assert_eq!("", escape_ascii(box_buf.readable()));
        assert!(box_buf.writable().is_none());
        // let slice_buf: FixedBuf<&mut [u8]> = FixedBuf::default(); // compiler error
    }

    #[test]
    fn test_debug() {
        let mut array_buf: FixedBuf<[u8; 8]> = FixedBuf::default();
        let mut box_buf: FixedBuf<Box<[u8]>> = FixedBuf::new(Box::new([0; 16]));
        let mut mem = [0u8; 15];
        let mut slice_buf = FixedBuf::new(&mut mem);
        array_buf.write_str("abc").unwrap();
        box_buf.write_str("abc").unwrap();
        slice_buf.write_str("abc").unwrap();
        array_buf.read_bytes(1);
        box_buf.read_bytes(1);
        slice_buf.read_bytes(1);
        assert_eq!(
            "FixedBuf{5 writable, 2 readable: \"bc\"}",
            format!("{:?}", array_buf)
        );
        assert_eq!(
            "FixedBuf{13 writable, 2 readable: \"bc\"}",
            format!("{:?}", box_buf)
        );
        assert_eq!(
            "FixedBuf{12 writable, 2 readable: \"bc\"}",
            format!("{:?}", slice_buf)
        );
    }
}

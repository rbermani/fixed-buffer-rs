use core::pin::Pin;
use core::task::{Context, Poll};

/// Convert a byte slice into a string.
/// Includes printable ASCII characters as-is.
/// Converts non-printable or non-ASCII characters to strings like "\n" and "\x19".
///
/// Uses [`std::ascii::escape_default`] internally to escape each byte.
///
/// This function is useful for printing byte slices to logs and comparing byte slices in tests.
///
/// Example test:
/// ```
/// use fixed_buffer::{escape_ascii, FixedBuf};
/// let mut buf: FixedBuf<[u8; 16]> = FixedBuf::new();
/// buf.write_str("ab");
/// buf.write_str("cd");
/// assert_eq!("abcd", escape_ascii(buf.readable()));
/// ```
///
/// [`std::ascii::escape_default`]: https://doc.rust-lang.org/std/ascii/fn.escape_default.html
pub fn escape_ascii(input: &[u8]) -> String {
    let mut result = String::new();
    for byte in input {
        for ascii_byte in std::ascii::escape_default(*byte) {
            result.push_str(std::str::from_utf8(&[ascii_byte]).unwrap());
        }
    }
    result
}

/// A trait providing access to the buffer's memory block.
pub trait MemBlock {
    fn u8slice(&self) -> &[u8];
    fn mut_u8slice(&mut self) -> &mut [u8];
}
impl MemBlock for &mut [u8] {
    fn u8slice(&self) -> &[u8] {
        &self[..]
    }

    fn mut_u8slice(&mut self) -> &mut [u8] {
        &mut self[..]
    }
}
macro_rules! generate_mem_block_impls {
    ( $( $LEN:expr , )+ ) => {
        $(
            impl MemBlock for [u8; $LEN] {
                fn u8slice(&self) -> &[u8] {
                    &self[..]
                }
                fn mut_u8slice(&mut self) -> &mut [u8] {
                    &mut self[..]
                }
            }

            impl MemBlock for Box<[u8; $LEN]> {
                fn u8slice(&self) -> &[u8] {
                    &self.as_ref()[..]
                }
                fn mut_u8slice(&mut self) -> &mut [u8] {
                    &mut self.as_mut()[..]
                }
            }
        )+
    }
}
generate_mem_block_impls! {
    8, 16, 32, 64, 100, 128, 200, 256, 512,
    1024, 2 * 1024, 4 * 1024, 8 * 1024, 16 * 1024, 32 * 1024,
    64 * 1024, 128 * 1024, 256 * 1024, 512 * 1024, 1024 * 1024,
}

/// A trait for creating memory blocks that are part of a FixedBuf.
pub trait OwnedMemBlock: MemBlock {
    fn new() -> Self;
}
macro_rules! generate_owned_mem_block_impls {
    ( $( $LEN:expr , )+ ) => {
        $(
            impl OwnedMemBlock for [u8; $LEN] {
                fn new() -> Self {
                    [0u8; $LEN]
                }
            }

            impl OwnedMemBlock for Box<[u8; $LEN]> {
                fn new() -> Self {
                    Box::new([0u8; $LEN])
                }
            }
        )+
    }
}
generate_owned_mem_block_impls! {
    8, 16, 32, 64, 100, 128, 200, 256, 512,
    1024, 2 * 1024, 4 * 1024, 8 * 1024, 16 * 1024, 32 * 1024,
    64 * 1024, 128 * 1024, 256 * 1024, 512 * 1024, 1024 * 1024,
}

/// FixedBuf is a fixed-length byte buffer.
/// You can write bytes to it and then read them back.
///
/// It implements tokio's [`AsyncRead`] and [`AsyncWrite`] traits.
///
/// Use [`read_delimited`] to read lines and other delimited messages.
/// This works like [`tokio::io::AsyncBufReadExt::read_until`],
/// but uses a fixed sized buffer so network peers cannot OOM the process.
///
/// It is not a circular buffer.  Call [`shift`] periodically to
/// move unread bytes to the front of the buffer.
///
/// Use [`new`] to create `FixedBuf<[u8; N]>` and `FixedBuf<Box<[u8; N]>>` structs
/// with sizes
/// 8, 16, 32, 64, 100, 128, 200, 256, 512 bytes,
/// 1KB, 2KB, 4KB, 8KB, 16KB, 32KB 64KB, 128KB, 512KB, and 1MB.
///
/// Note that `FixedBuf<Box<[u8; N]>>` uses less memory than `Box<FixedBuf<[u8; N]>>`.
/// See [`new`] for details.
///
/// Use [`new_with_slice`] to create a `FixedBuf<&mut [u8]>` that uses a borrowed slice of any size.
///
/// [`Box`]: https://doc.rust-lang.org/std/boxed/struct.Box.html
/// [`new`]: #method.new
/// [`new_with_slice`]: #method.new_with_slice
/// [`read_delimited`]: #method.read_delimited
/// [`shift`]: #method.shift
/// [`AsyncRead`]: https://docs.rs/tokio/0.3.0/tokio/io/trait.AsyncRead.html
/// [`AsyncWrite`]: https://docs.rs/tokio/0.3.0/tokio/io/trait.AsyncWrite.html
/// [`tokio::io::AsyncBufReadExt::read_until`]: https://docs.rs/tokio/latest/tokio/io/trait.AsyncBufReadExt.html
#[derive(Clone, Eq, Hash, PartialEq)]
pub struct FixedBuf<T: MemBlock> {
    mem: T,
    read_index: usize,
    write_index: usize,
}

impl FixedBuf<&mut [u8]> {
    /// Makes a new buffer with the provided slice as the internal memory.
    pub fn new_with_slice(mem: &mut [u8]) -> FixedBuf<&mut [u8]> {
        FixedBuf {
            mem,
            write_index: 0,
            read_index: 0,
        }
    }
}

impl<T: OwnedMemBlock> Default for FixedBuf<T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T: OwnedMemBlock> FixedBuf<T> {
    /// Makes a new buffer with an internal memory array.
    ///
    /// Creates `FixedBuf<[u8; N]>` and `FixedBuf<Box<[u8; N]>>` structs
    /// with sizes
    /// 8, 16, 32, 64, 100, 128, 200, 256, 512 bytes,
    /// 1KB, 2KB, 4KB, 8KB, 16KB, 32KB 64KB, 128KB, 512KB, and 1MB.
    ///
    /// Use [`new_with_slice`] to create a `FixedBuf<&mut [u8]>`
    /// that uses a borrowed slice of any size.
    ///
    /// `FixedBuf<[u8; N]>` is stored on the stack.  Be careful of stack overflows!
    ///
    /// `FixedBuf<Box<[u8; N]>>` stores its memory block on the heap.
    ///
    /// Note that `FixedBuf<Box<[u8; N]>>` is 15% more memory efficient than
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
    /// Since the supported buffer sizes are powers of two, the struct size is always a few
    /// bytes larger than a power of two, and takes up the next larger block size on the
    /// heap.  For example, in a 64-bit program using jemalloc,
    /// Box<`FixedBuf<[u8; 128]>`> uses 128 + 8 + 8 (buffer + read_index + write_index) = 144 bytes,
    /// and gets stored in a 160 byte block, wasting an extra 11% of memory.
    ///
    /// Use `FixedBuf<Box<[u8; N]>>` to avoid wasting memory.
    /// Those structs allocate their buffer memory separately.
    /// A `FixedBuf<Box<[u8; 128]>>` allocates a 128-byte array on the heap and wastes no memory.
    ///
    /// Run the program [`examples/box_benchmark.rs`] to see the memory usage
    /// difference.
    ///
    /// [`examples/box_benchmark.rs`]: examples/box_benchmark.rs
    /// [`new_with_slice`]: #method.new_with_slice
    pub fn new() -> Self {
        Self {
            mem: T::new(),
            write_index: 0,
            read_index: 0,
        }
    }

    /// Consumes the provided array and uses it in a new empty buffer.
    ///
    /// This function is the inverse of [`into_inner`].
    ///
    /// [`into_inner`]: #method.into_inner
    pub fn new_with_mem(mem: T) -> Self {
        Self {
            mem,
            write_index: 0,
            read_index: 0,
        }
    }

    /// Drops the struct and returns its internal array.
    ///
    /// This function is the inverse of [`new_with_mem`].
    ///
    /// [`new_with_mem`]: #method.new_with_mem
    pub fn into_inner(self) -> T {
        self.mem
    }
}

impl<T: MemBlock> FixedBuf<T> {
    /// Returns the number of unread bytes in the buffer.
    ///
    /// Example:
    /// ```
    /// # use fixed_buffer::FixedBuf;
    /// let mut buf: FixedBuf<[u8; 16]> = FixedBuf::new();
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

    /// Returns the maximum number of bytes that can be stored in the buffer.
    ///
    /// Example:
    /// ```
    /// # use fixed_buffer::FixedBuf;
    /// let mut buf: FixedBuf<[u8; 16]> = FixedBuf::new();
    /// assert_eq!(16, buf.capacity());
    /// buf.write_str("abc").unwrap();
    /// assert_eq!(16, buf.capacity());
    /// ```
    pub fn capacity(&self) -> usize {
        self.mem.u8slice().len()
    }

    /// Returns true if there are unread bytes in the buffer.
    ///
    /// Example:
    /// ```
    /// # use fixed_buffer::FixedBuf;
    /// let mut buf: FixedBuf<[u8; 16]> = FixedBuf::new();
    /// assert!(buf.is_empty());
    /// buf.write_str("abc").unwrap();
    /// assert!(!buf.is_empty());
    /// buf.read_all();
    /// assert!(buf.is_empty());
    /// ```
    pub fn is_empty(&self) -> bool {
        self.write_index == self.read_index
    }

    /// Writes `s` into the buffer, after any unread bytes.
    ///
    /// Returns [`Err`] if the buffer doesn't have enough free space at the end
    /// for the whole string.
    ///
    /// See [`shift`].
    ///
    /// Example:
    /// ```
    /// # use fixed_buffer::{escape_ascii, FixedBuf};
    /// let mut buf: FixedBuf<[u8; 8]> = FixedBuf::new();
    /// buf.write_str("123").unwrap();
    /// buf.write_str("456").unwrap();
    /// assert_eq!("1234", escape_ascii(buf.read_bytes(4)));
    /// buf.write_str("78").unwrap();
    /// buf.write_str("9").unwrap_err();  // End of buffer is full.
    /// ```
    ///
    /// [`Ok`]: https://doc.rust-lang.org/std/result/enum.Result.html#variant.Ok
    /// [`shift`]: #method.shift
    pub fn write_str(&mut self, s: &str) -> std::io::Result<()> {
        self.write_bytes(s.as_bytes()).map(|_| ())
    }

    /// Try to write `data` into the buffer, after any unread bytes.
    ///
    /// Returns `Ok(data.len())` if it wrote all of the bytes.
    ///
    /// Returns [`Err`] if the buffer doesn't have enough free space at the end
    /// for all of the bytes.
    ///
    /// See [`shift`].
    ///
    /// Example:
    /// ```
    /// # use fixed_buffer::{escape_ascii, FixedBuf};
    /// let mut buf: FixedBuf<[u8; 8]> = FixedBuf::new();
    /// assert_eq!(3 as usize, buf.write_bytes("123".as_bytes()).unwrap());
    /// assert_eq!(3 as usize, buf.write_bytes("456".as_bytes()).unwrap());
    /// assert_eq!("1234", escape_ascii(buf.read_bytes(4)));
    /// assert_eq!(2 as usize, buf.write_bytes("78".as_bytes()).unwrap());  // Fills buffer.
    /// buf.write_bytes("9".as_bytes()).unwrap_err();  // Error, buffer is full.
    /// ```
    ///
    /// [`Err`]: https://doc.rust-lang.org/std/io/struct.Error.html
    /// [`shift`]: #method.shift
    pub fn write_bytes(&mut self, data: &[u8]) -> std::io::Result<usize> {
        let writable = self.writable().ok_or_else(|| {
            std::io::Error::new(std::io::ErrorKind::InvalidData, "end of buffer full")
        })?;
        if writable.len() < data.len() {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "Not enough free space in buffer",
            ));
        }
        let dest = &mut writable[..data.len()];
        dest.copy_from_slice(data);
        self.wrote(data.len());
        Ok(data.len())
    }

    /// Returns the writable part of the buffer.
    ///
    /// To use this, first modify bytes at the beginning of the slice.
    /// Then call [`wrote(usize)`] to commit those bytes into the buffer
    /// and make them available for reading.
    ///
    /// Returns [`None`] when the end of the buffer is full.  See [`shift`].
    ///
    /// This is a low-level method.
    /// You probably want to use [`std::io::Write::write`] and [`tokio::io::AsyncWriteExt::write`]
    /// instead.
    ///
    /// Example:
    /// ```
    /// # use fixed_buffer::{escape_ascii, FixedBuf};
    /// let mut buf: FixedBuf<[u8; 8]> = FixedBuf::new();
    /// buf.writable().unwrap()[0] = 'a' as u8;
    /// buf.writable().unwrap()[1] = 'b' as u8;
    /// buf.writable().unwrap()[2] = 'c' as u8;
    /// buf.wrote(3);
    /// assert_eq!("abc", escape_ascii(buf.read_bytes(3)));
    /// ```
    ///
    /// [`shift`]: #method.shift
    /// [`wrote(usize)`]: #method.wrote
    /// [`None`]: https://doc.rust-lang.org/std/option/enum.Option.html
    /// [`std::io::Write::write`]: https://doc.rust-lang.org/std/io/trait.Write.html#tymethod.write
    /// [`tokio::io::AsyncWriteExt::write`]: https://docs.rs/tokio/0.3.0/tokio/io/trait.AsyncWriteExt.html#method.write
    pub fn writable(&mut self) -> Option<&mut [u8]> {
        if self.write_index >= self.mem.u8slice().len() {
            // Ran out of space.
            return None;
        }
        Some(&mut self.mem.mut_u8slice()[self.write_index..])
    }

    /// Commit bytes into the buffer.
    /// Call this after writing to the front of the [`writable`] slice.
    ///
    /// This is a low-level method.
    ///
    /// Panics if [`writable()`] is not large enough.
    ///
    /// See example in [`writable()`].
    ///
    /// [`writable`]: #method.writable
    /// [`writable()`]: #method.writable
    pub fn wrote(&mut self, num_bytes: usize) {
        if num_bytes == 0 {
            return;
        }
        let new_write_index = self.write_index + num_bytes;
        if new_write_index > self.mem.u8slice().len() {
            panic!("write would overflow");
        }
        self.write_index = new_write_index;
    }

    /// Returns the slice of readable bytes in the buffer.
    /// After processing some bytes from the front of the slice, call [`read`]
    /// to consume the bytes.
    ///
    /// This is a low-level method.
    /// You probably want to use
    /// [`read`],
    /// [`std::io::Read::read`],
    /// and [`tokio::io::AsyncReadExt::read`]
    /// instead.
    ///
    /// Example:
    /// ```
    /// # use fixed_buffer::FixedBuf;
    /// # use std::io::{Error, ErrorKind};
    /// # use tokio::io::AsyncReadExt;
    /// fn try_process_record(b: &[u8]) -> Result<usize, Error> {
    ///     if b.len() < 2 {
    ///         return Ok(0)
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
    ///     let mut buf: FixedBuf<[u8; 1024]> = FixedBuf::new();
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
    ///
    /// [`read`]: #method.read
    /// [`std::io::Read::read`]: https://doc.rust-lang.org/std/io/trait.Read.html#tymethod.read
    /// [`tokio::io::AsyncReadExt::read`]: https://docs.rs/tokio/0.3.0/tokio/io/trait.AsyncReadExt.html#method.read
    pub fn readable(&self) -> &[u8] {
        &self.mem.u8slice()[self.read_index..self.write_index]
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
        &self.mem.u8slice()[old_read_index..new_read_index]
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
    /// Returns `Ok(0)` when the buffer is empty or `dest` is zero-length.
    pub fn read_and_copy_bytes(&mut self, dest: &mut [u8]) -> std::io::Result<usize> {
        let readable = self.readable();
        let len = core::cmp::min(dest.len(), readable.len());
        if len == 0 {
            return Ok(0);
        }
        let src = &readable[..len];
        let copy_dest = &mut dest[..len];
        copy_dest.copy_from_slice(src);
        self.read_bytes(len);
        Ok(len)
    }

    /// Reads from a [`tokio::io::AsyncRead`] into the buffer until it finds `delim`.
    /// Returns the slice up until `delim`.
    /// Consumes the returned bytes and `delim`.
    /// Leaves unused bytes in the buffer.
    ///
    /// If the buffer already contains `delim`,
    /// returns the data immediately without reading from `input`.
    ///
    /// If the buffer does not already contain `delim`, calls [`shift`] before
    /// reading from `input`.
    ///
    /// Returns `Err(Error(InvalidData,_))` if the buffer fills up before `delim` is found.
    ///
    /// Demo:
    /// ```
    /// # use fixed_buffer::{escape_ascii, FixedBuf};
    /// # tokio_test::block_on(async {
    /// let mut buf: FixedBuf<[u8; 32]> = FixedBuf::new();
    /// let mut input = std::io::Cursor::new(b"aaa\nbbb\n\nccc\n");
    /// assert_eq!("aaa", escape_ascii(buf.read_delimited(&mut input, b"\n").await.unwrap()));
    /// assert_eq!("bbb", escape_ascii(buf.read_delimited(&mut input, b"\n").await.unwrap()));
    /// assert_eq!("",    escape_ascii(buf.read_delimited(&mut input, b"\n").await.unwrap()));
    /// assert_eq!("ccc", escape_ascii(buf.read_delimited(&mut input, b"\n").await.unwrap()));
    /// assert_eq!(
    ///     std::io::ErrorKind::NotFound,
    ///     buf.read_delimited(&mut input, b"\n").await.unwrap_err().kind()
    /// );
    /// # })
    /// ```
    ///
    /// Example usage:
    /// ```
    /// # use fixed_buffer::FixedBuf;
    /// # use std::io::Error;
    /// # use tokio::io::{AsyncWriteExt, AsyncWrite, AsyncRead};
    /// # use tokio::net::TcpStream;
    /// #
    /// # struct Request(());
    /// # impl Request {
    /// #     pub fn parse(b: &[u8]) -> Result<Request, Error> {
    /// #         Ok(Request(()))
    /// #     }
    /// # }
    /// # async fn handle_request<W: AsyncWrite, R: AsyncRead>(output: W, reader: R, req: Request)
    /// #     -> Result<(), Error> {
    /// #     Ok(())
    /// # }
    /// # async fn handle_conn(mut tcp_stream: TcpStream) -> Result<(), Error> {
    /// let (mut input, mut output) = tcp_stream.split();
    /// let mut buf: FixedBuf<[u8; 1024]> = FixedBuf::new();
    /// loop {
    ///     // Read a line and leave leftover bytes in `buf`.
    ///     let line_bytes: &[u8] = buf.read_delimited(&mut input, b"\n").await?;
    ///     let request = Request::parse(line_bytes)?;
    ///     // Read any request payload from `buf` + `TcpStream`.
    ///     let payload_reader = tokio::io::AsyncReadExt::chain(&mut buf, &mut input);
    ///     handle_request(&mut output, payload_reader, request).await?;
    /// }
    /// # }
    /// ```
    ///
    /// [`shift`]: #method.shift
    /// [`tokio::io::AsyncRead`]: https://docs.rs/tokio/0.3.0/tokio/io/trait.AsyncRead.html
    pub async fn read_delimited<R>(&mut self, mut input: R, delim: &[u8]) -> std::io::Result<&[u8]>
    where
        R: tokio::io::AsyncRead + std::marker::Unpin + Send,
    {
        loop {
            if let Some(delim_index) = self
                .readable()
                .windows(delim.len())
                .enumerate()
                .filter(|(_index, window)| *window == delim)
                .map(|(index, _window)| index)
                .next()
            {
                let result_start = self.read_index;
                let result_end = self.read_index + delim_index;
                self.read_bytes(delim_index + delim.len());
                return Ok(&self.mem.u8slice()[result_start..result_end]);
            }
            self.shift();
            let writable = self.writable().ok_or_else(|| {
                std::io::Error::new(std::io::ErrorKind::InvalidData, "end of buffer full")
            })?;
            let num_bytes_read = tokio::io::AsyncReadExt::read(&mut input, writable).await?;
            if num_bytes_read == 0 {
                if self.read_index == 0 {
                    return Err(std::io::Error::new(
                        std::io::ErrorKind::NotFound,
                        "eof with no data read",
                    ));
                }
                return Err(std::io::Error::new(
                    std::io::ErrorKind::UnexpectedEof,
                    "eof before delim read",
                ));
            }
            self.wrote(num_bytes_read);
        }
    }

    /// Recovers buffer space.
    ///
    /// The buffer is not circular.
    /// After you read bytes, the space at the beginning of the buffer is unused.
    /// Call this method to move unread data to the beginning of the buffer and recover the space.
    /// This makes the free space available for writes, which go at the end of the buffer.
    ///
    /// For an example, see [`readable`].
    ///
    /// [`readable`]: #method.readable
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
            .mut_u8slice()
            .copy_within(self.read_index..self.write_index, 0);
        self.write_index -= self.read_index;
        self.read_index = 0;
    }
}

impl<T: MemBlock> std::io::Write for FixedBuf<T> {
    fn write(&mut self, data: &[u8]) -> std::io::Result<usize> {
        self.write_bytes(data)
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

impl<T: MemBlock> std::io::Read for FixedBuf<T> {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        self.read_and_copy_bytes(buf)
    }
}

impl<T: MemBlock + Unpin> tokio::io::AsyncWrite for FixedBuf<T> {
    fn poll_write(
        self: Pin<&mut Self>,
        _cx: &mut Context<'_>,
        buf: &[u8],
    ) -> Poll<Result<usize, std::io::Error>> {
        Poll::Ready(self.get_mut().write_bytes(buf))
    }

    fn poll_flush(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<Result<(), std::io::Error>> {
        Poll::Ready(Ok(()))
    }

    fn poll_shutdown(
        self: Pin<&mut Self>,
        _cx: &mut Context<'_>,
    ) -> Poll<Result<(), std::io::Error>> {
        Poll::Ready(Ok(()))
    }
}

impl<T: MemBlock + Unpin> tokio::io::AsyncRead for FixedBuf<T> {
    fn poll_read(
        self: Pin<&mut Self>,
        _cx: &mut Context<'_>,
        buf: &mut tokio::io::ReadBuf<'_>,
    ) -> Poll<Result<(), std::io::Error>> {
        Poll::Ready(
            self.get_mut()
                .read_and_copy_bytes(buf.initialize_unfilled())
                .map(|n| buf.advance(n)),
        )
    }
}

impl<T: MemBlock> std::fmt::Debug for FixedBuf<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "FixedBuf{{{} writable, {} readable: \"{}\"}}",
            self.capacity() - self.write_index,
            self.len(),
            escape_ascii(self.readable())
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_array_sizes() {
        let _: FixedBuf<[u8; 8]> = FixedBuf::new();
        let _: FixedBuf<[u8; 16]> = FixedBuf::new();
        let _: FixedBuf<[u8; 32]> = FixedBuf::new();
        let _: FixedBuf<[u8; 64]> = FixedBuf::new();
        let _: FixedBuf<[u8; 100]> = FixedBuf::new();
        let _: FixedBuf<[u8; 128]> = FixedBuf::new();
        let _: FixedBuf<[u8; 200]> = FixedBuf::new();
        let _: FixedBuf<[u8; 256]> = FixedBuf::new();
        let _: FixedBuf<[u8; 512]> = FixedBuf::new();
        let _: FixedBuf<[u8; 1024]> = FixedBuf::new();
        let _: FixedBuf<[u8; 2 * 1024]> = FixedBuf::new();
        let _: FixedBuf<[u8; 4 * 1024]> = FixedBuf::new();
        let _: FixedBuf<[u8; 8 * 1024]> = FixedBuf::new();
        let _: FixedBuf<[u8; 16 * 1024]> = FixedBuf::new();
        let _: FixedBuf<[u8; 32 * 1024]> = FixedBuf::new();
        let _: FixedBuf<[u8; 64 * 1024]> = FixedBuf::new();
        let _: FixedBuf<[u8; 128 * 1024]> = FixedBuf::new();
        let _: FixedBuf<[u8; 256 * 1024]> = FixedBuf::new();
        let _: FixedBuf<[u8; 512 * 1024]> = FixedBuf::new();
        // let _: FixedBuf<[u8; 1024 * 1024]> = FixedBuf::new(); // overflows stack
    }

    #[test]
    fn test_box_array_sizes() {
        let _: FixedBuf<Box<[u8; 8]>> = FixedBuf::new();
        let _: FixedBuf<Box<[u8; 16]>> = FixedBuf::new();
        let _: FixedBuf<Box<[u8; 32]>> = FixedBuf::new();
        let _: FixedBuf<Box<[u8; 64]>> = FixedBuf::new();
        let _: FixedBuf<Box<[u8; 100]>> = FixedBuf::new();
        let _: FixedBuf<Box<[u8; 128]>> = FixedBuf::new();
        let _: FixedBuf<Box<[u8; 200]>> = FixedBuf::new();
        let _: FixedBuf<Box<[u8; 256]>> = FixedBuf::new();
        let _: FixedBuf<Box<[u8; 512]>> = FixedBuf::new();
        let _: FixedBuf<Box<[u8; 1024]>> = FixedBuf::new();
        let _: FixedBuf<Box<[u8; 2 * 1024]>> = FixedBuf::new();
        let _: FixedBuf<Box<[u8; 4 * 1024]>> = FixedBuf::new();
        let _: FixedBuf<Box<[u8; 8 * 1024]>> = FixedBuf::new();
        let _: FixedBuf<Box<[u8; 16 * 1024]>> = FixedBuf::new();
        let _: FixedBuf<Box<[u8; 32 * 1024]>> = FixedBuf::new();
        let _: FixedBuf<Box<[u8; 64 * 1024]>> = FixedBuf::new();
        let _: FixedBuf<Box<[u8; 128 * 1024]>> = FixedBuf::new();
        let _: FixedBuf<Box<[u8; 256 * 1024]>> = FixedBuf::new();
        let _: FixedBuf<Box<[u8; 512 * 1024]>> = FixedBuf::new();
        let _: FixedBuf<Box<[u8; 1024 * 1024]>> = FixedBuf::new();
    }

    #[test]
    fn test_array_constructors() {
        let mut buf: FixedBuf<[u8; 16]> = FixedBuf::new();
        buf.write_str("abc").unwrap();
        assert_eq!("abc", escape_ascii(buf.readable()));
        let mem: [u8; 16] = buf.into_inner();
        buf = FixedBuf::new_with_mem(mem);
        assert_eq!("", escape_ascii(buf.readable()));
        buf.wrote(3);
        assert_eq!("abc", escape_ascii(buf.read_all()));
        assert_eq!("", escape_ascii(buf.readable()));
    }

    #[test]
    fn test_box_array_constructors() {
        let mut buf: FixedBuf<Box<[u8; 16]>> = FixedBuf::new();
        buf.write_str("abc").unwrap();
        assert_eq!("abc", escape_ascii(buf.readable()));
        let mem = buf.into_inner();
        buf = FixedBuf::new_with_mem(mem);
        assert_eq!("", escape_ascii(buf.readable()));
        buf.wrote(3);
        assert_eq!("abc", escape_ascii(buf.read_all()));
        assert_eq!("", escape_ascii(buf.readable()));
    }

    #[test]
    fn test_slice_constructor() {
        let mut mem = [0u8; 15];
        let mut buf = FixedBuf::new_with_slice(&mut mem);
        buf.write_str("abc").unwrap();
        assert_eq!("abc", escape_ascii(buf.readable()));
        buf = FixedBuf::new_with_slice(&mut mem);
        assert_eq!("", escape_ascii(buf.readable()));
        buf.wrote(3);
        assert_eq!("abc", escape_ascii(buf.read_all()));
        assert_eq!("", escape_ascii(buf.readable()));
    }

    #[test]
    fn empty() {
        let mut buf: FixedBuf<[u8; 16]> = FixedBuf::new();
        assert_eq!("", escape_ascii(buf.readable()));
        assert_eq!("", escape_ascii(buf.read_all()));
        buf.shift();
        assert_eq!("", escape_ascii(buf.readable()));
        assert_eq!("", escape_ascii(buf.read_all()));
    }

    #[test]
    fn test_len() {
        let mut buf: FixedBuf<[u8; 16]> = FixedBuf::new();
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
    fn test_write_str() {
        let mut buf: FixedBuf<[u8; 16]> = FixedBuf::new();
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
        let mut buf: FixedBuf<[u8; 16]> = FixedBuf::new();
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
        let mut buf: FixedBuf<[u8; 16]> = FixedBuf::new();
        buf.wrote(17);
    }

    #[test]
    fn test_readable_and_read() {
        let mut buf: FixedBuf<[u8; 16]> = FixedBuf::new();
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
        let mut buf: FixedBuf<[u8; 16]> = FixedBuf::new();
        buf.write_str("a").unwrap();
        buf.read_bytes(2);
    }

    #[test]
    fn test_read_all() {
        let mut buf: FixedBuf<[u8; 16]> = FixedBuf::new();
        assert_eq!("", escape_ascii(buf.read_all()));
        buf.write_str("abc").unwrap();
        assert_eq!("abc", escape_ascii(buf.read_all()));
        buf.write_str("def").unwrap();
        assert_eq!("def", escape_ascii(buf.read_all()));
        assert_eq!("", escape_ascii(buf.read_all()));
    }

    #[tokio::test]
    async fn test_read_delimited_example() {
        let mut buf: FixedBuf<[u8; 16]> = FixedBuf::new();
        assert_eq!("", escape_ascii(buf.readable()));
        let mut input = std::io::Cursor::new(b"aaa\nbbb\n\nccc\n");
        assert_eq!(
            "aaa",
            escape_ascii(buf.read_delimited(&mut input, b"\n").await.unwrap())
        );
        assert_eq!(
            "bbb",
            escape_ascii(buf.read_delimited(&mut input, b"\n").await.unwrap())
        );
        assert_eq!(
            "",
            escape_ascii(buf.read_delimited(&mut input, b"\n").await.unwrap())
        );
        assert_eq!(
            "ccc",
            escape_ascii(buf.read_delimited(&mut input, b"\n").await.unwrap())
        );
        assert_eq!(
            std::io::ErrorKind::NotFound,
            buf.read_delimited(&mut input, b"\n")
                .await
                .unwrap_err()
                .kind()
        );
    }

    #[tokio::test]
    async fn test_read_delimited_empty() {
        let mut buf: FixedBuf<[u8; 16]> = FixedBuf::new();
        assert_eq!(
            std::io::ErrorKind::NotFound,
            buf.read_delimited(&mut std::io::Cursor::new(""), b"b")
                .await
                .unwrap_err()
                .kind()
        );
    }

    #[tokio::test]
    async fn test_read_delimited_not_found_eof() {
        let mut buf: FixedBuf<[u8; 16]> = FixedBuf::new();
        assert_eq!(
            std::io::ErrorKind::NotFound,
            buf.read_delimited(&mut std::io::Cursor::new("abc"), b"d")
                .await
                .unwrap_err()
                .kind()
        );
        buf.read_all();
    }

    #[tokio::test]
    async fn test_read_delimited_not_found_buffer_almost_full() {
        let mut buf: FixedBuf<[u8; 16]> = FixedBuf::new();
        assert_eq!(
            std::io::ErrorKind::NotFound,
            buf.read_delimited(&mut std::io::Cursor::new(&"b".repeat(15)), b"d")
                .await
                .unwrap_err()
                .kind()
        );
    }

    #[tokio::test]
    async fn test_read_delimited_not_found_buffer_full() {
        let mut buf: FixedBuf<[u8; 16]> = FixedBuf::new();
        assert_eq!(
            std::io::ErrorKind::InvalidData,
            buf.read_delimited(&mut std::io::Cursor::new(&"b".repeat(16)), b"d")
                .await
                .unwrap_err()
                .kind()
        );
    }

    #[tokio::test]
    async fn test_read_delimited_found() {
        let mut buf: FixedBuf<[u8; 16]> = FixedBuf::new();
        assert_eq!(
            "ab",
            escape_ascii(
                buf.read_delimited(&mut std::io::Cursor::new("abc"), b"c")
                    .await
                    .unwrap()
            )
        );
    }

    #[tokio::test]
    async fn test_read_delimited_found_with_leftover() {
        let mut buf: FixedBuf<[u8; 16]> = FixedBuf::new();
        assert_eq!(
            "ab",
            escape_ascii(
                buf.read_delimited(&mut std::io::Cursor::new("abcdef"), b"c")
                    .await
                    .unwrap()
            )
        );
        assert_eq!("def", escape_ascii(buf.read_all()));
    }

    struct AsyncReadableThatPanics;

    impl tokio::io::AsyncRead for AsyncReadableThatPanics {
        fn poll_read(
            self: Pin<&mut Self>,
            _cx: &mut Context<'_>,
            _buf: &mut tokio::io::ReadBuf<'_>,
        ) -> Poll<Result<(), std::io::Error>> {
            panic!("AsyncReadableThatPanics::poll_read called");
        }
    }

    #[tokio::test]
    async fn test_read_delimited_already_in_buffer() {
        let mut buf: FixedBuf<[u8; 16]> = FixedBuf::new();
        buf.write_str("abc").unwrap();
        let mut input = AsyncReadableThatPanics {};
        assert_eq!(
            "ab",
            escape_ascii(buf.read_delimited(&mut input, b"c").await.unwrap())
        );

        buf.write_str("aaxbbx").unwrap();
        assert_eq!(
            "aa",
            escape_ascii(buf.read_delimited(&mut input, b"x").await.unwrap())
        );
        assert_eq!(
            "bb",
            escape_ascii(buf.read_delimited(&mut input, b"x").await.unwrap())
        );
    }

    #[test]
    fn test_std_io_write() {
        let mut buf: FixedBuf<[u8; 16]> = FixedBuf::new();
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

    #[tokio::test]
    async fn test_async_write() {
        let mut buf: FixedBuf<[u8; 16]> = FixedBuf::new();
        tokio::io::AsyncWriteExt::write_all(&mut buf, b"abc")
            .await
            .unwrap();
        assert_eq!("abc", escape_ascii(buf.readable()));
        tokio::io::AsyncWriteExt::write_all(&mut buf, b"def")
            .await
            .unwrap();
        assert_eq!("abcdef", escape_ascii(buf.readable()));
        buf.read_bytes(1);
        tokio::io::AsyncWriteExt::write_all(&mut buf, b"g")
            .await
            .unwrap();
        assert_eq!("bcdefg", escape_ascii(buf.readable()));
        tokio::io::AsyncWriteExt::write_all(&mut buf, "h".repeat(8).as_bytes())
            .await
            .unwrap();
        tokio::io::AsyncWriteExt::write_all(&mut buf, b"i")
            .await
            .unwrap();
        assert_eq!(
            std::io::ErrorKind::InvalidData,
            tokio::io::AsyncWriteExt::write_all(&mut buf, b"def")
                .await
                .unwrap_err()
                .kind()
        );
    }

    #[test]
    fn test_std_io_read() {
        let mut buf: FixedBuf<[u8; 16]> = FixedBuf::new();
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

    #[tokio::test]
    async fn test_async_read() {
        let mut buf: FixedBuf<[u8; 16]> = FixedBuf::new();
        let mut data = ['.' as u8; 16];
        assert_eq!(
            0,
            tokio::io::AsyncReadExt::read(&mut buf, &mut data)
                .await
                .unwrap()
        );
        assert_eq!("..........", escape_ascii(&data[..10]));
        buf.write_str("abc").unwrap();
        assert_eq!(
            3,
            tokio::io::AsyncReadExt::read(&mut buf, &mut data)
                .await
                .unwrap()
        );
        assert_eq!("abc.......", escape_ascii(&data[..10]));
        assert_eq!(
            0,
            tokio::io::AsyncReadExt::read(&mut buf, &mut data)
                .await
                .unwrap()
        );
        let many_bs = "b".repeat(16);
        buf.write_str(&many_bs).unwrap();
        assert_eq!(
            16,
            tokio::io::AsyncReadExt::read(&mut buf, &mut data)
                .await
                .unwrap()
        );
        assert_eq!(many_bs, escape_ascii(&data[..]));
        assert_eq!(
            0,
            tokio::io::AsyncReadExt::read(&mut buf, &mut data)
                .await
                .unwrap()
        );
    }

    #[test]
    fn test_default() {
        let array_buf: FixedBuf<[u8; 16]> = FixedBuf::default();
        assert_eq!("", escape_ascii(array_buf.readable()));
        let box_buf: FixedBuf<Box<[u8; 16]>> = FixedBuf::default();
        assert_eq!("", escape_ascii(box_buf.readable()));
        // let slice_buf: FixedBuf<&mut [u8]> = FixedBuf::default(); // compiler error
    }

    #[test]
    fn test_debug() {
        let mut array_buf: FixedBuf<[u8; 8]> = FixedBuf::default();
        let mut box_buf: FixedBuf<Box<[u8; 16]>> = FixedBuf::new();
        let mut mem = [0u8; 15];
        let mut slice_buf = FixedBuf::new_with_slice(&mut mem);
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

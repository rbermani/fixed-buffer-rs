use core::pin::Pin;
use core::task::{Context, Poll};

/// The number of bytes that can be stored in an FixedBuf.
pub const BUFFER_LEN: usize = 4 * 1024;

/// FixedBuf is a byte buffer that holds a few KB.
/// You can write bytes to it and then read them back.
///
/// It implements tokio's [`AsyncRead`] and [`AsyncWrite`] traits.
///
/// Use [`read_delimited`] to read lines and other delimited messages.
/// This works like [`tokio::io::AsyncBufReadExt::read_until`],
/// but uses a fixed sized buffer so network peers cannot OOM the process.
///
/// The buffer comes from the stack.  Put it in a [`Box`] to put it on the heap.
///
/// It is not a circular buffer.  You can call [`shift`] periodically to
/// move unread bytes to the front of the buffer.
///
/// [`Box`]: https://doc.rust-lang.org/std/boxed/struct.Box.html
/// [`read_delimited`]: #method.read_delimited
/// [`shift`]: #method.shift
/// [`AsyncRead`]: https://docs.rs/tokio/0.3.0/tokio/io/trait.AsyncRead.html
/// [`AsyncWrite`]: https://docs.rs/tokio/0.3.0/tokio/io/trait.AsyncWrite.html
/// [`tokio::io::AsyncBufReadExt::read_until`]: https://docs.rs/tokio/latest/tokio/io/trait.AsyncBufReadExt.html
#[derive(Clone, Copy, Eq, Hash, PartialEq)]
pub struct FixedBuf {
    buf: [u8; BUFFER_LEN],
    write_index: usize,
    read_index: usize,
}

impl FixedBuf {
    /// Makes a new buffer with a few 4KB of internal memory.
    ///
    /// Allocates on the stack by default.  Put it in a [`Box`] to use the heap.
    ///
    /// [`Box`]: https://doc.rust-lang.org/std/boxed/struct.Box.html
    pub fn new() -> FixedBuf {
        FixedBuf {
            buf: [0; BUFFER_LEN],
            write_index: 0,
            read_index: 0,
        }
    }

    /// Makes a new FixedBuf which uses the specified memory.
    ///
    /// This function is the inverse of [`into_inner`].
    ///
    /// [`into_inner`]: #method.into_inner
    pub fn with_mem(buf: [u8; BUFFER_LEN]) -> FixedBuf {
        FixedBuf {
            buf,
            write_index: 0,
            read_index: 0,
        }
    }

    /// Drops the struct and returns its internal memory.
    ///
    /// This function is the inverse of [`with_mem`].
    ///
    /// [`with_mem`]: #method.with_mem
    pub fn into_inner(self) -> [u8; BUFFER_LEN] {
        self.buf
    }

    /// Returns the number of unread bytes in the buffer.
    pub fn len(&self) -> usize {
        self.write_index - self.read_index
    }

    /// Writes `s` into the buffer, after any unread bytes.
    ///
    /// Panics if the buffer doesn't have enough free space at the end for the whole string.
    pub fn append(&mut self, s: &str) {
        std::io::Write::write(self, s.as_bytes()).unwrap();
    }

    /// Writes `s` into the buffer, after any unread bytes.
    ///
    /// Returns [`None`] if the buffer doesn't have enough free space at the end for the whole string.
    ///
    /// [`None`]: https://doc.rust-lang.org/std/option/enum.Option.html
    pub fn try_append(&mut self, s: &str) -> Option<()> {
        std::io::Write::write(self, s.as_bytes()).ok().map(|_| ())
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
    /// [`shift`]: #method.shift
    /// [`wrote(usize)`]: #method.wrote
    /// [`None`]: https://doc.rust-lang.org/std/option/enum.Option.html
    /// [`std::io::Write::write`]: https://doc.rust-lang.org/std/io/trait.Write.html#tymethod.write
    /// [`tokio::io::AsyncWriteExt::write`]: https://docs.rs/tokio/0.3.0/tokio/io/trait.AsyncWriteExt.html#method.write
    pub fn writable(&mut self) -> Option<&mut [u8]> {
        if self.write_index >= self.buf.len() {
            // buf ran out of space.
            return None;
        }
        Some(&mut self.buf[self.write_index..])
    }

    /// Commit bytes into the buffer.
    /// Call this after writing to the front of the [`writable`] slice.
    ///
    /// This is a low-level method.
    ///
    /// Panics if [`writable()`] is not large enough.
    ///
    /// [`writable`]: #method.writable
    /// [`writable()`]: #method.writable
    pub fn wrote(&mut self, num_bytes: usize) {
        if num_bytes == 0 {
            return;
        }
        let new_write_index = self.write_index + num_bytes;
        if new_write_index > self.buf.len() {
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
    /// ```rust
    /// # use fixed_buffer::FixedBuf;
    /// # use std::io::{Error, ErrorKind};
    /// # use tokio::io::AsyncReadExt;
    /// # fn try_process_record(b: &[u8]) -> Result<usize, Error> { Ok(0) }
    /// # async fn f<R: tokio::io::AsyncRead + Unpin>(mut input: R) -> Result<(), Error> {
    /// let mut buf: FixedBuf = FixedBuf::new();
    /// loop {
    ///     // Read a chunk into the buffer.
    ///     let mut writable = buf.writable()
    ///         .ok_or(Error::new(ErrorKind::InvalidData, "record too long, buffer full"))?;
    ///     let bytes_written = AsyncReadExt::read(&mut input, &mut writable).await?;
    ///     if bytes_written == 0 {
    ///         return Err(Error::from(ErrorKind::UnexpectedEof));
    ///     }
    ///     buf.wrote(bytes_written);
    ///
    ///     // Process records in the buffer.
    ///     loop {
    ///         let bytes_read = try_process_record(buf.readable())?;
    ///         if bytes_read == 0 {
    ///             break;
    ///         }
    ///         buf.read(bytes_read);
    ///     }
    ///     // Shift data in the buffer to free up space at the end for writing.
    ///     buf.shift();
    /// }
    /// # }
    /// ```
    ///
    /// [`read`]: #method.read
    /// [`std::io::Read::read`]: https://doc.rust-lang.org/std/io/trait.Read.html#tymethod.read
    /// [`tokio::io::AsyncReadExt::read`]: https://docs.rs/tokio/0.3.0/tokio/io/trait.AsyncReadExt.html#method.read
    pub fn readable(&self) -> &[u8] {
        &self.buf[self.read_index..self.write_index]
    }

    /// Read bytes from the buffer.
    ///
    /// Panics if the buffer does not contain enough bytes.
    pub fn read(&mut self, num_bytes: usize) -> &[u8] {
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
        &self.buf[old_read_index..new_read_index]
    }

    /// Read all the bytes from the buffer.
    /// The buffer becomes empty and subsequent writes can fill the whole buffer.
    pub fn read_all(&mut self) -> &[u8] {
        self.read(self.len())
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
    /// Returns Err(InvalidData) if the buffer fills up before `delim` is found.
    ///
    /// Demo:
    /// ```rust
    /// # use fixed_buffer::{escape_ascii, FixedBuf};
    /// # async fn f() {
    /// let mut buf: FixedBuf = FixedBuf::new();
    /// let mut input = std::io::Cursor::new(b"aaa\nbbb\n\nccc\n");
    /// assert_eq!("aaa", escape_ascii(buf.read_delimited(&mut input, b"\n").await.unwrap()));
    /// assert_eq!("bbb", escape_ascii(buf.read_delimited(&mut input, b"\n").await.unwrap()));
    /// assert_eq!("",    escape_ascii(buf.read_delimited(&mut input, b"\n").await.unwrap()));
    /// assert_eq!("ccc", escape_ascii(buf.read_delimited(&mut input, b"\n").await.unwrap()));
    /// assert_eq!(
    ///     std::io::ErrorKind::NotFound,
    ///     buf.read_delimited(&mut input, b"\n").await.unwrap_err().kind()
    /// );
    /// # }
    /// ```
    ///
    /// Example usage:
    /// ```rust
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
    /// let mut buf: FixedBuf = FixedBuf::new();
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
    pub async fn read_delimited<'b, T>(
        &mut self,
        mut input: T,
        delim: &[u8],
    ) -> std::io::Result<&[u8]>
    where
        T: tokio::io::AsyncRead + std::marker::Unpin,
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
                self.read(delim_index + delim.len());
                return Ok(&self.buf[result_start..result_end]);
            }
            self.shift();
            let writable = self.writable().ok_or(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "end of buffer full",
            ))?;
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
        self.buf.copy_within(self.read_index..self.write_index, 0);
        self.write_index -= self.read_index;
        self.read_index = 0;
    }
}

impl std::io::Write for FixedBuf {
    fn write(&mut self, data: &[u8]) -> std::io::Result<usize> {
        let writable = self.writable().ok_or(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            "end of buffer full",
        ))?;
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

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

impl std::io::Read for FixedBuf {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        let readable = self.readable();
        let len = core::cmp::min(buf.len(), readable.len());
        if len == 0 {
            return Ok(0);
        }
        let src = &readable[..len];
        let dest = &mut buf[..len];
        dest.copy_from_slice(src);
        self.read(len);
        Ok(len)
    }
}

impl tokio::io::AsyncWrite for FixedBuf {
    fn poll_write(
        self: Pin<&mut Self>,
        _cx: &mut Context<'_>,
        buf: &[u8],
    ) -> Poll<Result<usize, std::io::Error>> {
        Poll::Ready(std::io::Write::write(self.get_mut(), buf))
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

impl tokio::io::AsyncRead for FixedBuf {
    fn poll_read(
        self: Pin<&mut Self>,
        _cx: &mut Context<'_>,
        buf: &mut tokio::io::ReadBuf<'_>,
    ) -> Poll<Result<(), std::io::Error>> {
        Poll::Ready(
            std::io::Read::read(self.get_mut(), buf.initialize_unfilled()).map(|n| buf.advance(n)),
        )
    }
}

impl Default for FixedBuf {
    fn default() -> Self {
        Self::new()
    }
}

/// Convert a byte slice into a string.
/// Includes printable ASCII characters as-is.
/// Converts non-printable or non-ASCII characters to strings like "\n" and "\x19".
///
/// Uses [`std::ascii::escape_default`] internally to escape each byte.
///
/// This function is useful for printing byte slices to logs and comparing byte slices in tests.
///
/// Example test:
/// ```rust
/// #[test]
/// fn test_append() {
/// #   use fixed_buffer::{escape_ascii, FixedBuf};
///     let mut buf = FixedBuf::new();
///     buf.append("ab");
///     buf.append("cd");
///     assert_eq!("abcd", escape_ascii(buf.readable()));
/// }
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

impl std::fmt::Debug for FixedBuf {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "FixedBuf{{{} writable, {} readable: \"{}\"}}",
            BUFFER_LEN - self.write_index,
            self.len(),
            escape_ascii(self.readable())
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_constructors() {
        let mut buf = FixedBuf::new();
        buf.append("abc");
        assert_eq!("abc", escape_ascii(buf.readable()));
        let mem = buf.into_inner();
        buf = FixedBuf::with_mem(mem);
        assert_eq!("", escape_ascii(buf.readable()));
        buf.wrote(3);
        assert_eq!("abc", escape_ascii(buf.read_all()));
        assert_eq!("", escape_ascii(buf.readable()));
    }

    #[test]
    fn empty() {
        let mut buf = FixedBuf::new();
        assert_eq!("", escape_ascii(buf.readable()));
        assert_eq!("", escape_ascii(buf.read_all()));
        buf.shift();
        assert_eq!("", escape_ascii(buf.readable()));
        assert_eq!("", escape_ascii(buf.read_all()));
    }

    #[test]
    fn test_len() {
        let mut buf = FixedBuf::new();
        assert_eq!(0, buf.len());
        buf.append("abc");
        assert_eq!(3, buf.len());
        buf.read(2);
        assert_eq!(1, buf.len());
        buf.shift();
        assert_eq!(1, buf.len());
        buf.read_all();
        assert_eq!(0, buf.len());
    }

    #[test]
    fn test_append() {
        let mut buf = FixedBuf::new();
        buf.append("abc");
        assert_eq!("abc", escape_ascii(buf.readable()));
        buf.append("def");
        assert_eq!("abcdef", escape_ascii(buf.readable()));
        buf.append(&"g".repeat(BUFFER_LEN - 6));
    }

    #[test]
    #[should_panic]
    fn test_append_buffer_full() {
        let mut buf = FixedBuf::new();
        buf.append(&"c".repeat(BUFFER_LEN + 1));
    }

    #[test]
    fn test_try_append() {
        let mut buf = FixedBuf::new();
        buf.try_append("a").unwrap();
        buf.append("b");
        assert_eq!("ab", escape_ascii(buf.readable()));
        let many_cs = "c".repeat(BUFFER_LEN - 3);
        buf.try_append(&many_cs).unwrap();
        buf.try_append("d").unwrap();
        assert_eq!(
            "ab".to_string() + &many_cs + "d",
            escape_ascii(buf.readable())
        );
        assert_eq!(None, buf.try_append("e"));
    }

    #[test]
    fn test_writable_and_wrote() {
        let mut buf = FixedBuf::new();
        assert_eq!(BUFFER_LEN, buf.writable().unwrap().len());
        buf.writable().unwrap()[0] = 'a' as u8;
        buf.wrote(1);
        assert_eq!("a", escape_ascii(buf.readable()));
        let many_bs = "b".repeat(BUFFER_LEN - 1);
        assert_eq!(many_bs.len(), buf.writable().unwrap().len());
        buf.writable().unwrap().copy_from_slice(many_bs.as_bytes());
        buf.wrote(many_bs.len());
        assert_eq!("a".to_string() + &many_bs, escape_ascii(buf.readable()));
        assert_eq!(None, buf.writable());
    }

    #[test]
    #[should_panic]
    fn test_wrote_too_much() {
        let mut buf = FixedBuf::new();
        buf.wrote(BUFFER_LEN + 1);
    }

    #[test]
    fn test_readable_and_read() {
        let mut buf = FixedBuf::new();
        assert_eq!("", escape_ascii(buf.readable()));
        buf.append("abc");
        assert_eq!("abc", escape_ascii(buf.readable()));
        buf.read(1);
        assert_eq!("bc", escape_ascii(buf.readable()));
        buf.read(2);
        assert_eq!("", escape_ascii(buf.readable()));
        buf.append("d");
        assert_eq!("d", escape_ascii(buf.readable()));
        buf.read(1);
        assert_eq!("", escape_ascii(buf.readable()));
    }

    #[test]
    #[should_panic]
    fn test_read_too_much() {
        let mut buf = FixedBuf::new();
        buf.append("a");
        buf.read(2);
    }

    #[test]
    fn test_read_all() {
        let mut buf = FixedBuf::new();
        assert_eq!("", escape_ascii(buf.read_all()));
        buf.append("abc");
        assert_eq!("abc", escape_ascii(buf.read_all()));
        buf.append("def");
        assert_eq!("def", escape_ascii(buf.read_all()));
        assert_eq!("", escape_ascii(buf.read_all()));
    }

    #[tokio::test]
    async fn test_read_delimited_example() {
        let mut buf: FixedBuf = FixedBuf::new();
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
        let mut buf = FixedBuf::new();
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
        let mut buf = FixedBuf::new();
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
        let mut buf = FixedBuf::new();
        assert_eq!(
            std::io::ErrorKind::NotFound,
            buf.read_delimited(&mut std::io::Cursor::new(&"b".repeat(BUFFER_LEN - 1)), b"d")
                .await
                .unwrap_err()
                .kind()
        );
    }

    #[tokio::test]
    async fn test_read_delimited_not_found_buffer_full() {
        let mut buf = FixedBuf::new();
        assert_eq!(
            std::io::ErrorKind::InvalidData,
            buf.read_delimited(&mut std::io::Cursor::new(&"b".repeat(BUFFER_LEN)), b"d")
                .await
                .unwrap_err()
                .kind()
        );
    }

    #[tokio::test]
    async fn test_read_delimited_found() {
        let mut buf = FixedBuf::new();
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
        let mut buf = FixedBuf::new();
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
        let mut buf = FixedBuf::new();
        buf.append("abc");
        let mut input = AsyncReadableThatPanics {};
        assert_eq!(
            "ab",
            escape_ascii(buf.read_delimited(&mut input, b"c").await.unwrap())
        );

        buf.append("aaxbbx");
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
        let mut buf = FixedBuf::new();
        std::io::Write::write(&mut buf, b"abc").unwrap();
        assert_eq!("abc", escape_ascii(buf.readable()));
        std::io::Write::write(&mut buf, b"def").unwrap();
        assert_eq!("abcdef", escape_ascii(buf.readable()));
        buf.read(1);
        std::io::Write::write(&mut buf, b"g").unwrap();
        assert_eq!("bcdefg", escape_ascii(buf.readable()));
        std::io::Write::write(&mut buf, "h".repeat(BUFFER_LEN - 8).as_bytes()).unwrap();
        std::io::Write::write(&mut buf, b"i").unwrap();
        assert_eq!(
            std::io::ErrorKind::InvalidData,
            std::io::Write::write(&mut buf, b"def").unwrap_err().kind()
        );
    }

    #[tokio::test]
    async fn test_async_write() {
        let mut buf = FixedBuf::new();
        tokio::io::AsyncWriteExt::write_all(&mut buf, b"abc")
            .await
            .unwrap();
        assert_eq!("abc", escape_ascii(buf.readable()));
        tokio::io::AsyncWriteExt::write_all(&mut buf, b"def")
            .await
            .unwrap();
        assert_eq!("abcdef", escape_ascii(buf.readable()));
        buf.read(1);
        tokio::io::AsyncWriteExt::write_all(&mut buf, b"g")
            .await
            .unwrap();
        assert_eq!("bcdefg", escape_ascii(buf.readable()));
        tokio::io::AsyncWriteExt::write_all(&mut buf, "h".repeat(BUFFER_LEN - 8).as_bytes())
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
        let mut buf = FixedBuf::new();
        let mut data: [u8; BUFFER_LEN] = ['.' as u8; BUFFER_LEN];
        assert_eq!(0, std::io::Read::read(&mut buf, &mut data).unwrap());
        assert_eq!("..........", escape_ascii(&data[..10]));
        buf.append("abc");
        assert_eq!(3, std::io::Read::read(&mut buf, &mut data).unwrap());
        assert_eq!("abc.......", escape_ascii(&data[..10]));
        assert_eq!(0, std::io::Read::read(&mut buf, &mut data).unwrap());
        let many_bs = "b".repeat(BUFFER_LEN);
        buf.append(&many_bs);
        assert_eq!(
            BUFFER_LEN,
            std::io::Read::read(&mut buf, &mut data).unwrap()
        );
        assert_eq!(many_bs, escape_ascii(&data[..]));
        assert_eq!(0, std::io::Read::read(&mut buf, &mut data).unwrap());
    }

    #[tokio::test]
    async fn test_async_read() {
        let mut buf = FixedBuf::new();
        let mut data: [u8; BUFFER_LEN] = ['.' as u8; BUFFER_LEN];
        assert_eq!(
            0,
            tokio::io::AsyncReadExt::read(&mut buf, &mut data)
                .await
                .unwrap()
        );
        assert_eq!("..........", escape_ascii(&data[..10]));
        buf.append("abc");
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
        let many_bs = "b".repeat(BUFFER_LEN);
        buf.append(&many_bs);
        assert_eq!(
            BUFFER_LEN,
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
        let buf = FixedBuf::default();
        assert_eq!("", escape_ascii(buf.readable()));
    }

    #[test]
    fn test_debug() {
        let mut buf = FixedBuf::default();
        buf.append("abc");
        buf.read(1);
        assert_eq!(
            "FixedBuf{4093 writable, 2 readable: \"bc\"}",
            format!("{:?}", buf)
        );
    }
}

#![forbid(unsafe_code)]

// TODO(mleonhard) Test.
/// A wrapper for a pair of structs.
/// The first implements [`Read`](https://doc.rust-lang.org/stable/std/io/trait.Read.html).
/// The second implements
/// [`Read`](https://doc.rust-lang.org/stable/std/io/trait.Read.html)+[`Write`](https://doc.rust-lang.org/stable/std/io/trait.Write.html).
///
/// Passes reads through to the Read.
/// Once the Read returns EOF, passes reads to Read+Write.
///
/// Passes all writes through to the Read+Write.
///
/// This is like [`std::io::Chain`](https://doc.rust-lang.org/stable/std/io/struct.Chain.html)
/// that passes through writes.
/// This makes it usable with Read+Write objects like
/// [`std::net::TcpStream`](https://doc.rust-lang.org/stable/std/net/struct.TcpStream.html)
/// and [`rustls::Stream`](https://docs.rs/rustls/latest/rustls/struct.Stream.html).
pub struct ReadWriteChain<'a, R: std::io::Read, RW: std::io::Read + std::io::Write> {
    reader: Option<&'a mut R>,
    read_writer: &'a mut RW,
}

impl<'a, R: std::io::Read, RW: std::io::Read + std::io::Write> ReadWriteChain<'a, R, RW> {
    /// See [`ReadWriteChain`](struct.ReadWriteChain.html).
    pub fn new(reader: &'a mut R, read_writer: &'a mut RW) -> ReadWriteChain<'a, R, RW> {
        Self {
            reader: Some(reader),
            read_writer,
        }
    }
}

impl<'a, R: std::io::Read, RW: std::io::Read + std::io::Write> std::io::Read
    for ReadWriteChain<'a, R, RW>
{
    fn read(&mut self, buf: &mut [u8]) -> Result<usize, std::io::Error> {
        if let Some(ref mut reader) = self.reader {
            match reader.read(buf) {
                Ok(0) => {
                    // EOF
                    self.reader = None;
                }
                Ok(num_read) => return Ok(num_read),
                Err(e) => return Err(e),
            }
        }
        self.read_writer.read(buf)
    }
}

impl<'a, R: std::io::Read, RW: std::io::Read + std::io::Write> std::io::Write
    for ReadWriteChain<'a, R, RW>
{
    fn write(&mut self, buf: &[u8]) -> Result<usize, std::io::Error> {
        self.read_writer.write(buf)
    }

    fn flush(&mut self) -> Result<(), std::io::Error> {
        self.read_writer.flush()
    }
}

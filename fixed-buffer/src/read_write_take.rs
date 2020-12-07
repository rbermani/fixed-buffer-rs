#![forbid(unsafe_code)]

// TODO(mleonhard) Test.
/// Wraps a struct that implements
/// [`Read`](https://doc.rust-lang.org/stable/std/io/trait.Read.html)+[`Write`](https://doc.rust-lang.org/stable/std/io/trait.Write.html).
/// Passes through reads and writes to the struct.
/// Limits the number of bytes that can be read.
///
/// This is like [`std::io::Take`](https://doc.rust-lang.org/stable/std/io/struct.Take.html)
/// that passes through writes.
/// This makes it usable with Read+Write objects like
/// [`std::net::TcpStream`](https://doc.rust-lang.org/stable/std/net/struct.TcpStream.html)
/// and [`rustls::Stream`](https://docs.rs/rustls/latest/rustls/struct.Stream.html).
pub struct ReadWriteTake<'a, RW: std::io::Read + std::io::Write> {
    read_writer: &'a mut RW,
    remaining_bytes: u64,
}

impl<'a, RW: std::io::Read + std::io::Write> ReadWriteTake<'a, RW> {
    /// See [`ReadWriteTake`](struct.ReadWriteTake.html).
    pub fn new(read_writer: &'a mut RW, len: u64) -> ReadWriteTake<'a, RW> {
        Self {
            read_writer,
            remaining_bytes: len,
        }
    }
}

impl<'a, RW: std::io::Read + std::io::Write> std::io::Read for ReadWriteTake<'a, RW> {
    fn read(&mut self, buf: &mut [u8]) -> Result<usize, std::io::Error> {
        if self.remaining_bytes == 0 {
            return Ok(0);
        }
        let num_to_read = self.remaining_bytes.min(buf.len() as u64) as usize;
        let mut dest = &mut buf[0..num_to_read];
        match self.read_writer.read(&mut dest) {
            Ok(num_read) => {
                self.remaining_bytes -= num_read as u64;
                Ok(num_read)
            }
            Err(e) => Err(e),
        }
    }
}

impl<'a, RW: std::io::Read + std::io::Write> std::io::Write for ReadWriteTake<'a, RW> {
    fn write(&mut self, buf: &[u8]) -> Result<usize, std::io::Error> {
        self.read_writer.write(buf)
    }

    fn flush(&mut self) -> Result<(), std::io::Error> {
        self.read_writer.flush()
    }
}

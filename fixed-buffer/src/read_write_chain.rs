#![forbid(unsafe_code)]

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

#[cfg(test)]
mod tests {
    use super::super::*;

    #[test]
    fn both_empty() {
        let mut reader = std::io::Cursor::new(b"");
        let mut read_writer = FixedBuf::new([0u8; 8]);
        let mut chain = ReadWriteChain::new(&mut reader, &mut read_writer);
        let mut buf = [b'.'; 8];
        assert_eq!(0, std::io::Read::read(&mut chain, &mut buf).unwrap());
        assert_eq!("........", escape_ascii(&buf));
    }

    #[test]
    fn doesnt_read_second_when_first_has_data() {
        let mut reader = std::io::Cursor::new(b"abc");
        let mut read_writer = FakeReadWriter::empty();
        let mut chain = ReadWriteChain::new(&mut reader, &mut read_writer);
        let mut buf = [b'.'; 4];
        assert_eq!(3, std::io::Read::read(&mut chain, &mut buf).unwrap());
        assert_eq!("abc.", escape_ascii(&buf));
    }

    #[test]
    fn doesnt_read_second_when_first_returns_error() {
        let mut reader = FakeReadWriter::new(vec![Err(err1()), Err(err1())]);
        let mut read_writer = FakeReadWriter::empty();
        let mut chain = ReadWriteChain::new(&mut reader, &mut read_writer);
        let mut buf = [b'.'; 4];
        let err = std::io::Read::read(&mut chain, &mut buf).unwrap_err();
        assert_eq!(std::io::ErrorKind::Other, err.kind());
        assert_eq!("err1", err.to_string());
        assert_eq!("....", escape_ascii(&buf));
        std::io::Read::read(&mut chain, &mut buf).unwrap_err();
    }

    #[test]
    fn reads_second_when_first_empty() {
        let mut reader = std::io::Cursor::new(b"");
        let mut read_writer = FixedBuf::new([0u8; 4]);
        read_writer.write_str("abc").unwrap();
        let mut chain = ReadWriteChain::new(&mut reader, &mut read_writer);
        let mut buf = [b'.'; 4];
        assert_eq!(3, std::io::Read::read(&mut chain, &mut buf).unwrap());
        assert_eq!("abc.", escape_ascii(&buf));
    }

    #[test]
    fn reads_first_then_second() {
        let mut reader = std::io::Cursor::new(b"ab");
        let mut read_writer = FixedBuf::new([0u8; 4]);
        read_writer.write_str("cd").unwrap();
        let mut chain = ReadWriteChain::new(&mut reader, &mut read_writer);
        let mut buf = [b'.'; 4];
        assert_eq!(2, std::io::Read::read(&mut chain, &mut buf).unwrap());
        assert_eq!("ab..", escape_ascii(&buf));
        assert_eq!(2, std::io::Read::read(&mut chain, &mut buf).unwrap());
        assert_eq!("cd..", escape_ascii(&buf));
    }

    #[test]
    fn returns_error_from_second() {
        let mut reader = std::io::Cursor::new(b"");
        let mut read_writer = FakeReadWriter::new(vec![Err(err1()), Err(err1())]);
        let mut chain = ReadWriteChain::new(&mut reader, &mut read_writer);
        let mut buf = [b'.'; 4];
        let err = std::io::Read::read(&mut chain, &mut buf).unwrap_err();
        assert_eq!(std::io::ErrorKind::Other, err.kind());
        assert_eq!("err1", err.to_string());
        assert_eq!("....", escape_ascii(&buf));
        std::io::Read::read(&mut chain, &mut buf).unwrap_err();
    }

    #[test]
    fn passes_writes_through() {
        let mut reader = std::io::Cursor::new(b"");
        let mut read_writer = FixedBuf::new([0u8; 4]);
        let mut chain = ReadWriteChain::new(&mut reader, &mut read_writer);
        assert_eq!(3, std::io::Write::write(&mut chain, b"abc").unwrap());
        assert_eq!("abc", read_writer.escape_ascii());
    }
}

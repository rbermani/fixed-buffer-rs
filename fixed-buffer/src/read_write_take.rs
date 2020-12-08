#![forbid(unsafe_code)]

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

#[cfg(test)]
mod tests {
    use super::super::*;

    #[test]
    fn read_error() {
        let mut read_writer = FakeReadWriter::new(vec![Err(err1()), Ok(2), Ok(0)]);
        let mut take = ReadWriteTake::new(&mut read_writer, 3);
        let mut buf = [b'.'; 4];
        assert_eq!(
            "err1",
            std::io::Read::read(&mut take, &mut buf)
                .unwrap_err()
                .to_string()
        );
        assert_eq!(2, std::io::Read::read(&mut take, &mut buf).unwrap());
        assert_eq!("ab..", escape_ascii(&buf));
        assert_eq!(0, std::io::Read::read(&mut take, &mut buf).unwrap());
        assert_eq!("ab..", escape_ascii(&buf));
    }

    #[test]
    fn empty() {
        let mut read_writer = FakeReadWriter::new(vec![Ok(0)]);
        let mut take = ReadWriteTake::new(&mut read_writer, 3);
        let mut buf = [b'.'; 4];
        assert_eq!(0, std::io::Read::read(&mut take, &mut buf).unwrap());
        assert_eq!("....", escape_ascii(&buf));
    }

    #[test]
    fn doesnt_read_when_zero() {
        let mut read_writer = FakeReadWriter::empty();
        let mut take = ReadWriteTake::new(&mut read_writer, 0);
        let mut buf = [b'.'; 4];
        assert_eq!(0, std::io::Read::read(&mut take, &mut buf).unwrap());
        assert_eq!("....", escape_ascii(&buf));
    }

    #[test]
    fn fewer_than_len() {
        let mut read_writer = FakeReadWriter::new(vec![Ok(2), Ok(0)]);
        let mut take = ReadWriteTake::new(&mut read_writer, 3);
        let mut buf = [b'.'; 4];
        assert_eq!(2, std::io::Read::read(&mut take, &mut buf).unwrap());
        assert_eq!("ab..", escape_ascii(&buf));
        assert_eq!(0, std::io::Read::read(&mut take, &mut buf).unwrap());
        assert_eq!("ab..", escape_ascii(&buf));
    }

    #[test]
    fn fewer_than_len_in_multiple_reads() {
        let mut read_writer = FakeReadWriter::new(vec![Ok(2), Ok(2), Ok(0)]);
        let mut take = ReadWriteTake::new(&mut read_writer, 5);
        let mut buf = [b'.'; 4];
        assert_eq!(2, std::io::Read::read(&mut take, &mut buf).unwrap());
        assert_eq!("ab..", escape_ascii(&buf));
        assert_eq!(2, std::io::Read::read(&mut take, &mut buf).unwrap());
        assert_eq!("cd..", escape_ascii(&buf));
        assert_eq!(0, std::io::Read::read(&mut take, &mut buf).unwrap());
        assert_eq!("cd..", escape_ascii(&buf));
    }

    #[test]
    fn exactly_len() {
        let mut read_writer = FakeReadWriter::new(vec![Ok(3), Ok(0)]);
        let mut take = ReadWriteTake::new(&mut read_writer, 3);
        let mut buf = [b'.'; 4];
        assert_eq!(3, std::io::Read::read(&mut take, &mut buf).unwrap());
        assert_eq!("abc.", escape_ascii(&buf));
        assert_eq!(0, std::io::Read::read(&mut take, &mut buf).unwrap());
        assert_eq!("abc.", escape_ascii(&buf));
    }

    #[test]
    fn exactly_len_in_multiple_reads() {
        let mut read_writer = FakeReadWriter::new(vec![Ok(2), Ok(1), Ok(0)]);
        let mut take = ReadWriteTake::new(&mut read_writer, 3);
        let mut buf = [b'.'; 4];
        assert_eq!(2, std::io::Read::read(&mut take, &mut buf).unwrap());
        assert_eq!("ab..", escape_ascii(&buf));
        assert_eq!(1, std::io::Read::read(&mut take, &mut buf).unwrap());
        assert_eq!("cb..", escape_ascii(&buf));
        assert_eq!(0, std::io::Read::read(&mut take, &mut buf).unwrap());
        assert_eq!("cb..", escape_ascii(&buf));
    }

    #[test]
    fn doesnt_call_read_after_len_reached() {
        let mut read_writer = FakeReadWriter::new(vec![Ok(3)]);
        let mut take = ReadWriteTake::new(&mut read_writer, 3);
        let mut buf = [b'.'; 4];
        assert_eq!(3, std::io::Read::read(&mut take, &mut buf).unwrap());
        assert_eq!("abc.", escape_ascii(&buf));
        assert_eq!(0, std::io::Read::read(&mut take, &mut buf).unwrap());
        assert_eq!("abc.", escape_ascii(&buf));
    }

    #[test]
    fn doesnt_call_read_after_len_reached_in_multiple_reads() {
        let mut read_writer = FakeReadWriter::new(vec![Ok(2), Ok(1)]);
        let mut take = ReadWriteTake::new(&mut read_writer, 3);
        let mut buf = [b'.'; 4];
        assert_eq!(2, std::io::Read::read(&mut take, &mut buf).unwrap());
        assert_eq!("ab..", escape_ascii(&buf));
        assert_eq!(1, std::io::Read::read(&mut take, &mut buf).unwrap());
        assert_eq!("cb..", escape_ascii(&buf));
        assert_eq!(0, std::io::Read::read(&mut take, &mut buf).unwrap());
        assert_eq!("cb..", escape_ascii(&buf));
    }

    #[test]
    fn passes_writes_through() {
        let mut read_writer = FakeReadWriter::new(vec![Ok(3)]);
        let mut take = ReadWriteTake::new(&mut read_writer, 2);
        assert_eq!(3, std::io::Write::write(&mut take, b"abc").unwrap());
        assert!(read_writer.is_empty());
    }
}

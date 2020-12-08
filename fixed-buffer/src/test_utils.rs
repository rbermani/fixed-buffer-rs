pub fn err1() -> std::io::Error {
    std::io::Error::new(std::io::ErrorKind::Other, "err1")
}

pub struct FakeReadWriter {
    results: Vec<Result<usize, std::io::Error>>,
    byte_iter: Box<dyn std::iter::Iterator<Item = u8>>,
}

impl FakeReadWriter {
    fn make_byte_iter() -> Box<dyn std::iter::Iterator<Item = u8>> {
        Box::new(
            std::iter::repeat(b"abcdefghijklmnopqrstuvwxyz".iter().map(|ref_u8| *ref_u8)).flatten(),
        )
    }

    pub fn new(results: Vec<Result<usize, std::io::Error>>) -> Self {
        Self {
            results,
            byte_iter: Self::make_byte_iter(),
        }
    }

    pub fn empty() -> Self {
        Self {
            results: Vec::new(),
            byte_iter: Self::make_byte_iter(),
        }
    }

    pub fn is_empty(&self) -> bool {
        self.results.is_empty()
    }
}

impl std::io::Read for FakeReadWriter {
    fn read(&mut self, buf: &mut [u8]) -> Result<usize, std::io::Error> {
        if self.results.is_empty() {
            panic!("unexpected read");
        }
        let result = self.results.remove(0);
        if let Ok(num_read) = result {
            if num_read > 0 {
                let src: Vec<u8> =
                    std::iter::Iterator::take(&mut self.byte_iter, num_read).collect();
                let dest = &mut buf[0..num_read];
                dest.copy_from_slice(src.as_slice());
            }
        }
        result
    }
}

impl std::io::Write for FakeReadWriter {
    fn write(&mut self, _buf: &[u8]) -> Result<usize, std::io::Error> {
        if self.results.is_empty() {
            panic!("unexpected write");
        }
        self.results.remove(0)
    }

    fn flush(&mut self) -> Result<(), std::io::Error> {
        Ok(())
    }
}

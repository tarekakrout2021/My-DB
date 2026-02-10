use std::io::Write;

pub struct Output<'a> {
    f: &'a mut dyn Write,
}

impl<'a> Output<'a> {
    pub fn new(f: &'a mut dyn Write) -> Self {
        Self { f }
    }

    pub fn write(&mut self, s: &str) -> std::io::Result<()> {
        write!(self.f, "{s}")
    }

    pub fn writeln(&mut self, s: &str) -> std::io::Result<()> {
        writeln!(self.f, "{s}")
    }
}

impl Write for Output<'_> {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        self.f.write(buf)
    }

    fn flush(&mut self) -> std::io::Result<()> {
        self.f.flush()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::BufWriter;

    #[test]
    fn test_buffer() {
        let mut buf = BufWriter::new(Vec::new());
        let mut out = Output::new(&mut buf);

        // test macro usage
        write!(out, "hello").unwrap();

        // test write functions
        Output::write(&mut out, " world").unwrap();
        Output::writeln(&mut out, "!").unwrap();

        let bytes = buf.into_inner().unwrap();
        let string = String::from_utf8(bytes).unwrap();
        assert_eq!(string, "hello world!\n");
    }
}

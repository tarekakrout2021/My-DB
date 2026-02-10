use crate::types::{Tag, Type};
use crate::util::io::Output;
use std::error::Error;
use std::io::Write;

/// Timestamp type for storing date/time values.
#[derive(Copy, Clone, Debug, Eq, Hash, PartialEq, Ord, PartialOrd)]
pub struct Timestamp {
    /// The underlying timestamp value
    pub value: i64,
}

impl Timestamp {
    /// Creates a new Timestamp from an i64 value.
    pub fn new(value: i64) -> Self {
        Self { value }
    }

    /// Parses a string into a Timestamp.
    pub fn input(r#in: &str, r#type: Type) -> Result<Self, Box<dyn Error>> {
        assert_eq!(r#type.r#type(), Tag::Timestamp);

        Ok(Self {
            value: r#in.parse::<i64>()?,
        })
    }

    /// Writes a Timestamp to the output writer.
    pub fn output(writer: &mut Output, r#type: Type, out: Self) -> std::io::Result<()> {
        assert_eq!(r#type.r#type(), Tag::Timestamp);
        write!(writer, "{}", out.value)
    }
}

impl std::fmt::Display for Timestamp {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.value)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::BufWriter;

    macro_rules! output_eq {
        ($l:expr, $t:expr, $r:expr) => {
            let mut buf = BufWriter::new(Vec::new());
            let mut out = Output::new(&mut buf);
            Timestamp::output(&mut out, $t, $l).unwrap();
            let bytes = buf.into_inner().unwrap();
            let out = String::from_utf8(bytes).unwrap();
            assert_eq!(out, $r.to_string());
        };
    }

    #[test]
    fn test_input() {
        let r#type = Type::new_timestamp();

        let t = Timestamp::input("161", r#type).unwrap();
        output_eq!(t, r#type, "161");

        Timestamp::input("hello world!", r#type).unwrap_err();
    }
}

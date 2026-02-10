use std::error::Error;
use std::io::Write;

use crate::types::{Tag, Type};
use crate::util::io::Output;

/// Boolean type.
#[derive(Copy, Clone, Debug, Eq, Hash, PartialEq, Ord, PartialOrd)]
pub struct Bool {
    /// The underlying boolean value
    pub value: bool,
}

impl Bool {
    /// Creates a new Bool from a bool value.
    pub fn new(value: bool) -> Self {
        Self { value }
    }

    /// Parses a string into an Integer.
    pub fn input(r#in: &str, r#type: Type) -> Result<Self, Box<dyn Error>> {
        assert_eq!(r#type.r#type(), Tag::Bool);

        Ok(Self {
            value: r#in.parse::<bool>()?,
        })
    }

    /// Writes an Integer to the output writer.
    pub fn output(writer: &mut Output, r#type: Type, out: Self) -> std::io::Result<()> {
        assert_eq!(r#type.r#type(), Tag::Bool);
        write!(writer, "{}", out.value)
    }
}

impl From<Bool> for bool {
    fn from(value: Bool) -> Self {
        value.value
    }
}

impl std::fmt::Display for Bool {
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
            Bool::output(&mut out, $t, $l).unwrap();
            let bytes = buf.into_inner().unwrap();
            let out = String::from_utf8(bytes).unwrap();
            assert_eq!(out, $r.to_string());
        };
    }

    #[test]
    fn test_input() {
        let r#type = Type::new_bool();

        let b = Bool::input("true", r#type).unwrap();
        assert_eq!(b.value, true);
        output_eq!(b, r#type, "true");

        let b = Bool::input("false", r#type).unwrap();
        assert_eq!(b.value, false);
        output_eq!(b, r#type, "false");

        Bool::input("0x10", r#type).unwrap_err();
    }
}

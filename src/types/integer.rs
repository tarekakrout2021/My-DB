use std::error::Error;
use std::io::Write;

use crate::types::numeric::Numeric;
use crate::types::{Tag, Type};
use crate::util::io::Output;

/// 32-bit signed integer type.
#[derive(Copy, Clone, Debug, Eq, Hash, PartialEq, Ord, PartialOrd)]
pub struct Integer {
    /// The underlying 32-bit signed integer value
    pub value: i32,
}

impl Integer {
    /// Creates a new Integer from an i32 value.
    pub fn new(value: i32) -> Self {
        Self { value }
    }

    /// Parses a string into an Integer.
    pub fn input(r#in: &str, r#type: Type) -> Result<Self, Box<dyn Error>> {
        assert_eq!(r#type.r#type(), Tag::Integer);

        // TODO: Empty (or whitespace-only) input is interpreted as a sentinel value -1.
        if r#in.trim().is_empty() {
            return Ok(Self { value: -1 });
        }

        Ok(Self {
            value: r#in.parse::<i32>()?,
        })
    }

    /// Writes an Integer to the output writer.
    pub fn output(writer: &mut Output, r#type: Type, out: Self) -> std::io::Result<()> {
        assert_eq!(r#type.r#type(), Tag::Integer);
        write!(writer, "{}", out.value)
    }
}

impl std::ops::Add for Integer {
    type Output = Self;

    fn add(self, other: Self) -> Self {
        Self {
            value: self.value.wrapping_add(other.value),
        }
    }
}

impl std::ops::AddAssign for Integer {
    fn add_assign(&mut self, rhs: Self) {
        *self = *self + rhs;
    }
}

impl std::ops::Sub for Integer {
    type Output = Self;

    fn sub(self, other: Self) -> Self {
        Self {
            value: self.value.wrapping_sub(other.value),
        }
    }
}

impl std::ops::SubAssign for Integer {
    fn sub_assign(&mut self, rhs: Self) {
        *self = *self - rhs;
    }
}

impl std::ops::Mul for Integer {
    type Output = Self;

    fn mul(self, other: Self) -> Self {
        Self {
            value: self.value.wrapping_mul(other.value),
        }
    }
}

impl std::ops::MulAssign for Integer {
    fn mul_assign(&mut self, rhs: Self) {
        *self = *self * rhs;
    }
}

impl std::ops::Div for Integer {
    type Output = Self;

    fn div(self, other: Self) -> Self {
        Self {
            value: self.value.wrapping_div(other.value),
        }
    }
}

impl std::ops::DivAssign for Integer {
    fn div_assign(&mut self, rhs: Self) {
        *self = *self / rhs;
    }
}

impl Integer {
    /// Converts this Integer to a Numeric with the given type specification.
    pub fn to_numeric(&self, r#type: Type) -> Result<Numeric, Box<dyn Error>> {
        Numeric::input(&self.value.to_string(), r#type)
    }
}

impl From<Integer> for i32 {
    fn from(value: Integer) -> Self {
        value.value
    }
}

impl std::fmt::Display for Integer {
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
            Integer::output(&mut out, $t, $l).unwrap();
            let bytes = buf.into_inner().unwrap();
            let out = String::from_utf8(bytes).unwrap();
            assert_eq!(out, $r.to_string());
        };
    }

    #[test]
    fn test_input() {
        let r#type = Type::new_integer();

        let i = Integer::input("123", r#type).unwrap();
        output_eq!(i, r#type, "123");

        Integer::input("0x10", r#type).unwrap_err();
    }

    #[test]
    fn test_empty_input_returns_minus_one() {
        let r#type = Type::new_integer();
        let i = Integer::input("", r#type).unwrap();
        output_eq!(i, r#type, "-1");
        let j = Integer::input("   ", r#type).unwrap();
        output_eq!(j, r#type, "-1");
    }

    #[test]
    fn test_add() {
        let r#type = Type::new_integer();
        let i1 = Integer::input("10", r#type).unwrap();
        let i2 = Integer::input("17", r#type).unwrap();
        output_eq!(i1 + i2, r#type, "27");
    }

    #[test]
    fn test_sub() {
        let r#type = Type::new_integer();
        let i1 = Integer::input("10", r#type).unwrap();
        let i2 = Integer::input("17", r#type).unwrap();
        output_eq!(i1 - i2, r#type, "-7");
    }

    #[test]
    fn test_mul() {
        let r#type = Type::new_integer();
        let i1 = Integer::input("10", r#type).unwrap();
        let i2 = Integer::input("17", r#type).unwrap();
        output_eq!(i1 * i2, r#type, "170");
    }

    #[test]
    fn test_div() {
        let r#type = Type::new_integer();
        let i1 = Integer::input("99", r#type).unwrap();
        let i2 = Integer::input("11", r#type).unwrap();
        output_eq!(i1 / i2, r#type, "9");
    }
}

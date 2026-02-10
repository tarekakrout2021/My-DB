use std::cmp::Ordering;
use std::error::Error;
use std::io::Write;

use crate::types::{Tag, Type};
use crate::util::io::Output;

/// Maximum precision (total digits) supported by NUMERIC type
pub const MAX_PRECISION_NUMERIC: u32 = 18;
/// Maximum scale (digits after decimal) supported by NUMERIC type
pub const MAX_SCALE_NUMERIC: u16 = 18;
/// Powers of 10 lookup table for efficient scaling operations
const NUMERIC_SHIFTS: [u64; 19] = [
    1,
    10,
    100,
    1000,
    10000,
    100000,
    1000000,
    10000000,
    100000000,
    1000000000,
    10000000000,
    100000000000,
    1000000000000,
    10000000000000,
    100000000000000,
    1000000000000000,
    10000000000000000,
    100000000000000000,
    1000000000000000000,
];

/// Fixed-precision decimal number type with configurable precision and scale.
#[derive(Copy, Clone, Debug, Eq, Hash, PartialEq)]
pub struct Numeric {
    /// Scaled integer representation of the decimal value
    pub value: u64,
}

/// Operations supported by the Numeric type.
pub enum NumericOp {
    Add,
    Sub,
    Mul,
}

impl Numeric {
    /// Creates a Numeric from a raw scaled integer value.
    pub fn new(value: u64) -> Self {
        Self { value }
    }

    /// Parses a string into a Numeric.
    pub fn input(s: &str, r#type: Type) -> Result<Self, Box<dyn Error>> {
        assert_eq!(r#type.r#type(), Tag::Numeric);

        let precision = r#type.precision() as usize;
        let scale = r#type.scale() as usize;

        let mut trimmed = s.trim();

        if trimmed.is_empty() {
            return Err(format!("invalid input syntax for type numeric: \"{}\"", s).into());
        }

        let mut value = 0;

        let first = trimmed.chars().next().unwrap();
        if first == '+' || first == '-' {
            trimmed = &trimmed[1..];
        }
        let neg = first == '-';

        // after fraction
        let mut decimals = 0;
        let mut fraction = false;

        for c in trimmed.chars() {
            if c.is_ascii_digit() {
                value = value * 10 + (c as u64 - b'0' as u64);
                if fraction {
                    decimals += 1;
                }
            } else if c == '.' {
                if fraction {
                    return Err(format!("invalid input syntax for type numeric: \"{}\"", s).into());
                }
                fraction = true;
            } else {
                return Err(format!("invalid input syntax for type numeric: \"{}\"", s).into());
            }
        }

        if decimals <= scale {
            // shift left
            value *= NUMERIC_SHIFTS[scale - decimals];
        } else {
            // shift right and round
            value =
                (value + NUMERIC_SHIFTS[decimals - scale] / 2) / NUMERIC_SHIFTS[decimals - scale];
        }

        if value >= NUMERIC_SHIFTS[precision] {
            let exp = precision as isize - scale as isize;
            return Err(format!("numeric field overflow, a field with precision {}, scale {} must round to an absolute value less than 10^{}", precision, scale, exp).into());
        }

        match neg {
            true => Ok(Self {
                value: value.wrapping_neg(),
            }),
            false => Ok(Self { value }),
        }
    }

    /// Writes a Numeric to the output writer.
    pub fn output(writer: &mut Output, r#type: Type, out: Self) -> std::io::Result<()> {
        assert_eq!(r#type.r#type(), Tag::Numeric);

        let precision = r#type.precision() as usize;
        let scale = r#type.scale() as usize;

        let mut value = out.value as i64;
        let mut result = String::new();

        if value < 0 {
            result.push('-');
            value = -value;
        }

        let s = value.to_string();

        if scale == 0 {
            return write!(writer, "{}", result + &s);
        }

        if scale >= precision || out.value < NUMERIC_SHIFTS[scale] {
            let padded = format!("{:0>width$}", s, width = scale);
            write!(writer, "{}", result + "0." + &padded)
        } else {
            write!(
                writer,
                "{}",
                result + &s[..s.len() - scale] + "." + &s[s.len() - scale..]
            )
        }
    }

    /// Casts this numeric to a different precision, truncating if necessary.
    pub fn cast_precision(&self, _type: Type, new_type: Type) -> Result<Numeric, Box<dyn Error>> {
        // simplified cast
        let value = self.value % NUMERIC_SHIFTS[new_type.precision() as usize];
        Ok(Numeric { value })
    }

    /// Casts this numeric to a different scale, adjusting decimal places.
    pub fn cast_scale(&self, r#type: Type, new_type: Type) -> Result<Numeric, Box<dyn Error>> {
        // simplified cast
        let current_scale = r#type.scale() as usize;
        let new_scale = new_type.scale() as usize;

        let value = match new_scale < current_scale {
            true => self.value / NUMERIC_SHIFTS[current_scale - new_scale],
            false => self.value * NUMERIC_SHIFTS[new_scale - current_scale],
        };

        Ok(Numeric { value })
    }

    /// Computes the result type (precision and scale) for a Numeric operation.
    pub fn result_type(op: NumericOp, r#type: Type) -> Type {
        let (precision, scale) = (r#type.precision(), r#type.scale());

        let (precision, scale) = match op {
            NumericOp::Add => (precision + 1, scale),
            NumericOp::Sub => (precision, scale),
            NumericOp::Mul => (precision + scale as u32, scale + scale),
        };

        // treating this correctly is tricky
        if precision > MAX_PRECISION_NUMERIC || scale > MAX_SCALE_NUMERIC {
            unimplemented!("operation exceeds numeric limits");
        }

        Type::new_numeric(precision, scale)
    }
}

impl std::ops::Add for Numeric {
    type Output = Self;

    fn add(self, rhs: Self) -> Self {
        Self {
            value: self.value.wrapping_add(rhs.value),
        }
    }
}

impl std::ops::AddAssign for Numeric {
    fn add_assign(&mut self, rhs: Self) {
        *self = *self + rhs;
    }
}

impl std::ops::Sub for Numeric {
    type Output = Self;

    fn sub(self, rhs: Self) -> Self {
        Self {
            value: self.value.wrapping_sub(rhs.value),
        }
    }
}

impl std::ops::SubAssign for Numeric {
    fn sub_assign(&mut self, rhs: Self) {
        *self = *self - rhs;
    }
}

impl std::ops::Mul for Numeric {
    type Output = Self;

    fn mul(self, rhs: Self) -> Self {
        Self {
            value: self.value.wrapping_mul(rhs.value),
        }
    }
}

impl std::ops::MulAssign for Numeric {
    fn mul_assign(&mut self, rhs: Self) {
        *self = *self * rhs;
    }
}

impl PartialOrd for Numeric {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Numeric {
    fn cmp(&self, other: &Self) -> Ordering {
        let lhs = self.value as i64;
        let rhs = other.value as i64;
        lhs.cmp(&rhs)
    }
}

impl std::fmt::Display for Numeric {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.value)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::Type;
    use std::io::BufWriter;

    macro_rules! output_eq {
        ($l:expr, $t:expr, $r:expr) => {
            let mut buf = BufWriter::new(Vec::new());
            let mut out = Output::new(&mut buf);
            Numeric::output(&mut out, $t, $l).unwrap();
            let bytes = buf.into_inner().unwrap();
            let out = String::from_utf8(bytes).unwrap();
            assert_eq!(out, $r.to_string());
        };
    }

    #[test]
    fn test_input() {
        let type_5_2 = Type::new_numeric(5, 2);
        let type_3_5 = Type::new_numeric(3, 5);
        let type_1_0 = Type::new_numeric(1, 0);
        let type_2_0 = Type::new_numeric(2, 0);
        let type_4_2 = Type::new_numeric(4, 2);
        let type_7_15 = Type::new_numeric(7, 15);

        Numeric::input("123.45", type_5_2).unwrap();
        Numeric::input("+123.45", type_5_2).unwrap();
        Numeric::input("-123.45", type_5_2).unwrap();
        Numeric::input(".45", type_5_2).unwrap();
        Numeric::input("0.009", type_3_5).unwrap();
        Numeric::input("0.00123456789", type_3_5).unwrap();
        Numeric::input("1.001", type_1_0).unwrap();
        Numeric::input("  -1.001    ", type_1_0).unwrap();

        let n = Numeric::input("+123.45", type_5_2).unwrap();
        assert_eq!(n.value, 12345);
        output_eq!(n, type_5_2, "123.45");

        let n = Numeric::input("-123.45", type_5_2).unwrap();
        assert_eq!(n.value, !(12345 - 1));
        output_eq!(n, type_5_2, "-123.45");

        let n = Numeric::input("0.009", type_3_5).unwrap();
        assert_eq!(n.value, 900);
        output_eq!(n, type_3_5, "0.00900");

        let n = Numeric::input("0.00123456789", type_3_5).unwrap();
        assert_eq!(n.value, 123);
        output_eq!(n, type_3_5, "0.00123");

        let n = Numeric::input("0.00123499999", type_3_5).unwrap();
        assert_eq!(n.value, 123);
        output_eq!(n, type_3_5, "0.00123");

        let n = Numeric::input("0.00123500000", type_3_5).unwrap();
        assert_eq!(n.value, 124);
        output_eq!(n, type_3_5, "0.00124");

        let n = Numeric::input("1.001", type_1_0).unwrap();
        assert_eq!(n.value, 1);
        output_eq!(n, type_1_0, "1");

        let n = Numeric::input("1.5", type_1_0).unwrap();
        assert_eq!(n.value, 2);
        output_eq!(n, type_1_0, "2");

        let n = Numeric::input("1.5", type_2_0).unwrap();
        assert_eq!(n.value, 2);
        output_eq!(n, type_2_0, "2");

        let n = Numeric::input("99", type_2_0).unwrap();
        assert_eq!(n.value, 99);
        output_eq!(n, type_2_0, "99");

        let n = Numeric::input("099.090000", type_4_2).unwrap();
        assert_eq!(n.value, 9909);
        output_eq!(n, type_4_2, "99.09");

        Numeric::input("0.01", type_3_5).unwrap_err();
        Numeric::input("1.01", type_3_5).unwrap_err();
        Numeric::input("1.01.03", type_7_15).unwrap_err();
        Numeric::input("21", type_1_0).unwrap_err();
    }

    #[test]
    fn test_add() {
        let type_1_0 = Type::new_numeric(1, 0);
        let type_2_0 = Type::new_numeric(2, 0);

        let result_type = Numeric::result_type(NumericOp::Add, type_1_0);
        let a = Numeric::input("5", type_1_0).unwrap();
        let b = Numeric::input("4", type_1_0).unwrap();
        let result = a + b;
        output_eq!(result, result_type, "9");

        let result_type = Numeric::result_type(NumericOp::Add, type_2_0);
        let a = Numeric::input("-10", type_2_0).unwrap();
        let b = Numeric::input("15", type_2_0).unwrap();
        let result = a + b;
        output_eq!(result, result_type, "5");
    }

    #[test]
    fn test_sub() {
        let type_1_0 = Type::new_numeric(1, 0);

        let result_type = Numeric::result_type(NumericOp::Sub, type_1_0);
        let a = Numeric::input("5", type_1_0).unwrap();
        let b = Numeric::input("4", type_1_0).unwrap();
        let result = a - b;

        output_eq!(result, result_type, "1");
    }

    #[test]
    fn test_mul() {
        let type_4_2 = Type::new_numeric(4, 2);
        let type_2_0 = Type::new_numeric(2, 0);
        let type_5_2 = Type::new_numeric(5, 2);

        let result_type = Numeric::result_type(NumericOp::Mul, type_4_2);
        let a = Numeric::input("12.34", type_4_2).unwrap();
        let b = Numeric::input("56.78", type_4_2).unwrap();
        let result = a * b;
        output_eq!(result, result_type, "700.6652");

        let result_type = Numeric::result_type(NumericOp::Mul, type_2_0);
        let a = Numeric::input("2", type_2_0).unwrap();
        let b = Numeric::input("5", type_2_0).unwrap();
        let result = a * b;
        output_eq!(result, result_type, "10");

        let result_type = Numeric::result_type(NumericOp::Mul, type_2_0);
        let a = Numeric::input("10", type_2_0).unwrap();
        let b = Numeric::input("-10", type_2_0).unwrap();
        let result = a * b;
        output_eq!(result, result_type, "-100");

        let result_type = Numeric::result_type(NumericOp::Mul, type_5_2);
        let a = Numeric::input("10", type_5_2).unwrap();
        let b = Numeric::input("-10", type_5_2).unwrap();
        let result = a * b;
        output_eq!(result, result_type, "-100.0000");
    }

    #[test]
    fn test_cmp() {
        let type_1_0 = Type::new_numeric(1, 0);

        let a = Numeric::input("5", type_1_0).unwrap();
        let b = Numeric::input("4", type_1_0).unwrap();
        let c = Numeric::input("-1", type_1_0).unwrap();
        let d = Numeric::input("4", type_1_0).unwrap();

        assert!(a > b);
        assert!(c < a);
        assert!(b <= d);

        assert_eq!(b, d);
    }

    #[test]
    fn test_cast_precision() {
        let type_5_3 = Type::new_numeric(5, 3);
        let type_4_3 = Type::new_numeric(4, 3);

        let a = Numeric::input("12.345", type_5_3).unwrap();
        let b = a.cast_precision(type_5_3, type_4_3).unwrap();

        output_eq!(b, type_4_3, "2.345");
    }

    #[test]
    fn test_cast_scale() {
        let type_5_3 = Type::new_numeric(5, 3);
        let type_5_2 = Type::new_numeric(5, 2);

        let a = Numeric::input("12.345", type_5_3).unwrap();
        let b = a.cast_scale(type_5_3, type_5_2).unwrap();

        assert_eq!(b.value, 1234);
        output_eq!(b, type_5_2, "12.34");
    }
}

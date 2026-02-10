//! Database type system for IMLAB.
//!
//! This module provides a comprehensive type system for database values including
//! integers, strings, decimals, and timestamps. All types support parsing from strings
//! and display formatting.

pub mod bool;
pub mod integer;
pub mod numeric;
pub mod text;
pub mod timestamp;

pub mod prelude {
    pub use super::bool::Bool;
    pub use super::integer::Integer;
    pub use super::numeric::Numeric;
    pub use super::text::Text;
    pub use super::timestamp::Timestamp;
}

/// Type tags used to identify different database types in the encoded type system.
#[derive(Debug, PartialEq)]
#[repr(u8)]
pub enum Tag {
    /// Boolean type
    Bool,
    /// Fixed-length character type
    Char,
    /// 32-bit signed integer type
    Integer,
    /// Fixed-precision decimal type with configurable precision and scale
    Numeric,
    /// Variable-length text type
    Text,
    /// Timestamp type for date/time values
    Timestamp,
    /// Variable-length character type
    VarChar,
}

/// Compact representation of database types using bit-packed encoding.
///
/// The type information is encoded in a 64-bit integer:
/// - Bits 63-56: Type tag (8 bits)
/// - Bits 55-24: Precision (32 bits)
/// - Bits 23-8: Scale (16 bits)
/// - Bit 0: Nullable flag (1 bit)
#[derive(Copy, Clone, Eq, PartialEq, Hash)]
pub struct Type {
    code: u64,
}

impl Type {
    /// Creates a new BOOLEAN type.
    pub fn new_bool() -> Self {
        Self::new(Tag::Bool, 0, 0, false)
    }

    /// Creates a new INTEGER type.
    pub fn new_integer() -> Self {
        Self::new(Tag::Integer, 0, 0, false)
    }

    /// Creates a new CHAR type with the specified length.
    ///
    /// # Arguments
    /// * `precision` - The fixed length of the character field
    pub fn new_char(precision: u32) -> Self {
        Self::new(Tag::Char, precision, 0, false)
    }

    /// Creates a new NUMERIC type with the specified precision and scale.
    ///
    /// # Arguments
    /// * `precision` - Total number of digits (1-38)
    /// * `scale` - Number of digits after decimal point (1-38)
    ///
    /// # Panics
    /// Panics if precision or scale are out of valid range.
    pub fn new_numeric(precision: u32, scale: u16) -> Self {
        assert!(
            0 < precision && precision <= numeric::MAX_PRECISION_NUMERIC,
            "NUMERIC precision must be between 1 and {}",
            numeric::MAX_PRECISION_NUMERIC
        );
        assert!(
            scale <= numeric::MAX_SCALE_NUMERIC,
            "NUMERIC scale must be between 0 and {}",
            numeric::MAX_SCALE_NUMERIC
        );
        Self::new(Tag::Numeric, precision, scale, false)
    }

    /// Creates a new TEXT type.
    pub fn new_text() -> Self {
        Self::new(Tag::Text, 0, 0, false)
    }

    /// Creates a new TIMESTAMP type.
    pub fn new_timestamp() -> Self {
        Self::new(Tag::Timestamp, 0, 0, false)
    }

    /// Creates a new VARCHAR type with the specified maximum length.
    ///
    /// # Arguments
    /// * `precision` - Maximum length of the variable character field
    pub fn new_varchar(precision: u32) -> Self {
        Self::new(Tag::VarChar, precision, 0, false)
    }

    /// Returns a nullable version of this type.
    pub fn as_nullable(&self) -> Self {
        Self {
            code: self.code | 1,
        }
    }

    /// Returns the type tag for this type.
    pub fn r#type(&self) -> Tag {
        unsafe { std::mem::transmute((self.code >> 56) as u8) }
    }

    /// Returns the precision (total digits for NUMERIC, length for CHAR/VARCHAR).
    pub fn precision(&self) -> u32 {
        (self.code >> 24) as u32
    }

    /// Returns the scale (digits after decimal point) for NUMERIC types.
    pub fn scale(&self) -> u16 {
        (self.code >> 8) as u16
    }

    /// Returns true if this type allows NULL values.
    pub fn is_nullable(&self) -> bool {
        self.code & 1 == 1
    }

    /// Returns a string representation suitable for code generation.
    ///
    /// # Examples
    /// - INTEGER -> "Integer"
    /// - CHAR(10) -> "Char"
    /// - NUMERIC(12,2) -> "Numeric"
    pub fn fmt_codegen(&self) -> &'static str {
        match self.r#type() {
            Tag::Bool => "Bool",
            Tag::Char => "Char",
            Tag::Integer => "Integer",
            Tag::Numeric => "Numeric",
            Tag::Text => "Text",
            Tag::Timestamp => "Timestamp",
            Tag::VarChar => "VarChar",
        }
    }

    /// Internal constructor for creating types with all parameters.
    fn new(tag: Tag, precision: u32, scale: u16, nullable: bool) -> Self {
        Self {
            code: ((tag as u64) << 56)
                + ((precision as u64) << 24)
                + ((scale as u64) << 8)
                + nullable as u64,
        }
    }
}

impl std::fmt::Debug for Type {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut dbg_struct = f.debug_struct("Type");

        // type
        dbg_struct.field("type", &self.r#type());

        // precision
        match self.r#type() {
            Tag::Char | Tag::Numeric | Tag::VarChar => {
                dbg_struct.field("precision", &self.precision());
            }
            _ => {}
        };

        // scale
        if self.r#type() == Tag::Numeric {
            dbg_struct.field("scale", &self.scale());
        };

        // nullable
        dbg_struct.field("nullable", &self.is_nullable());

        dbg_struct.finish()
    }
}

impl From<u64> for Type {
    fn from(code: u64) -> Self {
        Self { code }
    }
}

impl From<&u64> for Type {
    fn from(code: &u64) -> Self {
        Self { code: *code }
    }
}

impl From<Type> for u64 {
    fn from(r#type: Type) -> Self {
        r#type.code
    }
}

impl From<&Type> for u64 {
    fn from(r#type: &Type) -> Self {
        r#type.code
    }
}

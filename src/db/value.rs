use crate::types::bool::Bool;
use crate::types::integer::Integer;
use crate::types::numeric::Numeric;
use crate::types::text::Text;
use crate::types::timestamp::Timestamp;

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub enum Value {
    Bool(Bool),
    Integer(Integer),
    Numeric(Numeric),
    Text(Text),
    Timestamp(Timestamp),
}

impl From<i32> for Value {
    fn from(v: i32) -> Self {
        Value::Integer(Integer::new(v))
    }
}

impl From<bool> for Value {
    fn from(v: bool) -> Self {
        Value::Bool(Bool::new(v))
    }
}

impl From<u64> for Value {
    fn from(v: u64) -> Self {
        Value::Numeric(Numeric::new(v))
    }
}

impl From<Bool> for Value {
    fn from(v: Bool) -> Self {
        Value::Bool(v)
    }
}

impl From<Integer> for Value {
    fn from(v: Integer) -> Self {
        Value::Integer(v)
    }
}

impl From<Numeric> for Value {
    fn from(v: Numeric) -> Self {
        Value::Numeric(v)
    }
}

impl From<Text> for Value {
    fn from(v: Text) -> Self {
        Value::Text(v)
    }
}

impl From<Timestamp> for Value {
    fn from(v: Timestamp) -> Self {
        Value::Timestamp(v)
    }
}

use std::fmt;

impl fmt::Display for Value {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Value::Bool(b) => write!(f, "{}", b),
            Value::Integer(i) => write!(f, "{}", i),
            Value::Numeric(n) => write!(f, "{}", n),
            Value::Text(t) => write!(f, "{}", t),
            Value::Timestamp(ts) => write!(f, "{}", ts),
        }
    }
}
